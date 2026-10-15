import { readFile, writeFile } from 'node:fs/promises';
import { fitAssembly, inspectAssembly } from './assembly.mjs';

try{
  const [operation,projectPath,guidePath,outputPath,...extra]=process.argv.slice(2);
  if(!['inspect','fit'].includes(operation)||!projectPath||!guidePath||extra.length||(operation==='fit')!==Boolean(outputPath))throw new Error('Usage: assembly-cli.mjs inspect SOURCE GUIDE | fit SOURCE GUIDE NEW_OUTPUT');
  const source=JSON.parse(await readFile(projectPath,'utf8')),guide=JSON.parse(await readFile(guidePath,'utf8'));
  const baseline=await fitAssembly(source,guide,{apply:false}),fitted=await fitAssembly(source,guide);
  const report={baseline:inspectAssembly(baseline),fitted:inspectAssembly(fitted),poses:[{head:-1,body:1},{head:1,body:-1}].map(p=>inspectAssembly(fitted,p))};
  if(operation==='fit')await writeFile(outputPath,JSON.stringify(fitted.character.project,null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify(report,null,2));
}catch(error){console.error(error.message);process.exitCode=1;}
