import { compileProject, rectanglePath } from './runtime.mjs';

const refAssert=(ok,message)=>{if(!ok)throw new Error(message);};
function refFields(value,keys){refAssert(value&&typeof value==='object'&&!Array.isArray(value)&&Object.keys(value).every(k=>keys.includes(k)),'Unsupported separation fields');}
function refPolygon(points){
  const cross=(a,b,c)=>(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]);
  refAssert(Math.abs(points.reduce((sum,a,i)=>{const b=points[(i+1)%points.length];return sum+a[0]*b[1]-b[0]*a[1];},0))>0,'Degenerate boundary');
  for(let i=0;i<points.length;i++)for(let j=i+2;j<points.length;j++){
    if(i===0&&j===points.length-1)continue;const a=points[i],b=points[(i+1)%points.length],c=points[j],d=points[(j+1)%points.length];
    refAssert(!(cross(a,b,c)*cross(a,b,d)<0&&cross(c,d,a)*cross(c,d,b)<0),'Self-intersecting boundary');
  }
}
function refPath(commands,canvas){
  refAssert(Array.isArray(commands)&&commands.length>3&&commands.length<=512,'Invalid boundary size');let open=false,points=[];
  for(const [op,...values]of commands){refAssert(['M','L','Z'].includes(op)&&values.length===(op==='Z'?0:2),'Only closed polygon boundaries are supported');
    if(op==='M'){refAssert(!open,'Unclosed boundary');open=true;points=[values];}else if(op==='Z'){refAssert(open&&points.length>=3,'Invalid closed boundary');refPolygon(points);open=false;}else{refAssert(open,'Boundary must start with M');points.push(values);}
    refAssert(values.every((v,i)=>Number.isFinite(v)&&v>=0&&v<=canvas[i]),'Boundary point outside source');
  }refAssert(!open,'Unclosed boundary');
}
function reverseRefPath(commands){const result=[];let points=[];for(const c of commands){if(c[0]==='M')points=[c.slice(1)];else if(c[0]==='L')points.push(c.slice(1));else{points.reverse();result.push(['M',...points[0]],...points.slice(1).map(p=>['L',...p]),['Z']);}}return result;}
function refExclusionGroups(regions){
  const bounds=r=>{const p=r.path.filter(c=>c[0]!=='Z');return [Math.min(...p.map(c=>c[1])),Math.min(...p.map(c=>c[2])),Math.max(...p.map(c=>c[1])),Math.max(...p.map(c=>c[2]))];};
  const overlaps=(a,b)=>a[0]<=b[2]&&b[0]<=a[2]&&a[1]<=b[3]&&b[1]<=a[3],groups=[];
  for(const region of regions){const box=bounds(region),entry={region,box};const group=groups.find(g=>g.every(e=>!overlaps(e.box,box))&&5+g.reduce((n,e)=>n+e.region.path.length,0)+region.path.length<=512);if(group)group.push(entry);else groups.push([entry]);}
  refAssert(groups.length<=6,'Exclusions require more than six mask levels; simplify overlapping regions');return groups.map(g=>g.map(e=>e.region));
}
export function separateReference(asset,input){
  const recipe=structuredClone(input);refFields(recipe,['schema','source_digest','canvas','silhouette','regions','coverage']);
  refAssert(recipe.schema==='luxel.reference-separation/v1'&&recipe.source_digest===asset.digest,'Stale or foreign reference recipe');
  refAssert(JSON.stringify(recipe.canvas)===JSON.stringify(asset.size),'Reference canvas differs');refPath(recipe.silhouette,recipe.canvas);
  refAssert(Array.isArray(recipe.regions)&&recipe.regions.length>=2&&recipe.regions.length<=12,'Separation needs 2–12 regions');
  const ids=new Set();for(const r of recipe.regions){refFields(r,['id','label','path']);refAssert(/^[a-z][a-z0-9_]{0,29}$/.test(r.id)&&!ids.has(r.id),'Invalid or duplicate region');ids.add(r.id);refAssert(typeof r.label==='string'&&r.label.length<=60,'Invalid region label');refPath(r.path,recipe.canvas);}
  refAssert(Array.isArray(recipe.coverage)&&recipe.coverage.length<=16,'Invalid coverage ledger');for(const c of recipe.coverage){refFields(c,['movement','status','finding']);refAssert([c.movement,c.finding].every(s=>typeof s==='string'&&s.length>0&&s.length<300)&&c.status==='unsupported_pending_authored_coverage','Unproven motion readiness');}
  const rect=[0,0,...asset.size],outer=rectanglePath(rect),parts=[];
  const mask=(id,path,under=null)=>{parts.push({id,label:id,node:'root',pivot:[0,0],order:-1,visible:false,fill:'none',stroke:'none',strokeWidth:0,path,channels:[],deforms:[],mask:under,maskSampling:'pixel'});return id;};
  mask('source_silhouette',recipe.silhouette);
  for(let i=0;i<recipe.regions.length;i++){
    const region=recipe.regions[i];let under=mask('region_'+region.id,region.path,'source_silhouette');
    for(const [j,group]of refExclusionGroups(recipe.regions.slice(i+1)).entries())under=mask('cut_'+i+'_'+j,[...outer,...group.flatMap(r=>reverseRefPath(r.path))],under);
    parts.push({id:region.id,label:region.label,node:'root',pivot:[0,0],order:i,visible:true,fill:'none',stroke:'none',strokeWidth:0,path:outer,channels:[],deforms:[],mask:under,raster:{texture:'reference',source:rect,rect}});
  }
  const project={schema:'luxel.layered-character/v2',id:'ornith_original_front',label:'Ornith — original artwork separation',canvas:asset.size,parameters:[],nodes:[{id:'root',parent:null,pivot:[0,0],channels:[]}],parts,clips:[],bindings:{},textures:[{id:'reference',size:asset.size,data:asset.png}],provenance:{kind:'original-source-with-editable-native-masks',source_digest:asset.digest,motion_readiness:'unsupported_pending_authored_coverage',coverage:recipe.coverage}};
  return {character:compileProject(project),recipe};
}
export function referenceStructure(project,recipe){
  const rasters=project.parts.filter(p=>p.raster),expected=recipe.regions.map(r=>r.id);
  return rasters.length===expected.length&&rasters.every(p=>expected.includes(p.id)&&p.mask&&p.raster.texture==='reference')&&project.textures.length===1&&project.clips.length===0&&project.parameters.length===0;
}
export function refPathString(commands){return commands.map(c=>c.join(' ')).join(' ');}
export async function referenceRaster(svg){
  const copy=svg.cloneNode(true);copy.querySelector('[data-overlay]')?.remove();copy.style.opacity='1';copy.style.visibility='visible';copy.setAttribute('xmlns','http://www.w3.org/2000/svg');const [w,h]=copy.getAttribute('viewBox').split(' ').slice(2).map(Number);copy.setAttribute('width',w);copy.setAttribute('height',h);
  const image=new Image();image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(new XMLSerializer().serializeToString(copy));await image.decode();const canvas=document.createElement('canvas');canvas.width=w;canvas.height=h;canvas.getContext('2d').drawImage(image,0,0);return canvas;
}
export async function compareReference(svg,asset,recipe,project){
  const [w,h]=asset.size,render=await referenceRaster(svg),actual=render.getContext('2d').getImageData(0,0,w,h).data;
  const source=new Image();source.src=asset.original;await source.decode();const canvas=document.createElement('canvas');canvas.width=w;canvas.height=h;const ctx=canvas.getContext('2d');ctx.drawImage(source,0,0);const original=ctx.getImageData(0,0,w,h).data;
  ctx.clearRect(0,0,w,h);ctx.save();ctx.clip(new Path2D(refPathString(recipe.silhouette)));recipe.regions.forEach((r,i)=>{ctx.fillStyle='rgb('+(i+1)+',0,0)';ctx.fill(new Path2D(refPathString(r.path)));});ctx.restore();const owners=ctx.getImageData(0,0,w,h).data;
  const difference=ctx.createImageData(w,h),regions=recipe.regions.map(r=>({id:r.id,interior:0,changed:0}));let changed=0,leaked=0,boundary=0,boundaryChanged=0,internalBoundaryChanged=0,interior=0,maxDelta=0,conversionDelta=0;
  const png=new Image();png.src=asset.png;await png.decode();ctx.clearRect(0,0,w,h);ctx.drawImage(png,0,0);const decoded=ctx.getImageData(0,0,w,h).data;
  for(let y=0;y<h;y++)for(let x=0;x<w;x++){
    const k=4*(y*w+x),owner=Math.round(owners[k]);let edge=owners[k+3]>0&&owners[k+3]<255,outerEdge=owners[k+3]!==255;
    for(let dy=-2;dy<=2;dy++)for(let dx=-2;dx<=2;dx++){const a=x+dx,b=y+dy;if(a>=0&&a<w&&b>=0&&b<h){const n=4*(b*w+a);if(owners[n]!==owners[k])edge=true;if(owners[n+3]===0)outerEdge=true;}}
    for(let c=0;c<3;c++)conversionDelta=Math.max(conversionDelta,Math.abs(decoded[k+c]-original[k+c]));
    let delta=0;for(let c=0;c<3;c++)delta=Math.max(delta,Math.abs(actual[k+c]-original[k+c]));
    if(edge){boundary++;if(actual[k+3]!==255||delta>1){boundaryChanged++;if(owner&&!outerEdge)internalBoundaryChanged++;difference.data[k]=255;difference.data[k+1]=170;difference.data[k+3]=160;}continue;}
    if(owner){interior++;regions[owner-1].interior++;maxDelta=Math.max(maxDelta,delta);if(delta>1||actual[k+3]!==255){changed++;regions[owner-1].changed++;difference.data[k]=255;difference.data[k+3]=255;}}
    else if(actual[k+3]>0){leaked++;difference.data[k]=255;difference.data[k+2]=255;difference.data[k+3]=255;}
  }
  ctx.putImageData(difference,0,0);
  const children=[...svg.querySelector('[data-artwork]').children],structural=referenceStructure(project,recipe)&&children.length===project.parts.length&&children.every(e=>project.parts.some(p=>p.id===e.dataset.part));
  return {report:{interior,changed,leaked,boundary,boundaryChanged,internalBoundaryChanged,maxDelta,conversionDelta,regions,structural,passed:changed===0&&leaked===0&&internalBoundaryChanged===0&&conversionDelta===0&&structural,visual_acceptance:false,motion_ready:false,boundary_policy:'Two source pixels around authored region boundaries; fixed before evaluation. Internal seams also checked.'},difference:canvas.toDataURL('image/png'),render:render.toDataURL('image/png')};
}
