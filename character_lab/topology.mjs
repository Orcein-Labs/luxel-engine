import { compileProject, rectanglePath } from './runtime.mjs';
import { assemblyDigest } from './assembly.mjs';

const fail=(ok,message)=>{if(!ok)throw new Error(message);};
const cross=(a,b,c)=>(b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]);
const area=points=>points.reduce((sum,p,i)=>{const q=points[(i+1)%points.length];return sum+p[0]*q[1]-q[0]*p[1];},0)/2;
const path=points=>points.map((p,i)=>[i?'L':'M',...p]).concat([['Z']]);
function touches(a,b,p){return Math.abs(cross(a,b,p))<1e-8&&p[0]>=Math.min(a[0],b[0])&&p[0]<=Math.max(a[0],b[0])&&p[1]>=Math.min(a[1],b[1])&&p[1]<=Math.max(a[1],b[1]);}
function intersects(a,b,c,d){return cross(a,b,c)*cross(a,b,d)<0&&cross(c,d,a)*cross(c,d,b)<0||touches(a,b,c)||touches(a,b,d)||touches(c,d,a)||touches(c,d,b);}

export function validateOpening(points,canvas){
  fail(Array.isArray(points)&&points.length>=3&&points.length<=32,'Opening needs 3–32 points');
  for(const p of points)fail(Array.isArray(p)&&p.length===2&&p.every((v,i)=>Number.isFinite(v)&&v>=0&&v<=canvas[i]),'Opening point outside canvas');
  fail(Math.abs(area(points))>=1,'Opening polygon is degenerate');
  for(let i=0;i<points.length;i++){
    fail(Math.hypot(...points[i].map((v,j)=>v-points[(i+1)%points.length][j]))>=1,'Opening has duplicate or adjacent points');
    for(let j=i+1;j<points.length;j++)if(j!==i+1&&!(i===0&&j===points.length-1))fail(!intersects(points[i],points[(i+1)%points.length],points[j],points[(j+1)%points.length]),'Opening polygon intersects itself');
  }
  return area(points)>0?structuredClone(points):structuredClone(points).reverse();
}

export async function partitionLayer(source,input){
  const original=compileProject(source),recipe=structuredClone(input);
  const keys=['schema','project','source_digest','part','subjects','back_order','front_order','front_polygon'];
  fail(recipe&&typeof recipe==='object'&&!Array.isArray(recipe)&&Object.keys(recipe).every(k=>keys.includes(k)),'Invalid partition recipe');
  fail(recipe.schema==='luxel.layer-partition/v1'&&recipe.project===source.id,'Partition belongs to another project');
  fail(recipe.source_digest===await assemblyDigest(source),'Stale partition source');
  const part=original.parts.get(recipe.part);fail(part?.raster,'Partition requires a raster layer');
  fail(Array.isArray(recipe.subjects)&&recipe.subjects.length>0&&recipe.subjects.length<=128&&new Set(recipe.subjects).size===recipe.subjects.length&&recipe.subjects.every(id=>id!==part.id&&original.parts.has(id)),'Invalid surrounded subject layers');
  for(const value of [recipe.back_order,recipe.front_order])fail(Number.isFinite(value)&&Math.abs(value)<=1000,'Invalid partition order');
  fail(recipe.subjects.every(id=>recipe.back_order<original.parts.get(id).order&&recipe.front_order>original.parts.get(id).order),'Front/rear layers must surround every subject layer');
  const polygon=validateOpening(recipe.front_polygon,source.canvas),ids=[part.id+'_rear_mask',part.id+'_front_mask',part.id+'_front'];
  fail(ids.every(id=>id.length<=64&&!original.parts.has(id)),'Partition part IDs collide');
  const [x,y,w,h]=part.raster.rect;fail(x>=0&&y>=0&&x+w<=source.canvas[0]&&y+h<=source.canvas[1],'Off-canvas raster partition is unsupported');
  const project=structuredClone(source),rear=project.parts.find(p=>p.id===part.id),front=structuredClone(rear);
  const mask=(id,shape)=>({id,label:id,node:part.node,pivot:part.pivot.slice(),order:recipe.back_order,visible:false,fill:'none',stroke:'none',strokeWidth:0,path:shape,channels:structuredClone(part.channels),deforms:[],mask:part.mask});
  const outer=rectanglePath([0,0,...source.canvas]);
  project.parts.push(mask(ids[0],[...outer,...path(polygon.slice().reverse())]),mask(ids[1],path(polygon)));
  rear.mask=ids[0];rear.order=recipe.back_order;
  front.id=ids[2];front.label=part.label+' · front region';front.mask=ids[1];front.order=recipe.front_order;project.parts.push(front);
  return {character:compileProject(project),recipe,roles:{rear:part.id,front:ids[2],rearMask:ids[0],frontMask:ids[1]}};
}

export async function ornithOpeningRecipe(source){
  return {schema:'luxel.layer-partition/v1',project:source.id,source_digest:await assemblyDigest(source),part:'hoodie',subjects:source.parts.filter(p=>p.node==='head').map(p=>p.id),back_order:5,front_order:65,
    front_polygon:[[140,415],[205,415],[245,435],[300,441],[355,435],[405,415],[460,415],[460,620],[140,620]]};
}
