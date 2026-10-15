import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { assemblyDigest } from './assembly.mjs';

const directory=new URL('./guides/',import.meta.url);await mkdir(directory,{recursive:true});
const mark=(id,label,part,point)=>({id,label,part,point});
const group=(id,label,parts,anchor,target,scale=1,nodes=[])=>({id,label,parts,nodes,anchor,target,offset:[0,0],scale});
const contact=(label,a,b,max_distance)=>({label,a,b,max_distance});
for(const id of ['cyan_vtuber','ornith_chibi']){
  const source=JSON.parse(await readFile(new URL('../assets/avatars/native/'+id+'.json',import.meta.url),'utf8'));
  const human=id==='cyan_vtuber';
  const landmarks=human?[
    mark('eye_line','Eye line','eye_white_left',[300,263]),mark('nose','Nose tip','face',[300,275]),mark('lips','Lip center','mouth_closed',[300,306]),
    mark('chin','Chin','face',[300,332]),mark('neck_seat','Skin at collar','face',[300,344]),mark('collar','Collar top','torso',[300,362]),mark('fringe_top','Front hair crown','hair_front',[303,75]),mark('bridge_top','Concealed skin underpaint','neck_bridge',[300,333])
  ]:[
    mark('head_seat','Head inside hoodie','face',[300,414]),mark('hood_seat','Concealed hood seat','hoodie',[300,445]),
    mark('left_wrist','Left wrist','paw_left',[192,662]),mark('right_wrist','Right wrist','paw_right',[425,662]),
    mark('left_cuff','Left sleeve cuff','hoodie',[170,575]),mark('right_cuff','Right sleeve cuff','hoodie',[433,575]),
    mark('left_grip','Left bezel grip','laptop',[170,575]),mark('right_grip','Right bezel grip','laptop',[433,575])
  ];
  const groups=human?[
    group('eyes','Eye line',source.parts.filter(p=>/^(eye_|iris_|closed_)/.test(p.id)).map(p=>p.id),'eye_line',[300,245]),
    group('mouth','Lip placement',['mouth_closed','mouth_open'],'lips',[300,299]),
    group('fringe','Front fringe',['hair_front'],'fringe_top',[303,75],.86),
    group('collar','Collar to existing skin',['torso'],'collar','neck_seat'),
    group('neck_underpaint','Narrow concealed neck underpaint',['neck_bridge'],'bridge_top',[300,340],.8)
  ]:[
    group('head','Complete head into hood',source.parts.filter(p=>p.node==='head').map(p=>p.id),'head_seat','hood_seat',1,['head']),
    group('left_hand','Left sleeve / grip',['paw_left'],'left_wrist','left_cuff'),
    group('right_hand','Right sleeve / grip',['paw_right'],'right_wrist','right_cuff')
  ];
  const guide={schema:'luxel.assembly-guide/v1',project:id,source_digest:await assemblyDigest(source),label:human?'Female reference proportion study':'Ornith concealed neckline / edge-grip study',groups,landmarks,
    contacts:human?[contact('Skin / collar seat','neck_seat','collar',4)]:[contact('Head / hood overlap anchor','head_seat','hood_seat',4),contact('Left wrist / cuff','left_wrist','left_cuff',5),contact('Right wrist / cuff','right_wrist','right_cuff',5),contact('Left wrist / bezel','left_wrist','left_grip',5),contact('Right wrist / bezel','right_wrist','right_grip',5)],
    ratios:human?[{label:'Eye–nose / nose–lip spacing',numerator:['eye_line','nose'],denominator:['nose','lips'],target:1.4,tolerance:.4}]:[],
    occlusion:human?[{part:'torso',order:25}]:[{part:'hoodie',order:25},{part:'paw_left',order:85},{part:'paw_right',order:85}],joints:human?[]:[{node:'head',landmark:'head_seat'}],findings:[{id:'matte_cleanup',detail:'Colored/transparent atlas edges need authored silhouette cleanup.'},{id:'feature_closure',detail:'Opacity-blended blink/mouth midpoints still ghost; textured deformation is unsupported.'},...(human?[{id:'neck_deformation',detail:'Foreground collar overlaps the skin boundary; connected skin/collar deformation and consistent shading remain unimplemented.'}]:[{id:'reference_gesture',detail:'Raised glasses-touch sleeve/arm and articulated keyboard hand are absent. The lid hides overlapping paw pixels; visible side portions are a placement study, not an articulated grip or that reference pose.'}])]};
  await writeFile(new URL(id+'.json',directory),JSON.stringify(guide,null,2)+'\n');
}
console.log('Authored source-bound human and chibi assembly guides');
