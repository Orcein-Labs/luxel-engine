import { compileProject, evaluateCharacter, transformPoint } from './runtime.mjs';
import { mountCharacter, paintCharacter, prepareTextures } from './renderer.mjs';

const cases = [
  ['neutral',{}],
  ['blink',{blinkL:1,blinkR:1}],
  ['mid-expression',{blinkL:.5,blinkR:.5,speech:.5}],
  ['speaking',{speech:1}],
  ['left-opposed',{head:-1,body:1,sway:-1,gazeX:-1,gazeY:1,speech:.6}],
  ['right-opposed',{head:1,body:-1,sway:1,gazeX:1,gazeY:-1,speech:.6}],
];

async function raster(svg, canvasSize) {
  const copy=svg.cloneNode(true); copy.setAttribute('width',canvasSize[0]); copy.setAttribute('height',canvasSize[1]);
  const image=new Image(); image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(copy));
  await image.decode();
  const canvas=document.createElement('canvas'); [canvas.width,canvas.height]=canvasSize;
  const context=canvas.getContext('2d',{willReadFrequently:true}); context.drawImage(image,0,0);
  return {canvas, pixels:context.getImageData(0,0,...canvasSize).data};
}

function maxInternalGap(values) {
  const first=values.findIndex(v=>v>=128),last=values.findLastIndex(v=>v>=128);
  if(first<0||first>5||last<values.length-6) return 100;
  let maximum=0,run=0;
  for(let i=first;i<=last;i++){run=values[i]<128?run+1:0;maximum=Math.max(maximum,run);}
  return maximum;
}

async function neckProbe(svg, character, pose) {
  const isolated=svg.cloneNode(true);
  for(const node of isolated.querySelectorAll('[data-artwork] [data-part]')) if(!['face','torso','neck_bridge'].includes(node.dataset.part)) node.remove();
  const {pixels}=await raster(isolated,character.project.canvas), width=character.project.canvas[0];
  // Coordinates belong to this authored Cyan recipe, not a universal anatomy rule.
  const matrix=pose.parts.find(p=>p.id==='torso').matrix;
  const rays=[294,300,306].map(x=>{
    const alpha=[];
    for(let y=320;y<=395;y++){
      const [a,b]=transformPoint(matrix,[x,y]).map(Math.round);
      alpha.push(pixels[4*(b*width+a)+3]??0);
    }
    return {x,max_gap_px:maxInternalGap(alpha),alpha};
  });
  return {passed:rays.every(r=>r.max_gap_px<=1),rays};
}

async function visiblePixels(svg, character, ids) {
  const isolated=svg.cloneNode(true);
  for(const node of isolated.querySelectorAll('[data-artwork] [data-part]'))if(!ids.includes(node.dataset.part))node.remove();
  const {pixels}=await raster(isolated,character.project.canvas);
  let count=0,x=0,y=0;
  for(let i=0;i<pixels.length;i+=4)if(pixels[i+3]>=128){count++;x+=(i/4)%character.project.canvas[0];y+=Math.floor(i/4/character.project.canvas[0]);}
  return {count,center:count?[x/count,y/count]:null};
}

async function birdProbe(svg, character, pose, label) {
  if(label==='neutral')return {beak:await visiblePixels(svg,character,['beak_lower']),glasses:await visiblePixels(svg,character,['sunglasses'])};
  if(!['bare-eyes','gaze-left','gaze-right'].includes(label))return null;
  const offsets=[];
  for(const side of ['left','right']){
    const window=pose.parts.find(p=>p.id==='eye_window_'+side);
    if(!window)return {offsets:null};
    const iris=await visiblePixels(svg,character,['iris_'+side]),bounds=window.bounds;
    offsets.push(iris.center?[iris.center[0]-(bounds[0]+bounds[2])/2,iris.center[1]-(bounds[1]+bounds[3])/2]:null);
  }
  return {offsets};
}

export async function generateVisualReview(projects, reference, {requireJoin=false}={}) {
  const root=document.getElementById('luxel-review'), records=[];
  const title=document.createElement('h1');title.textContent='Luxel visual QA — art acceptance remains open';root.append(title);
  const note=document.createElement('p');note.textContent='Actual native composition. Join measurements cover declared neck rays; they do not grade attractiveness or certify all poses.';root.append(note);
  const referenceCard=document.createElement('figure'),referenceImage=document.createElement('img'),referenceLabel=document.createElement('figcaption');
  referenceImage.src=reference;referenceImage.alt='Original female design reference';referenceLabel.textContent='Female reference — flattened design study';referenceCard.append(referenceImage,referenceLabel);root.append(referenceCard);
  const grid=document.createElement('div');grid.className='review-grid';root.append(grid);
  for(const project of projects){
    const character=compileProject(project);await prepareTextures(character);
    const reviewCases=project.id==='ornith_chibi'?[...cases.map(([label,values])=>[label,label==='neutral'?values:{...values,shades:0}]),['bare-eyes',{shades:0}],['gaze-left',{shades:0,gazeX:-1}],['gaze-right',{shades:0,gazeX:1}]]:cases;
    for(const [label,values] of reviewCases){
      const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('xmlns','http://www.w3.org/2000/svg');
      const pose=evaluateCharacter(character,values),view=mountCharacter(svg,character,()=>{});paintCharacter(view,pose);
      const render=await raster(svg,character.project.canvas), png=render.canvas.toDataURL('image/png');
      const join=project.id==='cyan_vtuber'?await neckProbe(svg,character,pose):null;
      const bird=project.id==='ornith_chibi'?await birdProbe(svg,character,pose,label):null;
      const figure=document.createElement('figure'),image=document.createElement('img'),caption=document.createElement('figcaption');
      image.src=png;image.alt=project.label+' '+label;caption.textContent=project.label+' · '+label+(join?' · neck '+(join.passed?'rays pass':'GAP FOUND'):'');
      figure.append(image,caption);grid.append(figure);records.push({subject:project.id,label,values,join,bird,png});
    }
  }
  const control=structuredClone(projects.find(p=>p.id==='cyan_vtuber'));
  for(const p of control.parts) if(p.node==='head')p.channels.push({parameter:'head',property:'y',gain:-60});
  const c=compileProject(control),pose=evaluateCharacter(c,{head:1}),svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('xmlns','http://www.w3.org/2000/svg');
  paintCharacter(mountCharacter(svg,c,()=>{}),pose);
  const detached=await neckProbe(svg,c,pose);
  const joinPassed=records.filter(r=>r.join).every(r=>r.join.passed);
  const bird=label=>records.find(r=>r.subject==='ornith_chibi'&&r.label===label).bird;
  const centered=bird('bare-eyes').offsets,left=bird('gaze-left').offsets,right=bird('gaze-right').offsets;
  const gazePassed=Boolean(centered&&left&&right&&centered.every((v,i)=>v&&Math.abs(v[0])<2&&Math.abs(v[1])<2&&left[i]&&right[i]&&left[i][0]<v[0]-1&&right[i][0]>v[0]+1)&&Math.abs(centered[0][0]-centered[1][0])<1);
  const birdPassed=bird('neutral').beak.count>100&&bird('neutral').glasses.count>500&&gazePassed;
  const eyeControl=structuredClone(projects.find(p=>p.id==='ornith_chibi'));
  eyeControl.parts.find(p=>p.id==='iris_right').channels.find(c=>c.parameter==='gazeX').gain*=-1;
  const ec=compileProject(eyeControl),ep=evaluateCharacter(ec,{gazeX:1,shades:0}),es=document.createElementNS('http://www.w3.org/2000/svg','svg');es.setAttribute('xmlns','http://www.w3.org/2000/svg');
  paintCharacter(mountCharacter(es,ec,()=>{}),ep);
  const opposite=await birdProbe(es,ec,ep,'gaze-right');
  const oppositeRejected=Boolean(opposite.offsets&&opposite.offsets[0][0]>1&&opposite.offsets[1][0]<-1);
  for(const id of ['beak_lower','sunglasses'])es.querySelector('[data-part="'+id+'"]').setAttribute('opacity','0');
  const missing=await birdProbe(es,ec,ep,'neutral');
  const missingRejected=missing.beak.count===0&&missing.glasses.count===0;
  const report={schema:'luxel.visual-review/v1',functional_art_acceptance:false,join_scope:'Three authored Cyan rays, alpha >=128, <=1 internal transparent pixel, six fixed poses; missing endpoint coverage is reported as 100',join_passed:joinPassed,ornith_parts_and_gaze_passed:birdPassed,detached_control_rejected:!detached.passed,detached_control:detached,opposite_gaze_control_rejected:oppositeRejected,opposite_gaze_control:opposite,missing_parts_control_rejected:missingRejected,cases:records.map(({png,...rest})=>rest)};
  root.dataset.complete='true';
  if(requireJoin&&(!joinPassed||!birdPassed||detached.passed)) throw new Error('Visual assembly gate failed');
  return {report,images:records.map(({subject,label,png})=>({subject,label,png}))};
}
