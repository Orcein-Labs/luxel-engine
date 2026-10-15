import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { separateReference, referenceStructure } from './reference-separation.mjs';
import { REFERENCE_ASSET as asset, REFERENCE_RECIPE as recipe } from './reference-assets.mjs';

function contains(commands,x,y){
  let first,previous,winding=0;
  const edge=(a,b)=>{const cross=(b[0]-a[0])*(y-a[1])-(x-a[0])*(b[1]-a[1]);if(a[1]<=y&&b[1]>y&&cross>0)winding++;if(a[1]>y&&b[1]<=y&&cross<0)winding--;};
  for(const [op,...p]of commands){if(op==='M')first=previous=p;else if(op==='L'){edge(previous,p);previous=p;}else edge(previous,first);}return winding!==0;
}
test('original pixels retain one resource, bounded native parts and no unearned movement',async()=>{
  const source=await readFile(new URL('./references/ornith-front-original.webp',import.meta.url));
  assert.equal(createHash('sha256').update(source).digest('hex'),asset.digest);
  const before=JSON.stringify(recipe),{character}=separateReference(asset,recipe),p=character.project;
  assert.equal(JSON.stringify(recipe),before);assert.equal(p.textures.length,1);assert.equal(p.parts.filter(p=>p.raster).length,10);assert.ok(p.parts.length<=128);
  assert.equal(p.clips.length+p.parameters.length,0);assert.ok(p.provenance.coverage.every(c=>c.status==='unsupported_pending_authored_coverage'));assert.ok(referenceStructure(p,recipe));
});
test('native mask chains give each covered source point exactly one visible owner',()=>{
  const {character}=separateReference(asset,recipe),p=character.project,parts=new Map(p.parts.map(p=>[p.id,p]));
  const nativeContains=(part,x,y)=>contains(part.path,x,y)&&(!part.mask||nativeContains(parts.get(part.mask),x,y));
  for(let y=.37;y<1024;y+=13)for(let x=.19;x<1024;x+=13){
    const owners=p.parts.filter(part=>part.raster&&nativeContains(part,x,y));
    const expected=contains(recipe.silhouette,x,y)?recipe.regions.findLast(r=>contains(r.path,x,y)):null;
    assert.deepEqual(owners.map(p=>p.id),expected?[expected.id]:[],`ownership at ${x},${y}`);
  }
  for(const region of recipe.regions)assert.ok(p.parts.find(p=>p.id===region.id).mask);
});
test('stale sources, invalid boundaries and invented readiness are rejected without changing accepted input',()=>{
  const accepted=JSON.stringify(recipe),cases=[r=>r.source_digest='old',r=>r.regions[2].path[0][1]=-1,r=>r.regions[2].path.pop(),r=>r.regions[2].path=[['M',0,0],['L',10,10],['L',0,10],['L',10,0],['L',20,20],['Z']],r=>r.coverage[0].status='ready',r=>r.regions[0].id=r.regions[1].id];
  for(const mutate of cases){const next=structuredClone(recipe);mutate(next);assert.throws(()=>separateReference(asset,next));assert.equal(JSON.stringify(recipe),accepted);}
});
test('a flattened underlay cannot satisfy the separation structure gate',()=>{
  const {character}=separateReference(asset,recipe),next=structuredClone(character.project);
  next.parts.push({...next.parts.find(p=>p.raster),id:'flattened_crutch',mask:null});assert.equal(referenceStructure(next,recipe),false);
});
