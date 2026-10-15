import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';
import { runReferenceChecks } from './reference-browser.mjs';

const reviewMode=process.argv.includes('--visual-review'), baseline=process.argv.includes('--baseline'),assemblyMode=process.argv.includes('--assembly');
const repairEvidence=process.argv.includes('--fit-repair');
const neckEvidence=process.argv.includes('--fit-neck');
const topologyMode=process.argv.includes('--topology');
const referenceMode=process.argv.includes('--reference');
const out = resolve(referenceMode?'artifacts/original-ornith-proof-20261010/browser':topologyMode?'artifacts/character-topology-1-20261009/browser':neckEvidence?'artifacts/assembly-fit-3-20261009/'+(assemblyMode?'browser':'functional'):repairEvidence?'artifacts/assembly-fit-2-20261009/'+(assemblyMode?'browser':'functional'):assemblyMode?'artifacts/assembly-fit-20261009/browser':process.argv.includes('--assembly-regression')?'artifacts/assembly-fit-20261009/functional':'artifacts/visual-qa-20261009/'+(reviewMode?(baseline?'baseline':'repaired'):'functional'));
await mkdir(out, { recursive: true });
const profile = await mkdtemp(join(tmpdir(), 'luxel-browser-'));
const downloads = join(profile, 'downloads'); await mkdir(downloads);
console.log('Starting isolated software Firefox');
const preferences = {
  'gfx.webrender.software': true, 'layers.acceleration.disabled': true,
  'webgl.disabled': true, 'media.hardware-video-decoding.enabled': false,
  'browser.shell.checkDefaultBrowser': false, 'browser.startup.homepage_override.mstone': 'ignore',
  'browser.download.folderList': 2, 'browser.download.dir': downloads, 'browser.download.useDownloadDir': true,
  'browser.helperApps.neverAsk.saveToDisk': 'application/json', 'browser.download.always_ask_before_handling_new_types': false,
};
await writeFile(join(profile, 'user.js'), Object.entries(preferences).map(([k, v]) => `user_pref(${JSON.stringify(k)}, ${JSON.stringify(v)});`).join('\n'));
const browser = spawn('firefox', ['--headless', '--no-remote', '--new-instance', '--profile', profile, '--remote-debugging-port', '0'], {
  detached: true, env: { ...process.env, MOZ_HEADLESS: '1', LIBGL_ALWAYS_SOFTWARE: '1', GALLIUM_DRIVER: 'llvmpipe' },
});
let log = '', ws, id = 0, activeContext;
const pending = new Map(), events = [], eventWaiters = [], evidence = { preferences, checks: [], passed: false };
const record = (name, result) => evidence.checks.push({ name, result });
const startup = new Promise((resolveReady, reject) => {
  const timer = setTimeout(() => reject(new Error('Firefox remote agent did not become ready')), 20000);
  const receive = chunk => {
    log += chunk.toString(); const match = log.match(/WebDriver BiDi listening on (ws:\/\/\S+)/);
    if (match) { clearTimeout(timer); resolveReady(match[1]); }
  };
  browser.stdout.on('data', receive); browser.stderr.on('data', receive);
  browser.once('error', error => { clearTimeout(timer); reject(error); });
  browser.once('exit', (code, signal) => { clearTimeout(timer); reject(new Error(`Firefox exited before readiness: ${code}, ${signal}`)); });
});
function command(method, params = {}) {
  return new Promise((resolveCommand, reject) => {
    const request = ++id, timer = setTimeout(() => { pending.delete(request); reject(new Error(`Command timed out: ${method} ${params.expression?.slice(0, 120) ?? ''}`)); }, reviewMode?60000:15000);
    pending.set(request, { resolve: value => { clearTimeout(timer); resolveCommand(value); }, reject: error => { clearTimeout(timer); reject(error); } });
    ws.send(JSON.stringify({ id: request, method, params }));
  });
}
try {
  const endpoint = await startup; ws = new WebSocket(endpoint + '/session');
  await new Promise((resolveOpen, reject) => { ws.addEventListener('open', resolveOpen, { once: true }); ws.addEventListener('error', reject, { once: true }); });
  ws.addEventListener('message', event => {
    const message = JSON.parse(event.data), call = pending.get(message.id);
    if (!call) {
      events.push(message);
      for (const waiter of eventWaiters) if (waiter.method === message.method) waiter.resolve(message);
      return;
    }
    pending.delete(message.id);
    if (message.type === 'error') call.reject(new Error(JSON.stringify(message))); else call.resolve(message.result);
  });
  const session = await command('session.new', { capabilities: {} }); evidence.browser = session.capabilities;
  await command('session.subscribe', { events: ['log.entryAdded', 'browsingContext.downloadEnd'] });
  const { context } = await command('browsingContext.create', { type: 'tab' });
  activeContext = context;
  await command('browsingContext.setViewport', { context, viewport: { width: 1100, height: 950 }, devicePixelRatio: 1 });
  await command('browsingContext.navigate', { context, url: pathToFileURL(resolve(referenceMode?'character_lab/reference.html':topologyMode?'character_lab/topology.html':assemblyMode?'character_lab/assembly.html':reviewMode?'character_lab/review.html':'character_lab/index.html')).href, wait: 'complete' });
  const evaluate = async expression => {
    const response = await command('script.evaluate', { expression, target: { context }, awaitPromise: true, userActivation: true });
    if (response.type === 'exception') throw new Error(JSON.stringify(response));
    return response.result.value;
  };
  if(referenceMode){
    await runReferenceChecks({evaluate,command,context,record,events,eventWaiters,downloads,out});evidence.passed=true;
  }else if(topologyMode){
    await evaluate(`new Promise((resolve,reject)=>{const r=document.querySelector('#luxel-topology');const check=()=>{if(r.dataset.ready){resolve(true);return true;}const error=r.querySelector('[data-topology="error"]').textContent;if(error){reject(new Error(error));return true;}return false;};if(check())return;const o=new MutationObserver(()=>{if(check())o.disconnect();});o.observe(r,{attributes:true,subtree:true,childList:true});})`);
    const topologyChange=(name,value,click=false)=>evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-topology'),old=r.dataset.revision,o=new MutationObserver(()=>{if(r.dataset.revision!==old){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});const e=document.querySelector(${JSON.stringify(click?'[data-topology-action="'+name+'"]':'[data-topology="'+name+'"]')});if(${click})e.click();else{if(e.type==='checkbox')e.checked=${Boolean(value)};else e.value=${JSON.stringify(String(value))};e.dispatchEvent(new Event('change'));}})`);
    const topologyCapture=async name=>{
      const png=await evaluate(`(async()=>{const s=document.querySelector('[data-topology-stage="partition"]').cloneNode(true);s.querySelector('[data-overlay]')?.remove();s.setAttribute('xmlns','http://www.w3.org/2000/svg');s.setAttribute('width',600);s.setAttribute('height',700);const i=new Image();i.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(s));await i.decode();const c=document.createElement('canvas');c.width=600;c.height=700;c.getContext('2d').drawImage(i,0,0);return c.toDataURL('image/png');})()`);await writeFile(join(out,name+'.png'),Buffer.from(png.split(',')[1],'base64'));
    };
    const relations=JSON.parse(await evaluate(`(async()=>{const render=async(which,onlyFace=false)=>{const s=document.querySelector('[data-topology-stage="'+which+'"]').cloneNode(true);s.querySelector('[data-overlay]')?.remove();if(onlyFace)for(const p of s.querySelectorAll('[data-part]'))if(p.dataset.part!=='face')p.remove();s.setAttribute('xmlns','http://www.w3.org/2000/svg');s.setAttribute('width',600);s.setAttribute('height',700);const i=new Image();i.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(s));await i.decode();const c=document.createElement('canvas');c.width=600;c.height=700;const x=c.getContext('2d');x.drawImage(i,0,0);return [[300,430],[300,444]].map(p=>[...x.getImageData(...p,1,1).data]);};return JSON.stringify({whole_hood:await render('current'),partition:await render('partition'),face:await render('partition',true)});})()`));
    await writeFile(join(out,'occlusion-relations.json'),JSON.stringify(relations,null,2));const brightness=p=>(p[0]+p[1]+p[2])/3;
    assert.ok(brightness(relations.face[0])>200&&brightness(relations.face[1])>200);assert.ok(brightness(relations.partition[0])>200);assert.ok(brightness(relations.partition[1])<100);assert.ok(brightness(relations.whole_hood[0])<100);record('rear_behind_face_front_lip_over_lower_face_and_whole_hood_rejected',true);
    const fullPage=await command('browsingContext.captureScreenshot',{context,origin:'document'});await writeFile(join(out,'opening-editor.png'),Buffer.from(fullPage.data,'base64'));await topologyCapture('neutral');
    const maskPath=()=>evaluate(`document.querySelector('[data-topology-stage="partition"] clipPath[id$="hoodie_front_mask"] path').getAttribute('d')`),neutralMask=await maskPath();
    for(const pose of ['left','right','blink','speaking']){await topologyChange('pose',pose);if(['left','right'].includes(pose))assert.notEqual(await maskPath(),neutralMask);await topologyCapture(pose);}record('posed_native_masks_and_captures',true);await topologyChange('pose','neutral');
    const point=JSON.parse(await evaluate(`JSON.stringify((()=>{const p=document.querySelector('[data-opening-point="3"]').getBoundingClientRect();return {x:Math.round(p.x+p.width/2),y:Math.round(p.y+p.height/2),revision:document.querySelector('#luxel-topology').dataset.revision};})())`));
    await command('input.performActions',{context,actions:[{type:'pointer',id:'opening-mouse',parameters:{pointerType:'mouse'},actions:[{type:'pointerMove',x:point.x,y:point.y},{type:'pointerDown',button:0},{type:'pointerMove',x:point.x+8,y:point.y+3},{type:'pointerUp',button:0}]}]});
    await evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-topology');if(r.dataset.revision!==${JSON.stringify(point.revision)}){resolve(true);return;}const o=new MutationObserver(()=>{if(r.dataset.revision!==${JSON.stringify(point.revision)}){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});})`);
    assert.equal(await evaluate(`document.querySelector('[data-topology="error"]').textContent`),'');assert.notEqual(await maskPath(),neutralMask);await topologyChange('undo',null,true);assert.equal(await maskPath(),neutralMask);record('actual_opening_pointer_edit_and_undo',true);
    await topologyChange('y',443);const changedMask=await maskPath();assert.notEqual(changedMask,neutralMask);await topologyChange('y',-1);assert.equal(await maskPath(),changedMask);assert.ok(await evaluate(`document.querySelector('[data-topology="error"]').textContent.length>0`));record('invalid_opening_preserves_accepted_geometry',true);
    const topologyDownload=async action=>{const done=new Promise((resolveDownload,reject)=>{const timer=setTimeout(()=>reject(new Error('Opening download did not finish')),15000);eventWaiters.push({method:'browsingContext.downloadEnd',resolve:r=>{clearTimeout(timer);resolveDownload(r);}});});await evaluate(`document.querySelector('[data-topology-action="${action}"]').click()`);assert.equal((await done).params.status,'complete');};
    await topologyDownload('recipe');const recipe=await readFile(join(downloads,'ornith-opening-recipe.json'),'utf8');assert.equal(JSON.parse(recipe).front_polygon[3][1],443);await topologyChange('reset',null,true);assert.equal(await maskPath(),neutralMask);
    const importOpening=async text=>evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-topology'),old=r.dataset.revision,o=new MutationObserver(()=>{if(r.dataset.revision!==old){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});const input=document.querySelector('[data-topology="import"]'),d=new DataTransfer();d.items.add(new File([${JSON.stringify(text)}],'opening.json',{type:'application/json'}));input.files=d.files;input.dispatchEvent(new Event('change'));})`);
    await importOpening('{"schema":"wrong"}');assert.equal(await maskPath(),neutralMask);await importOpening(recipe);assert.equal(await maskPath(),changedMask);await topologyDownload('source');const native=await readFile(join(downloads,'ornith-opening-native.json'),'utf8');await writeFile(join(out,'ornith-opening-native.json'),native);await writeFile(join(out,'ornith-opening-recipe.json'),recipe);record('real_recipe_export_import_and_native_download',true);
    await topologyChange('reset',null,true);for(const region of ['rear','front']){await topologyChange('region',region);await topologyCapture(region+'-only');}await topologyChange('region','composition');
    await command('browsingContext.setViewport',{context,viewport:{width:360,height:900}});assert.equal(await evaluate('document.documentElement.scrollWidth<=innerWidth'),true);record('opening_editor_narrow_layout',true);
    await command('browsingContext.navigate',{context,url:pathToFileURL(resolve('character_lab/index.html')).href,wait:'complete'});
    await evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-workbench');if(r.dataset.project){resolve(true);return;}const o=new MutationObserver(()=>{if(r.dataset.project){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});})`);
    await evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-workbench'),o=new MutationObserver(()=>{if(r.dataset.project==='ornith_chibi'){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});const input=document.querySelector('[data-input="import"]'),d=new DataTransfer();d.items.add(new File([${JSON.stringify(native)}],'opening-native.json',{type:'application/json'}));input.files=d.files;input.dispatchEvent(new Event('change'));})`);
    assert.equal(await evaluate(`document.querySelectorAll('[data-part="hoodie_front"]').length`),1);assert.ok(await evaluate(`document.querySelector('[data-part="hoodie_front"]').getAttribute('clip-path').includes('hoodie_front_mask')`));record('partitioned_character_imports_into_actual_workbench',true);
    assert.equal(events.filter(e=>e.method==='log.entryAdded'&&e.params.level==='error').length,0);record('no_browser_errors',true);evidence.passed=true;
  }else if(assemblyMode){
    await evaluate(`new Promise((resolve,reject)=>{const r=document.querySelector('#luxel-assembly');const check=()=>{if(r.dataset.ready){resolve(true);return true;}const error=r.querySelector('[data-fit="error"]').textContent;if(error){reject(new Error(error));return true;}return false;};if(check())return;const observer=new MutationObserver(()=>{if(check())observer.disconnect();});observer.observe(r,{attributes:true,subtree:true,childList:true});})`);
    const change=(selector,value,event='change')=>evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-assembly'),old=r.dataset.revision;const o=new MutationObserver(()=>{if(r.dataset.revision!==old){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});const e=document.querySelector(${JSON.stringify(selector)});if(${JSON.stringify(event)}==='click')e.click();else{if(e.type==='checkbox')e.checked=${JSON.stringify(Boolean(value))};else e.value=${JSON.stringify(String(value))};e.dispatchEvent(new Event('change',{bubbles:true}));}})`);
    const report=async()=>JSON.parse(await evaluate(`document.querySelector('[data-fit="report"]').textContent`));
    const capture=async(name,which='fitted')=>{
      const png=await evaluate(`(async()=>{const s=document.querySelector('[data-stage="${which}"]').cloneNode(true);s.setAttribute('xmlns','http://www.w3.org/2000/svg');const [w,h]=s.getAttribute('viewBox').split(' ').slice(2).map(Number);s.setAttribute('width',w);s.setAttribute('height',h);const i=new Image();i.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(s));await i.decode();const c=document.createElement('canvas');c.width=w;c.height=h;c.getContext('2d').drawImage(i,0,0);return c.toDataURL('image/png');})()`);
      await writeFile(join(out,name+'.png'),Buffer.from(png.split(',')[1],'base64'));
    };
    const waitForDownload=()=>new Promise((resolveDownload,reject)=>{const timer=setTimeout(()=>reject(new Error('Assembly export did not complete')),15000);eventWaiters.push({method:'browsingContext.downloadEnd',resolve:r=>{clearTimeout(timer);resolveDownload(r);}});});
    const exportClick=async action=>{const done=waitForDownload();await evaluate(`document.querySelector('[data-fit-action="${action}"]').click()`);assert.equal((await done).params.status,'complete');};
    const neckOverlap=async label=>{
      const result=JSON.parse(await evaluate(`(async()=>{const original=document.querySelector('[data-stage="fitted"]');const alpha=async(part,lift=0)=>{const s=original.cloneNode(true);s.querySelector('[data-fit-overlay]')?.remove();s.setAttribute('xmlns','http://www.w3.org/2000/svg');s.setAttribute('width',600);s.setAttribute('height',700);for(const p of s.querySelectorAll('[data-part]'))if(p.dataset.part!==part)p.remove();const face=s.querySelector('[data-part="face"]');if(face&&lift)face.setAttribute('transform','translate(0,'+lift+')');const i=new Image();i.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(s));await i.decode();const c=document.createElement('canvas');c.width=600;c.height=700;const x=c.getContext('2d');x.drawImage(i,0,0);return x.getImageData(0,0,600,700).data;};const [face,hood,raised]=await Promise.all([alpha('face'),alpha('hoodie'),alpha('face',-20)]);const overlap=a=>{let area=0,width=0;for(let x=220;x<=380;x++){let count=0;for(let y=395;y<=470;y++){const i=4*(y*600+x)+3;if(a[i]>=128&&hood[i]>=128){area++;count++;}}if(count>=8)width++;}return {area,width};};return JSON.stringify({fitted:overlap(face),raised:overlap(raised)});})()`));
      await writeFile(join(out,'ornith-neck-'+label+'.json'),JSON.stringify(result,null,2));
      assert.ok(result.fitted.width>=150,'Hood must overlap at least 150 of the central 161 columns by eight pixels');
      assert.ok(result.raised.width<150,'The perched-head negative control must fail the same contact-width gate');
      record('ornith_visible_hood_overlap_'+label,true);return result;
    };
    const neckConnectivity=async label=>{
      const rays=JSON.parse(await evaluate(`(async()=>{const s=document.querySelector('[data-stage="fitted"]').cloneNode(true);s.querySelector('[data-fit-overlay]')?.remove();s.setAttribute('xmlns','http://www.w3.org/2000/svg');s.setAttribute('width',600);s.setAttribute('height',800);for(const p of s.querySelectorAll('[data-part]'))if(!['face','torso','neck_bridge'].includes(p.dataset.part))p.remove();const i=new Image();i.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(s));await i.decode();const c=document.createElement('canvas');c.width=600;c.height=800;const x=c.getContext('2d');x.drawImage(i,0,0);const pixels=x.getImageData(0,0,600,800).data;const measure=data=>[294,300,306].map(a=>{let run=0,gap=0;for(let b=330;b<=390;b++){run=data[4*(b*600+a)+3]<128?run+1:0;gap=Math.max(gap,run);}return {x:a,max_gap:gap};});const rays=measure(pixels);for(const p of s.querySelectorAll('[data-part="face"],[data-part="neck_bridge"]'))p.remove();const missing=new Image();missing.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(s));await missing.decode();x.clearRect(0,0,600,800);x.drawImage(missing,0,0);return JSON.stringify({rays,missing_skin:measure(x.getImageData(0,0,600,800).data)});})()`));
      await writeFile(join(out,'cyan-neck-'+label+'.json'),JSON.stringify(rays,null,2));assert.ok(rays.rays.every(r=>r.max_gap<=1));assert.ok(rays.missing_skin.every(r=>r.max_gap>1));record('cyan_three_neck_rays_'+label,true);
    };
    for(const subject of ['cyan_vtuber','ornith_chibi']){
      await change('[data-fit="subject"]',subject);
      let measured=await report();assert.equal(measured.original.geometry_passed,false);assert.equal(measured.fitted.geometry_passed,true);assert.equal(measured.fitted.visual_acceptance,false);
      assert.equal(await evaluate(`(()=>{const ids=[...document.querySelectorAll('[id]')].map(e=>e.id);return new Set(ids).size===ids.length;})()`),true);record(subject+'_independent_document_masks',true);
      if(subject==='ornith_chibi'){
        // Atlas interiors have alpha 250–254; bound transmitted color by 255-alpha plus two rounding levels.
        const occlusion=JSON.parse(await evaluate(`(async()=>{const s=document.querySelector('[data-stage="fitted"]').cloneNode(true);s.querySelector('[data-fit-overlay]')?.remove();s.setAttribute('xmlns','http://www.w3.org/2000/svg');s.setAttribute('width',600);s.setAttribute('height',800);const pixel=async()=>{const i=new Image();i.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(s));await i.decode();const c=document.createElement('canvas');c.width=600;c.height=800;const x=c.getContext('2d');x.drawImage(i,0,0);return x.getImageData(0,0,600,800).data;};const shown=await pixel();for(const p of s.querySelectorAll('[data-part="paw_left"],[data-part="paw_right"]'))p.setAttribute('display','none');const hidden=await pixel();for(const p of s.querySelectorAll('[data-part]'))if(p.dataset.part!=='laptop')p.setAttribute('display','none');const lid=await pixel();let covered=0,changed=0,maxdelta=0;const samples=[];for(let i=0;i<lid.length;i+=4)if(lid[i+3]>=250){covered++;let d=0;for(let j=0;j<4;j++)d=Math.max(d,Math.abs(shown[i+j]-hidden[i+j]));if(d>255-lid[i+3]+2){changed++;maxdelta=Math.max(maxdelta,d);if(samples.length<10)samples.push([i/4%600,Math.floor(i/4/600),d]);}}return JSON.stringify({covered,changed,maxdelta,samples});})()`));await writeFile(join(out,'occlusion-pixels.json'),JSON.stringify(occlusion,null,2));assert.ok(occlusion.covered>1000);assert.equal(occlusion.changed,0);record('opaque_laptop_hides_overlapping_paw',true);
      }
      await writeFile(join(out,subject+'-report.json'),JSON.stringify(measured,null,2));
      const overlay=await command('browsingContext.captureScreenshot',{context,origin:'document'});await writeFile(join(out,subject+'-overlay.png'),Buffer.from(overlay.data,'base64'));
      await change('[data-fit="landmarks"]',false);await capture(subject+'-original','original');await capture(subject+'-fitted');
      if(neckEvidence&&subject==='ornith_chibi')await neckOverlap('neutral');
      if(neckEvidence&&subject==='cyan_vtuber')await neckConnectivity('neutral');
      if(subject==='ornith_chibi'){await change('[data-fit="eyes"]',true);await capture(subject+'-bare-eyes');}
      for(const pose of ['blink','midpoint','speaking','left','right']){await change('[data-fit="pose"]',pose);assert.equal((await report()).fitted.geometry_passed,true);await capture(subject+'-'+pose);if(neckEvidence&&['left','right'].includes(pose)){if(subject==='ornith_chibi')await neckOverlap(pose);else await neckConnectivity(pose);}}
      await change('[data-fit="pose"]','neutral');
      if(subject==='ornith_chibi')await change('[data-fit="eyes"]',false);
      await evaluate(`document.querySelector('[data-fit="group"]').value=${JSON.stringify(subject==='cyan_vtuber'?'eyes':'head')};document.querySelector('[data-fit="group"]').dispatchEvent(new Event('change'))`);
      const before=await evaluate(`document.querySelector('[data-stage="fitted"]').outerHTML`);
      await change('[data-fit="y"]',6);assert.notEqual(await evaluate(`document.querySelector('[data-stage="fitted"]').outerHTML`),before);
      await evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-assembly'),old=r.dataset.revision,o=new MutationObserver(()=>{if(r.dataset.revision!==old){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});for(const [name,value]of [['x',4],['y',6]]){const e=document.querySelector('[data-fit="'+name+'"]');e.value=value;e.dispatchEvent(new Event('change'));}})`);
      const valid=await evaluate(`document.querySelector('[data-stage="fitted"]').outerHTML`);await change('[data-fit="scale"]',0);assert.equal(await evaluate(`document.querySelector('[data-stage="fitted"]').outerHTML`),valid);assert.ok(await evaluate(`document.querySelector('[data-fit="error"]').textContent.length>0`));
      await exportClick('guide');const guideText=await readFile(join(downloads,subject+'-guide.json'),'utf8');assert.deepEqual(JSON.parse(guideText).groups[0].offset,[4,6]);
      await change('[data-fit-action="reset"]',null,'click');assert.equal((await report()).fitted.geometry_passed,true);
      await exportClick('source');const saved=JSON.parse(await readFile(join(downloads,subject+'-fitted.json'),'utf8'));assert.equal(saved.id,subject);await writeFile(join(out,subject+'-fitted.json'),JSON.stringify(saved,null,2));
      const importGuide=async text=>evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-assembly'),old=r.dataset.revision,o=new MutationObserver(()=>{if(r.dataset.revision!==old){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});const input=document.querySelector('[data-fit="import"]'),d=new DataTransfer();d.items.add(new File([${JSON.stringify(text)}],'guide.json',{type:'application/json'}));input.files=d.files;input.dispatchEvent(new Event('change'));})`);
      await importGuide('{"schema":"wrong"}');assert.equal((await report()).fitted.geometry_passed,true);assert.ok(await evaluate(`document.querySelector('[data-fit="error"]').textContent.length>0`));
      await importGuide(guideText);assert.equal(await evaluate(`document.querySelector('[data-fit="y"]').value`),'6');await change('[data-fit-action="reset"]',null,'click');
      record(subject+'_real_fit_pose_edit_rejection_export_import',true);
    }
    await command('browsingContext.setViewport',{context,viewport:{width:360,height:900}});assert.equal(await evaluate('document.documentElement.scrollWidth<=innerWidth'),true);record('assembly_narrow_layout',true);
    const saved=await readFile(join(out,'ornith_chibi-fitted.json'),'utf8');
    await command('browsingContext.navigate',{context,url:pathToFileURL(resolve('character_lab/index.html')).href,wait:'complete'});
    await evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-workbench');if(r.dataset.project){resolve(true);return;}const o=new MutationObserver(()=>{if(r.dataset.project){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});})`);
    await evaluate(`new Promise(resolve=>{const r=document.querySelector('#luxel-workbench'),o=new MutationObserver(()=>{if(r.dataset.project==='ornith_chibi'){o.disconnect();resolve(true);}});o.observe(r,{attributes:true});const input=document.querySelector('[data-input="import"]'),d=new DataTransfer();d.items.add(new File([${JSON.stringify(saved)}],'fitted.json',{type:'application/json'}));input.files=d.files;input.dispatchEvent(new Event('change'));})`);
    await evaluate(`const p=document.querySelector('[data-input="part"]');p.value='paw_left';p.dispatchEvent(new Event('change'));`);
    assert.equal(await evaluate(`document.querySelector('[data-input="layerY"]').value`),'551');record('fitted_source_imports_into_actual_workbench',true);
    assert.equal(events.filter(e=>e.method==='log.entryAdded'&&e.params.level==='error').length,0);record('no_browser_errors',true);evidence.passed=true;
  }else if(reviewMode){
    const result=JSON.parse(await evaluate('(async()=>JSON.stringify(await window.luxelVisualReview))()'));
    await writeFile(join(out,'report.json'),JSON.stringify(result.report,null,2));
    for(const item of result.images) await writeFile(join(out,item.subject+'-'+item.label+'.png'),Buffer.from(item.png.split(',')[1],'base64'));
    const html=await evaluate(`(()=>{const copy=document.documentElement.cloneNode(true);for(const s of copy.querySelectorAll('script'))s.remove();return '<!doctype html>'+copy.outerHTML})()`);
    await writeFile(join(out,'review.html'),html);
    assert.equal(result.report.detached_control_rejected,true);
    record('detached_negative_control_rejected',true);
    record('rendered_join_passed',result.report.join_passed);
    record('ornith_parts_and_gaze_passed',result.report.ornith_parts_and_gaze_passed);
    if(!baseline){
      assert.equal(result.report.join_passed,true);assert.equal(result.report.ornith_parts_and_gaze_passed,true);
      assert.equal(result.report.opposite_gaze_control_rejected,true);assert.equal(result.report.missing_parts_control_rejected,true);
      record('opposite_gaze_and_missing_parts_controls_rejected',true);
    }
    assert.equal(events.filter(e=>e.method==='log.entryAdded'&&e.params.level==='error').length,0);
    record('captured_actual_composition',result.images.length);evidence.passed=true;
  }else{
  const set = (selector, value, event = 'change') => evaluate(`(() => { const e=document.querySelector(${JSON.stringify(selector)}); e.value=${JSON.stringify(String(value))}; e.dispatchEvent(new Event(${JSON.stringify(event)},{bubbles:true})); return true; })()`);
  const click = selector => evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
  const path = part => evaluate(`document.querySelector('[data-artwork] [data-part="${part}"]').getAttribute('d')`);
  await evaluate(`new Promise((resolve,reject)=>{const root=document.querySelector('#luxel-workbench'); if(root.dataset.project){resolve(true);return;} const observer=new MutationObserver(()=>{if(root.dataset.project){observer.disconnect();resolve(true);}else if(root.querySelector('[data-output="error"]').textContent){observer.disconnect();reject(new Error(root.querySelector('[data-output="error"]').textContent));}}); observer.observe(root,{attributes:true,childList:true,subtree:true});})`);
  assert.equal(await evaluate(`document.querySelector('#luxel-workbench').dataset.project`), 'cyan_vtuber');
  await set('[data-input="subject"]', 'cyan');
  assert.equal(await evaluate(`getComputedStyle(document.querySelector('[data-view="placement"]')).display`),'none');
  const neutral = await path('face'); await set('[data-parameter="head"]', 0.7, 'input'); assert.notEqual(await path('face'), neutral); record('pose_changes_visible_geometry', true);
  await click('[data-action="neutral"]'); assert.equal(await path('face'), neutral);
  await click('[data-input="rig"]'); await set('[data-input="part"]', 'hair_left');
  await set('[data-input="pivotX"]', 201); await set('[data-input="gain"]', 7.5);
  assert.equal(await evaluate(`document.querySelector('[data-input="channel"]').selectedOptions[0].textContent`), 'Layer: sway · rotate');
  await set('[data-input="channel"]', 'node:0'); await set('[data-input="gain"]', 9.5);
  assert.equal(await evaluate(`document.querySelector('[data-input="channel"]').selectedOptions[0].textContent`), 'Rig: head · rotate');
  await set('[data-input="channel"]', 'part:0'); assert.equal(await evaluate(`document.querySelector('[data-input="gain"]').value`), '7.5');
  const hairBefore = await path('hair_left');
  const handle = JSON.parse(await evaluate(`JSON.stringify((()=>{const r=document.querySelector('.control-handle').getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2)}})())`));
  await command('input.performActions', { context, actions: [{ type: 'pointer', id: 'editor-mouse', parameters: { pointerType: 'mouse' },
    actions: [{ type: 'pointerMove', x: handle.x, y: handle.y }, { type: 'pointerDown', button: 0 },
      { type: 'pointerMove', x: handle.x+15, y: handle.y+10 }, { type: 'pointerUp', button: 0 }] }] });
  assert.notEqual(await path('hair_left'), hairBefore); record('actual_pointer_drag_edits_source', true);
  await click('[data-action="undo"]'); assert.equal(await path('hair_left'), hairBefore); record('undo_restores_source_geometry', true);
  const waitDownload = () => new Promise((resolveDownload, reject) => {
    const timer = setTimeout(() => reject(new Error('Export download did not complete')), 15000);
    eventWaiters.push({ method: 'browsingContext.downloadEnd', resolve: value => { clearTimeout(timer); resolveDownload(value); } });
  });
  const downloadReady = waitDownload();
  await click('[data-action="export"]');
  const download = await downloadReady; assert.equal(download.params.status, 'complete');
  await command('browsingContext.activate', { context });
  assert.equal(await evaluate('document.visibilityState'), 'visible');
  await evaluate(`new Promise(resolve => { const a=document.querySelector('[data-output="time"]'); const old=a.textContent; const observer=new MutationObserver(()=>{if(a.textContent!==old){observer.disconnect();resolve(true);}}); observer.observe(a,{childList:true,subtree:true,characterData:true}); document.querySelector('[data-action="play"]').click(); })`);
  await click('[data-action="play"]'); assert.equal(await evaluate(`document.querySelector('[data-action="play"]').getAttribute('aria-pressed')`), 'false'); record('elapsed_time_play_pause', true);
  await set('[data-input="time"]', 1.15, 'input'); assert.equal(await evaluate(`document.querySelector('[data-output="time"]').textContent`), '1.15 s');
  const svg = await command('browsingContext.captureScreenshot', { context, origin: 'document' }); await writeFile(join(out, 'human-workbench.png'), Buffer.from(svg.data, 'base64'));
  const savedText = await readFile(join(downloads, 'cyan.json'), 'utf8'); await writeFile(join(out, 'cyan.json'), savedText);
  const saved = JSON.parse(savedText);
  assert.equal(saved.parts.find(p => p.id === 'hair_left').pivot[0], 201);
  assert.equal(saved.parts.find(p => p.id === 'hair_left').channels[0].gain, 7.5); record('actual_download_contains_rig_edits', true);
  assert.equal(saved.nodes.find(n => n.id === 'head').channels[0].gain, 9.5); record('layer_and_rig_controls_edit_independently', true);
  const importText = text => evaluate(`new Promise(resolve => {
    const input=document.querySelector('[data-input="import"]'), transfer=new DataTransfer();
    transfer.items.add(new File([${JSON.stringify(text)}], 'import.json', {type:'application/json'}));
    const status=document.querySelector('[data-output="error"]');
    const observer=new MutationObserver(()=>{observer.disconnect();resolve(true)}); observer.observe(status,{childList:true,subtree:true}); observer.observe(document.querySelector('[data-output="status"]'),{childList:true,subtree:true});
    input.files=transfer.files; input.dispatchEvent(new Event('change',{bubbles:true}));
  })`);
  const before = await path('face'); await importText('{"schema":"wrong"}');
  assert.equal(await path('face'), before); assert.ok(await evaluate(`document.querySelector('[data-output="error"]').textContent.length > 0`)); record('invalid_import_preserves_character', true);
  await importText(JSON.stringify(saved)); assert.equal(await evaluate(`document.querySelector('[data-input="pivotX"]').value`), '300');
  await set('[data-input="part"]', 'hair_left'); assert.equal(await evaluate(`document.querySelector('[data-input="pivotX"]').value`), '201'); record('import_roundtrip_preserves_edit', true);
  await set('[data-input="subject"]', 'hacker_bird'); const beak = await path('beak_lower'); await set('[data-parameter="speech"]', 1, 'input'); assert.notEqual(await path('beak_lower'), beak);
  const paw = await path('paw_left'); await set('[data-parameter="typing"]', 1, 'input'); assert.notEqual(await path('paw_left'), paw); record('bird_custom_controls', true);
  const bird = await command('browsingContext.captureScreenshot', { context, origin: 'document' }); await writeFile(join(out, 'bird-workbench.png'), Buffer.from(bird.data, 'base64'));
  await command('browsingContext.setViewport', { context, viewport: { width: 360, height: 900 } });
  assert.equal(await evaluate(`document.documentElement.scrollWidth <= innerWidth`), true); record('narrow_layout_no_overflow', true);
  const narrow = await command('browsingContext.captureScreenshot', { context, origin: 'document' }); await writeFile(join(out, 'narrow-workbench.png'), Buffer.from(narrow.data, 'base64'));
  await command('browsingContext.setViewport', { context, viewport: { width: 1100, height: 950 } });
  await set('[data-input="subject"]','cyan_vtuber');
  assert.equal(await evaluate(`document.querySelectorAll('[data-raster]').length`),15);
  assert.equal(await evaluate(`document.querySelectorAll('.control-handle').length`),0);
  const textureTransform=()=>evaluate(`document.querySelector('[data-part="face"] [data-raster-transform]').getAttribute('transform')`);
  const rasterNeutral=await textureTransform(); await set('[data-parameter="head"]',.7,'input'); assert.notEqual(await textureTransform(),rasterNeutral);
  await set('[data-input="time"]',1.28,'input');
  for(const side of ['left','right']) { assert.equal(await evaluate(`document.querySelector('[data-part="eye_${side}"]').getAttribute('opacity')`),'0'); assert.equal(await evaluate(`document.querySelector('[data-part="closed_${side}"]').getAttribute('opacity')`),'1'); }
  record('textured_pose_and_bilateral_blink',true);
  await click('[data-action="neutral"]');
  await set('[data-input="part"]','torso'); await set('[data-input="layerX"]',91);
  const texturedDownload=waitDownload(); await click('[data-action="export"]'); assert.equal((await texturedDownload).params.status,'complete');
  const rasterText=await readFile(join(downloads,'cyan_vtuber.json'),'utf8'); const rasterSaved=JSON.parse(rasterText);
  assert.equal(rasterSaved.parts.find(p=>p.id==='torso').raster.rect[0],91); assert.ok(rasterSaved.textures[0].data.startsWith('data:image/png;base64,'));
  await importText(rasterText); await set('[data-input="part"]','torso'); assert.equal(await evaluate(`document.querySelector('[data-input="layerX"]').value`),'91'); record('textured_download_and_import_preserve_art_and_placement',true);
  await command('browsingContext.activate',{context});
  await evaluate(`new Promise(resolve=>{const a=document.querySelector('[data-output="time"]'),old=a.textContent; const observer=new MutationObserver(()=>{if(a.textContent!==old){observer.disconnect();resolve(true);}}); observer.observe(a,{childList:true,subtree:true}); document.querySelector('[data-action="play"]').click();})`);
  await click('[data-action="play"]'); assert.equal(await evaluate(`document.querySelector('[data-action="play"]').getAttribute('aria-pressed')`),'false'); record('textured_elapsed_time_play_pause',true);
  const corrupt=structuredClone(rasterSaved); const png=Buffer.from(corrupt.textures[0].data.slice(22),'base64'); corrupt.textures[0].data='data:image/png;base64,'+png.subarray(0,33).toString('base64');
  await importText(JSON.stringify(corrupt)); assert.equal(await evaluate(`document.querySelector('#luxel-workbench').dataset.project`),'cyan_vtuber'); assert.ok(await evaluate(`document.querySelector('[data-output="error"]').textContent.length>0`)); record('undecodable_png_preserves_active_project',true);
  await importText(rasterText);
  await click('[data-action="neutral"]'); await click('[data-input="rig"]');
  const humanArt=await command('browsingContext.captureScreenshot',{context,origin:'document'}); await writeFile(join(out,'human-textured.png'),Buffer.from(humanArt.data,'base64'));
  await set('[data-input="time"]',1.28,'input');
  const humanBlink=await command('browsingContext.captureScreenshot',{context,origin:'document'}); await writeFile(join(out,'human-textured-blink.png'),Buffer.from(humanBlink.data,'base64'));
  await set('[data-parameter="head"]',.7,'input'); await set('[data-parameter="gazeX"]',-.7,'input'); await set('[data-parameter="blinkL"]',0,'input'); await set('[data-parameter="blinkR"]',0,'input'); await set('[data-parameter="speech"]',.6,'input');
  const humanCombined=await command('browsingContext.captureScreenshot',{context,origin:'document'}); await writeFile(join(out,'human-textured-combined.png'),Buffer.from(humanCombined.data,'base64'));
  await set('[data-input="subject"]','ornith_chibi'); await set('[data-parameter="speech"]',.7,'input'); await set('[data-parameter="shades"]',1,'input');
  assert.equal(await evaluate(`document.querySelector('[data-part="sunglasses"]').getAttribute('opacity')`),'1'); record('chibi_accessory_and_beak_controls',true);
  const birdArt=await command('browsingContext.captureScreenshot',{context,origin:'document'}); await writeFile(join(out,'bird-textured.png'),Buffer.from(birdArt.data,'base64'));
  await set('[data-parameter="shades"]',0,'input');
  const birdEyes=await command('browsingContext.captureScreenshot',{context,origin:'document'}); await writeFile(join(out,'bird-textured-eyes.png'),Buffer.from(birdEyes.data,'base64'));
  await set('[data-input="time"]',1.28,'input');
  const birdBlink=await command('browsingContext.captureScreenshot',{context,origin:'document'}); await writeFile(join(out,'bird-textured-blink.png'),Buffer.from(birdBlink.data,'base64'));
  await command('browsingContext.setViewport',{context,viewport:{width:360,height:900}});
  assert.equal(await evaluate('document.documentElement.scrollWidth <= innerWidth'),true); record('textured_narrow_layout_no_overflow',true);
  const narrowArt=await command('browsingContext.captureScreenshot',{context,origin:'document'}); await writeFile(join(out,'narrow-textured.png'),Buffer.from(narrowArt.data,'base64'));
  const errors = events.filter(e => e.method === 'log.entryAdded' && e.params.level === 'error'); assert.equal(errors.length, 0, JSON.stringify(errors));
  record('no_browser_errors', true); evidence.passed = true;
  }
} catch (error) {
  console.error(error); evidence.failure = error.stack; evidence.browserLogs=events.filter(e=>e.method==='log.entryAdded'); process.exitCode = 1;
  if (activeContext) {
    const state = await command('script.evaluate', { expression: `JSON.stringify({visibility:document.visibilityState,time:document.querySelector('[data-output="time"]')?.textContent,error:document.querySelector('[data-output="error"]')?.textContent,play:document.querySelector('[data-action="play"]')?.textContent})`, target: { context: activeContext }, awaitPromise: false });
    evidence.pageState = state.result?.value;
  }
}
finally {
  console.log('Closing isolated browser');
  if (ws) ws.close(); for (const call of pending.values()) call.reject(new Error('Browser closed')); pending.clear();
  try { process.kill(-browser.pid, 'SIGTERM'); } catch {}
  await new Promise(resolveExit => { if (browser.exitCode !== null || browser.signalCode !== null) resolveExit(); else browser.once('exit', resolveExit); });
  await rm(profile, { recursive: true, force: true });
  await writeFile(join(out, 'browser.log'), log); await writeFile(join(out, 'browser-receipt.json'), JSON.stringify(evidence, null, 2));
  console.log(JSON.stringify(evidence, null, 2));
}
