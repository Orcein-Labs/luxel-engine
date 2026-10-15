import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { fitAssembly, inspectAssembly, assemblyDigest } from './assembly.mjs';

const read=async path=>JSON.parse(await readFile(new URL(path,import.meta.url),'utf8'));
const source=await read('../assets/avatars/native/cyan.json');
test('independent similarity oracle, contact failure, source immutability and fit serialization',async()=>{
  const p=structuredClone(source);p.parameters=[];p.clips=[];p.bindings={};p.nodes=[{id:'root',parent:null,pivot:[0,0],channels:[]}];
  p.parts=[{id:'moving',label:'Triangle',node:'root',pivot:[10,20],order:0,visible:true,fill:'#ffffff',stroke:'none',strokeWidth:0,channels:[],mask:null,deforms:[],path:[['M',10,20],['L',12,20],['L',10,22],['Z']]},
    {id:'fixed',label:'Target',node:'root',pivot:[30,40],order:1,visible:true,fill:'#ffffff',stroke:'none',strokeWidth:0,channels:[],mask:null,deforms:[],path:[['M',30,40],['L',32,40],['L',30,42],['Z']]}];
  const g={schema:'luxel.assembly-guide/v1',project:p.id,source_digest:await assemblyDigest(p),label:'Analytical fit',groups:[{id:'move',label:'Move',parts:['moving'],nodes:[],anchor:'a',target:[30,40],offset:[0,0],scale:1.5}],landmarks:[{id:'a',label:'A',part:'moving',point:[10,20]},{id:'b',label:'B',part:'fixed',point:[30,40]}],contacts:[{label:'Contact',a:'a',b:'b',max_distance:0}],ratios:[],occlusion:[],joints:[],findings:[]};
  const before=JSON.stringify(p),guideBefore=JSON.stringify(g),f=await fitAssembly(p,g);
  assert.deepEqual(f.character.project.parts[0].path,[['M',30,40],['L',33,40],['L',30,43],['Z']]);
  assert.deepEqual(f.character.project.parts[0].pivot,[30,40]);assert.equal(inspectAssembly(f).geometry_passed,true);
  assert.equal(inspectAssembly(await fitAssembly(p,g,{apply:false})).geometry_passed,false);
  const broken=structuredClone(g);broken.groups[0].offset=[20,0];assert.equal(inspectAssembly(await fitAssembly(p,broken)).geometry_passed,false);
  assert.equal(JSON.stringify(p),before);assert.equal(JSON.stringify(g),guideBefore);
});

test('actual human/chibi guide contacts survive named poses and preserve source texture bytes',async()=>{
  for(const id of ['cyan_vtuber','ornith_chibi']){
    const p=await read('../assets/avatars/native/'+id+'.json'),g=await read('./guides/'+id+'.json'),fit=await fitAssembly(p,g);
    assert.equal(inspectAssembly(await fitAssembly(p,g,{apply:false})).geometry_passed,false);
    for(const pose of [{},{head:-1,body:1},{head:1,body:-1},{blinkL:.5,blinkR:.5,speech:.5}])assert.equal(inspectAssembly(fit,pose).geometry_passed,true);
    assert.deepEqual(fit.character.project.textures,p.textures);
    if(id==='ornith_chibi')assert.deepEqual(fit.character.nodes.get('head').pivot,[300,445]);
  }
});

test('stale, foreign, malformed and partially moved rig guides fail closed',async()=>{
  const p=await read('../assets/avatars/native/ornith_chibi.json'),g=await read('./guides/ornith_chibi.json');
  for(const change of [x=>{x.source_digest='0'.repeat(64);},x=>{x.project='other';},x=>{x.groups[0].scale=NaN;},x=>{x.groups[0].scale=0;},x=>{x.groups[0].parts=['missing'];},x=>{x.groups=[];},x=>{x.groups[0].parts.pop();},x=>{x.groups[0].target='absent';},x=>{x.script='no';}]){
    const bad=structuredClone(g);change(bad);await assert.rejects(()=>fitAssembly(p,bad));
  }
  const changed=structuredClone(p);changed.label+=' changed';await assert.rejects(()=>fitAssembly(changed,g),/Stale/);
});

test('real CLI publishes readable fitted source and refuses an existing output',async()=>{
  const directory=await mkdtemp(join(tmpdir(),'luxel-assembly-'));
  try{
    const cli=fileURLToPath(new URL('./assembly-cli.mjs',import.meta.url)),p=fileURLToPath(new URL('../assets/avatars/native/ornith_chibi.json',import.meta.url)),g=fileURLToPath(new URL('./guides/ornith_chibi.json',import.meta.url)),out=join(directory,'fitted.json');
    const args=[cli,'fit',p,g,out],report=JSON.parse(execFileSync(process.execPath,args,{encoding:'utf8'}));
    assert.equal(report.fitted.geometry_passed,true);assert.equal(report.fitted.visual_acceptance,false);
    const first=await readFile(out,'utf8');assert.equal(JSON.parse(first).parts.find(p=>p.id==='paw_left').raster.rect[1],551);
    assert.throws(()=>execFileSync(process.execPath,args,{stdio:'pipe'}));assert.equal(await readFile(out,'utf8'),first);
  }finally{await rm(directory,{recursive:true,force:true});}
});
