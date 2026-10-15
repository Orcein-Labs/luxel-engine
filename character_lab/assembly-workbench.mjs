import { evaluateCharacter } from './runtime.mjs';
import { mountCharacter, paintCharacter, prepareTextures } from './renderer.mjs';
import { fitAssembly, inspectAssembly } from './assembly.mjs';
import { BUILTINS } from './fixtures.mjs';
import { ASSEMBLY_GUIDES } from './assembly-guides.mjs';

const desk=document.getElementById('luxel-assembly'),field=name=>desk.querySelector('[data-fit="'+name+'"]');
const sources=new Map(BUILTINS.filter(p=>ASSEMBLY_GUIDES.some(g=>g.project===p.id)).map(p=>[p.id,p]));
const recipes=new Map(ASSEMBLY_GUIDES.map(g=>[g.project,structuredClone(g)]));
let source,guide,draft,fitted,request=0,revision=0;
const poses={neutral:{},blink:{blinkL:1,blinkR:1},midpoint:{blinkL:.5,blinkR:.5,speech:.5},speaking:{speech:1},left:{head:-1,body:1,sway:-1},right:{head:1,body:-1,sway:1}};
function selectedFitGroup(){return guide.groups.find(g=>g.id===field('group').value);}
function groupFields(){const g=selectedFitGroup();for(const [name,value]of [['x',g.offset[0]],['y',g.offset[1]],['scale',g.scale]])field(name).value=String(value);}
function groupOptions(preferred=''){field('group').replaceChildren();for(const g of guide.groups){const o=document.createElement('option');o.value=g.id;o.textContent=g.label;field('group').append(o);}if(guide.groups.some(g=>g.id===preferred))field('group').value=preferred;}
function draw(svg,fit,report){
  const values={...poses[field('pose').value]};if(fit.character.parameters.has('shades')&&field('eyes').checked)values.shades=0;
  paintCharacter(mountCharacter(svg,fit.character,()=>{}),evaluateCharacter(fit.character,values));
  if(!field('landmarks').checked)return;
  const layer=document.createElementNS('http://www.w3.org/2000/svg','g');layer.setAttribute('data-fit-overlay','');svg.append(layer);
  const positions=new Map(report.landmarks.map(l=>[l.id,l.world]));
  const element=(name,attrs)=>{const e=document.createElementNS('http://www.w3.org/2000/svg',name);for(const [k,v]of Object.entries(attrs))e.setAttribute(k,String(v));layer.append(e);return e;};
  for(const c of report.contacts){const [a,b]=[positions.get(c.a),positions.get(c.b)];element('line',{x1:a[0],y1:a[1],x2:b[0],y2:b[1],stroke:c.passed?'#66ded0':'#ffb45b','stroke-width':3});}
  for(const l of report.landmarks){const [x,y]=l.world;element('circle',{cx:x,cy:y,r:4,fill:'#66ded0',stroke:'#18242c','stroke-width':1});const t=element('text',{x:x+7,y:y-7,fill:'#ffffff','font-size':11,'paint-order':'stroke',stroke:'#16232e','stroke-width':3});t.textContent=l.label;}
}
async function refresh(candidate=draft??guide){
  draft=candidate;
  const token=++request,activeSource=source;
  for(const button of desk.querySelectorAll('[data-fit-action="source"],[data-fit-action="guide"]'))button.disabled=true;
  try{
    const [next,original]=await Promise.all([fitAssembly(activeSource,candidate),fitAssembly(activeSource,candidate,{apply:false})]);
    await prepareTextures(next.character);
    if(token!==request)return false;
    guide=candidate;fitted=next;recipes.set(source.id,structuredClone(guide));groupOptions(field('group').value);
    const values=poses[field('pose').value],a=inspectAssembly(original,values),b=inspectAssembly(next,values);
    draw(desk.querySelector('[data-stage="original"]'),original,a);draw(desk.querySelector('[data-stage="fitted"]'),next,b);
    field('report').textContent=JSON.stringify({original:a,fitted:b},null,2);
    field('summary').textContent='Declared geometry: '+(b.geometry_passed?'within guide':'outside guide')+' · art acceptance open';
    field('findings').replaceChildren();for(const f of guide.findings){const li=document.createElement('li');li.textContent=f.detail;field('findings').append(li);}
    field('error').textContent='';groupFields();desk.dataset.revision=String(++revision);desk.dataset.ready='true';return true;
  }catch(error){if(token===request){draft=guide;field('error').textContent=error.message;groupFields();desk.dataset.revision=String(++revision);}return false;}
  finally{if(token===request)for(const button of desk.querySelectorAll('[data-fit-action="source"],[data-fit-action="guide"]'))button.disabled=!fitted||fitted.character.project.id!==source.id;}
}
function activate(){
  source=sources.get(field('subject').value);guide=structuredClone(recipes.get(source.id));draft=guide;groupOptions();
  field('pose').value='neutral';for(const image of desk.querySelectorAll('[data-reference]'))image.hidden=image.dataset.reference!==source.id;
  groupFields();return refresh();
}
function download(name,value){const url=URL.createObjectURL(new Blob([JSON.stringify(value,null,2)+'\n'],{type:'application/json'})),a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
for(const p of sources.values()){const o=document.createElement('option');o.value=p.id;o.textContent=p.label;field('subject').append(o);}
field('subject').addEventListener('change',activate);field('group').addEventListener('change',groupFields);
for(const name of ['pose','landmarks','eyes'])field(name).addEventListener('change',()=>refresh());
for(const name of ['x','y','scale'])field(name).addEventListener('change',()=>{const next=structuredClone(draft??guide),g=next.groups.find(g=>g.id===field('group').value);if(name==='scale')g.scale=Number(field(name).value);else g.offset[name==='x'?0:1]=Number(field(name).value);refresh(next);});
desk.querySelector('[data-fit-action="reset"]').addEventListener('click',()=>{recipes.set(source.id,structuredClone(ASSEMBLY_GUIDES.find(g=>g.project===source.id)));activate();});
desk.querySelector('[data-fit-action="source"]').addEventListener('click',()=>download(source.id+'-fitted.json',fitted.character.project));
desk.querySelector('[data-fit-action="guide"]').addEventListener('click',()=>download(source.id+'-guide.json',guide));
field('import').addEventListener('change',async()=>{
  const token=request,projectId=source.id;
  try{const file=field('import').files[0];if(!file)return;if(file.size>100000)throw new Error('Guide exceeds size limit');const candidate=JSON.parse(await file.text());if(token!==request||projectId!==source.id)return;await refresh(candidate);}
  catch(error){field('error').textContent=error.message;desk.dataset.revision=String(++revision);}finally{field('import').value='';}
});
activate();
