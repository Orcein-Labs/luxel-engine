import { evaluateCharacter, transformPoint } from './runtime.mjs';
import { mountCharacter, paintCharacter, prepareTextures } from './renderer.mjs';
import { fitAssembly } from './assembly.mjs';
import { partitionLayer, ornithOpeningRecipe } from './topology.mjs';
import { BUILTINS } from './fixtures.mjs';
import { ASSEMBLY_GUIDES } from './assembly-guides.mjs';

const maker=document.getElementById('luxel-topology'),control=name=>maker.querySelector('[data-topology="'+name+'"]');
const makerPoses={neutral:{},left:{head:-1,body:1},right:{head:1,body:-1},blink:{blinkL:1,blinkR:1},speaking:{speech:1}};
let baseCharacter,acceptedRecipe,pendingRecipe,assembled,sequence=0,revision=0,selectedPoint=3,dragging=null;
const history=[];
const stages={current:maker.querySelector('[data-topology-stage="current"]'),partition:maker.querySelector('[data-topology-stage="partition"]')};
function pointFields(){const p=acceptedRecipe.front_polygon[selectedPoint];control('x').value=String(p[0]);control('y').value=String(p[1]);control('point').value=String(selectedPoint);}
function pointOptions(){control('point').replaceChildren();acceptedRecipe.front_polygon.forEach((p,i)=>{const option=document.createElement('option');option.value=String(i);option.textContent='Boundary point '+(i+1);control('point').append(option);});selectedPoint=Math.min(selectedPoint,acceptedRecipe.front_polygon.length-1);pointFields();}
function drawMaker(){
  if(!assembled)return;
  const values={...makerPoses[control('pose').value]};if(control('eyes').checked)values.shades=0;
  paintCharacter(mountCharacter(stages.current,baseCharacter,()=>{}),evaluateCharacter(baseCharacter,values));
  const pose=evaluateCharacter(assembled.character,values),view=mountCharacter(stages.partition,assembled.character,()=>{});paintCharacter(view,pose);
  const mode=control('region').value;
  if(mode!=='composition')for(const [id,element]of view.paths)if(id!==(mode==='rear'?assembled.roles.rear:assembled.roles.front))element.setAttribute('display','none');
  if(control('boundary').checked){
    const matrix=pose.parts.find(p=>p.id===assembled.roles.front).matrix;
    const points=acceptedRecipe.front_polygon.map(p=>transformPoint(matrix,p));
    const line=document.createElementNS('http://www.w3.org/2000/svg','polyline');line.setAttribute('points',points.map(p=>p.join(',')).join(' '));line.setAttribute('fill','none');line.setAttribute('stroke','#f7ba63');line.setAttribute('stroke-width','2');line.dataset.openingOverlay='';view.overlay.append(line);
    for(let i=0;i<points.length;i++){
      const c=document.createElementNS('http://www.w3.org/2000/svg','circle');c.setAttribute('cx',points[i][0]);c.setAttribute('cy',points[i][1]);c.setAttribute('r',i===selectedPoint?'7':'5');c.setAttribute('fill',i===selectedPoint?'#ffffff':'#f7ba63');c.dataset.openingPoint=String(i);c.style.pointerEvents='all';view.overlay.append(c);
    }
  }
  control('state').textContent='Editable rear hood + front lip. Visual approval is still required.';
  maker.dataset.revision=String(++revision);
}
async function applyRecipe(candidate,{remember=true}={}){
  pendingRecipe=candidate;const token=++sequence;
  for(const b of maker.querySelectorAll('[data-topology-action="source"],[data-topology-action="recipe"]'))b.disabled=true;
  try{
    const next=await partitionLayer(baseCharacter.project,candidate);if(token!==sequence)return false;
    if(remember&&acceptedRecipe){history.push(structuredClone(acceptedRecipe));if(history.length>20)history.shift();}
    acceptedRecipe=structuredClone(candidate);pendingRecipe=acceptedRecipe;assembled=next;control('error').textContent='';pointOptions();drawMaker();return true;
  }catch(error){if(token===sequence){pendingRecipe=acceptedRecipe;control('error').textContent=error.message;pointFields();maker.dataset.revision=String(++revision);}return false;}
  finally{if(token===sequence)for(const b of maker.querySelectorAll('[data-topology-action="source"],[data-topology-action="recipe"]'))b.disabled=!assembled;}
}
function saveMaker(name,value){const url=URL.createObjectURL(new Blob([JSON.stringify(value,null,2)+'\n'],{type:'application/json'})),a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
control('point').addEventListener('change',()=>{selectedPoint=Number(control('point').value);pointFields();drawMaker();});
for(const name of ['x','y'])control(name).addEventListener('change',()=>{const next=structuredClone(pendingRecipe??acceptedRecipe);next.front_polygon[selectedPoint][name==='x'?0:1]=Number(control(name).value);applyRecipe(next);});
for(const name of ['pose','region','boundary','eyes'])control(name).addEventListener('change',drawMaker);
maker.querySelector('[data-topology-action="undo"]').addEventListener('click',async()=>{const previous=history.at(-1);if(previous&&await applyRecipe(previous,{remember:false}))history.pop();});
maker.querySelector('[data-topology-action="reset"]').addEventListener('click',async()=>applyRecipe(await ornithOpeningRecipe(baseCharacter.project)));
maker.querySelector('[data-topology-action="source"]').addEventListener('click',()=>saveMaker('ornith-opening-native.json',assembled.character.project));
maker.querySelector('[data-topology-action="recipe"]').addEventListener('click',()=>saveMaker('ornith-opening-recipe.json',acceptedRecipe));
control('import').addEventListener('change',async()=>{
  const token=sequence;
  try{const file=control('import').files[0];if(!file)return;if(file.size>100000)throw new Error('Recipe exceeds size limit');const candidate=JSON.parse(await file.text());if(token===sequence)await applyRecipe(candidate);}
  catch(error){control('error').textContent=error.message;maker.dataset.revision=String(++revision);}finally{control('import').value='';}
});
function localOpeningPoint(event){
  const point=stages.partition.createSVGPoint();point.x=event.clientX;point.y=event.clientY;
  const canvasPoint=point.matrixTransform(stages.partition.getScreenCTM().inverse());
  const values={...makerPoses[control('pose').value]};if(control('eyes').checked)values.shades=0;
  const m=evaluateCharacter(assembled.character,values).parts.find(p=>p.id===assembled.roles.front).matrix,det=m[0]*m[3]-m[1]*m[2];
  const x=canvasPoint.x-m[4],y=canvasPoint.y-m[5];return [Math.round((m[3]*x-m[2]*y)/det),Math.round((-m[1]*x+m[0]*y)/det)];
}
stages.partition.addEventListener('pointerdown',event=>{
  if(event.target.dataset.openingPoint===undefined||!assembled)return;
  selectedPoint=Number(event.target.dataset.openingPoint);pointFields();history.push(structuredClone(acceptedRecipe));if(history.length>20)history.shift();dragging=event.pointerId;stages.partition.setPointerCapture(event.pointerId);event.preventDefault();
});
stages.partition.addEventListener('pointermove',event=>{
  if(dragging!==event.pointerId)return;
  const next=structuredClone(pendingRecipe??acceptedRecipe);next.front_polygon[selectedPoint]=localOpeningPoint(event);applyRecipe(next,{remember:false});
});
function endOpeningDrag(event){if(dragging===event.pointerId){dragging=null;if(stages.partition.hasPointerCapture(event.pointerId))stages.partition.releasePointerCapture(event.pointerId);}}
stages.partition.addEventListener('pointerup',endOpeningDrag);stages.partition.addEventListener('pointercancel',endOpeningDrag);
try{
  const original=BUILTINS.find(p=>p.id==='ornith_chibi'),guide=ASSEMBLY_GUIDES.find(g=>g.project===original.id);
  baseCharacter=(await fitAssembly(original,guide)).character;await prepareTextures(baseCharacter);
  if(await applyRecipe(await ornithOpeningRecipe(baseCharacter.project),{remember:false}))maker.dataset.ready='true';
}catch(error){control('error').textContent=error.message;maker.dataset.revision=String(++revision);}
