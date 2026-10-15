import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { assemblyDigest, fitAssembly } from './assembly.mjs';
import { partitionLayer, ornithOpeningRecipe } from './topology.mjs';

const loadJSON=async path=>JSON.parse(await readFile(new URL(path,import.meta.url),'utf8'));
const original=await loadJSON('../assets/avatars/native/ornith_chibi.json'),guide=await loadJSON('./guides/ornith_chibi.json');
const fitted=(await fitAssembly(original,guide)).character.project;
test('compiled front and rear masks agree with an independent rectangle oracle for either winding',async()=>{
  const contains=(commands,x,y)=>{let winding=0,first,previous;for(const [op,...p]of commands){if(op==='M'){first=p;previous=p;continue;}const next=op==='Z'?first:p,[a,b]=previous,[c,d]=next,cross=(c-a)*(y-b)-(x-a)*(d-b);if(b<=y&&d>y&&cross>0)winding++;if(b>y&&d<=y&&cross<0)winding--;previous=next;}return winding!==0;};
  for(const points of [[[10,20],[40,20],[40,60],[10,60]],[[10,60],[40,60],[40,20],[10,20]]]){
    const recipe=await ornithOpeningRecipe(fitted);recipe.front_polygon=points;const result=await partitionLayer(fitted,recipe);
    for(const x of [5,15,30,45])for(const y of [10,25,50,65]){const expected=x>10&&x<40&&y>20&&y<60;assert.equal(contains(result.character.parts.get(result.roles.frontMask).path,x,y),expected);assert.equal(contains(result.character.parts.get(result.roles.rearMask).path,x,y),!expected);}
  }
});
test('actual partition roundtrips native source with unchanged artwork and matching mask transforms',async()=>{
  const before=JSON.stringify(fitted),recipe=await ornithOpeningRecipe(fitted),saved=JSON.stringify(recipe),result=await partitionLayer(fitted,recipe),p=result.character.project;
  assert.equal(JSON.stringify(fitted),before);assert.equal(JSON.stringify(recipe),saved);assert.deepEqual(p.textures,fitted.textures);
  const rear=result.character.parts.get(result.roles.rear),front=result.character.parts.get(result.roles.front);
  assert.deepEqual(front.raster,rear.raster);assert.deepEqual(front.channels,rear.channels);assert.equal(front.node,rear.node);
  for(const id of [result.roles.frontMask,result.roles.rearMask]){const mask=result.character.parts.get(id);assert.equal(mask.node,rear.node);assert.deepEqual(mask.channels,rear.channels);}
  assert.ok(rear.order<result.character.parts.get('face').order&&front.order>result.character.parts.get('sunglasses').order);
  const copy=JSON.parse(JSON.stringify(p));assert.equal(copy.parts.length,fitted.parts.length+3);
});
test('synthetic raster fixture partitions without bird-specific part names',async()=>{
  const p=structuredClone(fitted);p.parts=p.parts.filter(x=>['hoodie','face'].includes(x.id));p.parts[0].id='garment';p.parts[0].label='Garment';
  const recipe={schema:'luxel.layer-partition/v1',project:p.id,source_digest:await assemblyDigest(p),part:'garment',subjects:['face'],back_order:1,front_order:30,front_polygon:[[0,450],[600,450],[600,700],[0,700]]};
  const result=await partitionLayer(p,recipe);assert.equal(result.roles.front,'garment_front');assert.equal(result.character.parts.get('garment_front').mask,'garment_front_mask');
});
test('stale, colliding, malformed and self-crossing partition recipes are rejected',async()=>{
  const recipe=await ornithOpeningRecipe(fitted);
  for(const mutate of [r=>{r.source_digest='0'.repeat(64);},r=>{r.part='face';r.subjects=['face'];},r=>{r.front_order=21;},r=>{r.back_order=40;},r=>{r.subjects=[];},r=>{r.script='bad';},r=>{r.front_polygon=[[1,1],[2,2],[3,3]];},r=>{r.front_polygon=[[0,0],[100,100],[0,100],[100,0]];},r=>{r.front_polygon[0][0]=NaN;},r=>{r.front_polygon[0][0]=-1;}]){
    const bad=structuredClone(recipe);mutate(bad);await assert.rejects(()=>partitionLayer(fitted,bad));
  }
  const collision=structuredClone(fitted);collision.parts.push({...structuredClone(collision.parts[0]),id:'hoodie_front'});const bad=await ornithOpeningRecipe(collision);await assert.rejects(()=>partitionLayer(collision,bad),/collide/);
});
