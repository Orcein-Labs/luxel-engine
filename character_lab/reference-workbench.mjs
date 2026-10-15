import { evaluateCharacter } from './runtime.mjs';
import { mountCharacter, paintCharacter, prepareTextures } from './renderer.mjs';
import { separateReference, compareReference, refPathString } from './reference-separation.mjs';
import { REFERENCE_ASSET, REFERENCE_RECIPE } from './reference-assets.mjs';

const refRoot=document.getElementById('luxel-reference'),refControl=name=>refRoot.querySelector('[data-ref="'+name+'"]'),refAction=name=>refRoot.querySelector('[data-ref-action="'+name+'"]');
const refSvg=refRoot.querySelector('[data-ref-stage]'),refHistory=[];
let refRecipe,refCharacter,refView,refRevision=0,refSequence=0,refSelected='head_face',refPoint=0,refDrag=null,refFlicker=false;
function refBoundary(){return refSelected==='silhouette'?refRecipe.silhouette:refRecipe.regions.find(r=>r.id===refSelected).path;}
function refCoordinates(){return refBoundary().flatMap((c,i)=>c[0]==='Z'?[]:[{index:i,p:c.slice(1)}]);}
function refInspector(){
  refControl('layer').value=refSelected;const options=refCoordinates();refPoint=Math.min(refPoint,options.length-1);refControl('point').replaceChildren();
  options.forEach((c,i)=>{const o=document.createElement('option');o.value=String(i);o.textContent='Point '+(i+1);refControl('point').append(o);});refControl('point').value=String(refPoint);
  const p=options[refPoint].p;refControl('x').value=String(p[0]);refControl('y').value=String(p[1]);
}
function refDraw(){
  refView=mountCharacter(refSvg,refCharacter,()=>{});paintCharacter(refView,evaluateCharacter(refCharacter));
  if(refControl('isolate').checked)for(const [id,p]of refView.paths)if(refCharacter.parts.get(id).raster&&id!==refSelected)p.setAttribute('display','none');
  if(refControl('boundary').checked){
    const line=document.createElementNS('http://www.w3.org/2000/svg','path');line.setAttribute('d',refPathString(refBoundary()));line.setAttribute('fill','none');line.setAttribute('stroke','#ffba62');line.setAttribute('stroke-width','2');refView.overlay.append(line);
    refCoordinates().forEach((c,i)=>{const point=document.createElementNS('http://www.w3.org/2000/svg','circle');point.setAttribute('cx',c.p[0]);point.setAttribute('cy',c.p[1]);point.setAttribute('r',i===refPoint?'6':'3');point.setAttribute('fill',i===refPoint?'white':'#ffba62');point.dataset.refPoint=String(i);point.style.pointerEvents='all';refView.overlay.append(point);});
  }
  refRoot.dataset.revision=String(++refRevision);refShowMode();
}
function refShowMode(){
  const mode=refControl('mode').value;refSvg.style.opacity=mode==='reference'||refFlicker?'0':'1';
  const reference=refRoot.querySelector('[data-ref-overlay]');reference.style.opacity=mode==='reference'||refFlicker?'1':mode==='overlay'?String(refControl('opacity').value):'0';
  refControl('difference').hidden=mode!=='difference';refSvg.style.visibility=mode==='difference'?'hidden':'visible';
}
async function refMeasure(){
  const token=++refSequence;delete refRoot.dataset.measured;refControl('status').textContent='Checking reconstruction…';
  try{const result=await compareReference(refSvg,REFERENCE_ASSET,refRecipe,refCharacter.project);if(token!==refSequence)return;
    refControl('report').textContent=JSON.stringify(result.report,null,2);refControl('difference').src=result.difference;
    refControl('status').textContent=refControl('isolate').checked?'Isolated layer · full-character comparison intentionally differs':result.report.passed?'Visible interiors preserved · boundary appearance still needs review':'Reconstruction differs · inspect the highlighted regions';
    refRoot.dataset.measured=String(refRevision);
  }catch(error){if(token===refSequence){refControl('error').textContent=error.message;refRoot.dataset.measured=String(refRevision);}}
}
function refAccept(candidate,remember=true){
  try{const next=separateReference(REFERENCE_ASSET,candidate);if(remember&&refRecipe){refHistory.push(structuredClone(refRecipe));if(refHistory.length>20)refHistory.shift();}refRecipe=next.recipe;refCharacter=next.character;refControl('error').textContent='';refInspector();refDraw();refMeasure();return true;}
  catch(error){refControl('error').textContent=error.message;if(refRecipe)refInspector();refRoot.dataset.revision=String(++refRevision);return false;}
}
function refSave(name,value){const url=URL.createObjectURL(new Blob([JSON.stringify(value,null,2)+'\n'],{type:'application/json'})),a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
for(const r of [{id:'silhouette',label:'Character silhouette'},...REFERENCE_RECIPE.regions]){const o=document.createElement('option');o.value=r.id;o.textContent=r.label;refControl('layer').append(o);}
refControl('layer').addEventListener('change',()=>{refSelected=refControl('layer').value;refPoint=0;refInspector();refDraw();refMeasure();});
refControl('point').addEventListener('change',()=>{refPoint=Number(refControl('point').value);refInspector();refDraw();});
for(const [axis,name]of ['x','y'].entries())refControl(name).addEventListener('change',()=>{const next=structuredClone(refRecipe),path=refSelected==='silhouette'?next.silhouette:next.regions.find(r=>r.id===refSelected).path;path[refCoordinates()[refPoint].index][axis+1]=Number(refControl(name).value);refAccept(next);});
for(const name of ['isolate','boundary'])refControl(name).addEventListener('change',()=>{refDraw();refMeasure();});
for(const name of ['mode','opacity'])refControl(name).addEventListener('input',()=>{refFlicker=false;refShowMode();});
refControl('background').addEventListener('change',()=>{for(const e of refRoot.querySelectorAll('[data-ref-canvas]'))e.dataset.background=refControl('background').value;});
refControl('zoom').addEventListener('input',()=>{refRoot.style.setProperty('--ref-zoom',refControl('zoom').value+'%');});
refAction('flicker').addEventListener('click',()=>{refFlicker=!refFlicker;refShowMode();});refAction('check').addEventListener('click',refMeasure);
refAction('undo').addEventListener('click',()=>{const p=refHistory.at(-1);if(p&&refAccept(p,false))refHistory.pop();});
refAction('reset').addEventListener('click',()=>refAccept(REFERENCE_RECIPE));
refAction('recipe').addEventListener('click',()=>refSave('ornith-original-separation.json',refRecipe));
refAction('native').addEventListener('click',()=>refSave('ornith-original-native.json',refCharacter.project));
refControl('import').addEventListener('change',async()=>{const revision=refRevision;try{const file=refControl('import').files[0];if(!file)return;if(file.size>150000)throw new Error('Recipe exceeds size limit');const text=await file.text();if(revision===refRevision)refAccept(JSON.parse(text));}catch(error){if(revision===refRevision){refControl('error').textContent=error.message;refRoot.dataset.revision=String(++refRevision);}}finally{refControl('import').value='';}});
refSvg.addEventListener('pointerdown',event=>{if(event.target.dataset.refPoint===undefined)return;refPoint=Number(event.target.dataset.refPoint);refInspector();refHistory.push(structuredClone(refRecipe));if(refHistory.length>20)refHistory.shift();refDrag=event.pointerId;refSvg.setPointerCapture(event.pointerId);event.preventDefault();});
refSvg.addEventListener('pointermove',event=>{if(refDrag!==event.pointerId)return;const p=refSvg.createSVGPoint();p.x=event.clientX;p.y=event.clientY;const q=p.matrixTransform(refSvg.getScreenCTM().inverse()),next=structuredClone(refRecipe),path=refSelected==='silhouette'?next.silhouette:next.regions.find(r=>r.id===refSelected).path;path[refCoordinates()[refPoint].index]=[path[refCoordinates()[refPoint].index][0],Math.round(q.x),Math.round(q.y)];refAccept(next,false);});
for(const event of ['pointerup','pointercancel'])refSvg.addEventListener(event,e=>{if(refDrag===e.pointerId){refDrag=null;if(refSvg.hasPointerCapture(e.pointerId))refSvg.releasePointerCapture(e.pointerId);}});
try{const initial=separateReference(REFERENCE_ASSET,REFERENCE_RECIPE);await prepareTextures(initial.character);if(refAccept(REFERENCE_RECIPE,false))refRoot.dataset.ready='true';}catch(error){refControl('error').textContent=error.message;}
