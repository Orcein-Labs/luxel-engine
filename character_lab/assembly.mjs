import { compileProject, evaluateCharacter, rectanglePath, transformPoint } from './runtime.mjs';

function check(ok,message){if(!ok)throw new Error(message);}
function fields(value,keys){check(value&&typeof value==='object'&&!Array.isArray(value),'Expected guide object');for(const key of Object.keys(value))check(keys.includes(key),'Unsupported guide field: '+key);}
function pair(value,limit=10000){check(Array.isArray(value)&&value.length===2&&value.every(v=>Number.isFinite(v)&&Math.abs(v)<=limit),'Invalid guide point');}
function boundedList(value,limit){check(Array.isArray(value)&&value.length<=limit,'Invalid guide list');}
function entries(items){const map=new Map();for(const item of items){check(typeof item.id==='string'&&/^[a-z][a-z0-9_]{0,63}$/.test(item.id)&&!map.has(item.id),'Invalid or duplicate guide id');map.set(item.id,item);}return map;}

export async function assemblyDigest(project){
  const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(JSON.stringify(project)));
  return [...new Uint8Array(digest)].map(v=>v.toString(16).padStart(2,'0')).join('');
}

export async function fitAssembly(source,input,{apply=true}={}){
  const character=compileProject(source), guide=structuredClone(input);
  fields(guide,['schema','project','source_digest','label','groups','landmarks','contacts','ratios','occlusion','joints','findings']);
  check(guide.schema==='luxel.assembly-guide/v1'&&guide.project===source.id,'Guide belongs to a different project');
  check(guide.source_digest===await assemblyDigest(source),'Stale assembly guide: source digest differs');
  check(typeof guide.label==='string'&&guide.label.length<=120,'Invalid guide label');
  for(const [name,limit] of [['groups',16],['landmarks',64],['contacts',32],['ratios',16],['occlusion',16],['joints',16],['findings',16]])boundedList(guide[name],limit);
  check(guide.groups.length>0,'Assembly guide needs a fit group');
  check(guide.contacts.length+guide.ratios.length>0,'Assembly guide needs a declared constraint');
  const landmarks=entries(guide.landmarks);entries(guide.groups);
  for(const l of landmarks.values()){fields(l,['id','label','part','point']);check(character.parts.has(l.part),'Unknown landmark part');pair(l.point);check(typeof l.label==='string'&&l.label.length<=120,'Invalid landmark label');}
  for(const g of guide.groups){
    fields(g,['id','label','parts','nodes','anchor','target','offset','scale']);
    boundedList(g.parts,128);boundedList(g.nodes,64);check(g.parts.length>0&&new Set(g.parts).size===g.parts.length&&g.parts.every(id=>character.parts.has(id)),'Invalid assembly parts');
    check(new Set(g.nodes).size===g.nodes.length&&g.nodes.every(id=>character.nodes.has(id)&&character.project.parts.filter(p=>p.node===id).every(p=>g.parts.includes(p.id))),'A moved rig node requires all of its parts');
    check(g.nodes.every(id=>character.project.nodes.filter(n=>n.parent===id).every(n=>g.nodes.includes(n.id))),'A moved rig node requires its child nodes');
    check(landmarks.has(g.anchor)&&g.parts.includes(landmarks.get(g.anchor).part),'Missing group anchor');
    if(typeof g.target==='string')check(landmarks.has(g.target),'Missing target landmark');else pair(g.target);
    pair(g.offset,200);check(Number.isFinite(g.scale)&&g.scale>=.5&&g.scale<=1.5,'Fit scale outside authored limits');
    check(typeof g.label==='string'&&g.label.length<=120,'Invalid group label');
  }
  for(const c of guide.contacts){fields(c,['label','a','b','max_distance']);check(landmarks.has(c.a)&&landmarks.has(c.b)&&Number.isFinite(c.max_distance)&&c.max_distance>=0&&c.max_distance<=100,'Invalid contact constraint');}
  for(const r of guide.ratios){fields(r,['label','numerator','denominator','target','tolerance']);for(const ids of [r.numerator,r.denominator])check(Array.isArray(ids)&&ids.length===2&&ids.every(id=>landmarks.has(id)),'Invalid ratio landmarks');check(Number.isFinite(r.target)&&r.target>0&&Number.isFinite(r.tolerance)&&r.tolerance>=0,'Invalid ratio constraint');}
  for(const o of guide.occlusion){fields(o,['part','order']);check(character.parts.has(o.part)&&Number.isFinite(o.order)&&Math.abs(o.order)<=1000,'Invalid occlusion order');}
  check(new Set(guide.joints.map(j=>j.node)).size===guide.joints.length,'Duplicate joint target');
  for(const j of guide.joints){fields(j,['node','landmark']);check(character.nodes.has(j.node)&&landmarks.has(j.landmark)&&character.parts.get(landmarks.get(j.landmark).part).node===j.node,'Invalid joint landmark');}
  for(const f of guide.findings){fields(f,['id','detail']);check(typeof f.id==='string'&&typeof f.detail==='string'&&f.detail.length<=1000,'Invalid named finding');}
  const project=structuredClone(source);
  if(apply){
    for(const g of guide.groups){
      const anchor=landmarks.get(g.anchor).point.slice(),target=typeof g.target==='string'?landmarks.get(g.target).point:g.target;
      const move=p=>p.map((v,i)=>target[i]+g.offset[i]+g.scale*(v-anchor[i]));
      for(const p of project.parts.filter(p=>g.parts.includes(p.id))){
        p.pivot=move(p.pivot);
        if(p.raster){const r=p.raster.rect;const start=move(r.slice(0,2));p.raster.rect=[...start,r[2]*g.scale,r[3]*g.scale];p.path=rectanglePath(p.raster.rect);}
        else p.path=p.path.map(c=>{const next=[c[0]];for(let i=1;i<c.length;i+=2)next.push(...move(c.slice(i,i+2)));return next;});
        for(const d of p.deforms)for(const side of ['positive','negative'])d[side]=d[side].map(v=>v*g.scale);
        for(const c of p.channels)if(['x','y'].includes(c.property))c.gain*=g.scale;
        p.strokeWidth*=g.scale;
      }
      for(const n of project.nodes.filter(n=>g.nodes.includes(n.id))){n.pivot=move(n.pivot);for(const c of n.channels)if(['x','y'].includes(c.property))c.gain*=g.scale;}
      for(const l of landmarks.values())if(g.parts.includes(l.part))l.point=move(l.point);
    }
    for(const o of guide.occlusion)project.parts.find(p=>p.id===o.part).order=o.order;
    for(const j of guide.joints)project.nodes.find(n=>n.id===j.node).pivot=landmarks.get(j.landmark).point.slice();
  }
  return {character:compileProject(project),guide,applied:apply};
}

export function inspectAssembly(fit,values={}){
  const pose=evaluateCharacter(fit.character,values),parts=new Map(pose.parts.map(p=>[p.id,p]));
  const points=new Map(fit.guide.landmarks.map(l=>[l.id,transformPoint(parts.get(l.part).matrix,l.point)]));
  const distance=(a,b)=>Math.hypot(...points.get(a).map((v,i)=>v-points.get(b)[i]));
  const contacts=fit.guide.contacts.map(c=>({...c,distance:distance(c.a,c.b),passed:distance(c.a,c.b)<=c.max_distance+1e-8}));
  const ratios=fit.guide.ratios.map(r=>{const denominator=distance(...r.denominator),value=denominator>1e-9?distance(...r.numerator)/denominator:null;return {...r,value,passed:value!==null&&Math.abs(value-r.target)<=r.tolerance};});
  return {schema:'luxel.assembly-report/v1',project:fit.character.project.id,applied:fit.applied,visual_acceptance:false,geometry_passed:contacts.every(c=>c.passed)&&ratios.every(r=>r.passed),contacts,ratios,landmarks:fit.guide.landmarks.map(l=>({...l,world:points.get(l.id)})),findings:fit.guide.findings};
}
