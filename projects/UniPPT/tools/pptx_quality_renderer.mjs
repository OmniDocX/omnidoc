// Read-only native-package renderer used by the local web quality endpoint.
// This worker never re-saves PPTX, so native editing properties stay untouched.
import fs from 'node:fs/promises';
import path from 'node:path';
import {createRequire} from 'node:module';
import {pathToFileURL} from 'node:url';
import readline from 'node:readline';
import baselineAdapter from './native_baseline_preview.cjs';
const startup=performance.now();
const [input, output] = process.argv.slice(2);
const root = process.env.UNIPPT_PRESENTATION_NODE_MODULES;
if (!root || !path.isAbsolute(root)) throw new Error('Configure UNIPPT_PRESENTATION_NODE_MODULES for native PPTX quality rendering');
const require = createRequire(path.join(root, '_unippt_quality.cjs'));
const artifactPath=require.resolve('@oai/artifact-tool'),nestedRequire=createRequire(artifactPath);
const {FileBlob, PresentationFile} = await import(pathToFileURL(artifactPath).href);
const artifactVersion=JSON.parse(await fs.readFile(path.join(path.dirname(artifactPath),'../package.json'),'utf8')).version;
const {CanvasRenderingContext2D}=nestedRequire('skia-canvas');
const startupMs=performance.now()-startup;let jobs=0;
async function render(input,output){
 const started=performance.now();const deck=await PresentationFile.importPptx(await FileBlob.load(input));const imported=performance.now();
 if(deck.slides.items.length!==1)throw Error('Quality rendering accepts exactly one slide');
 const nativeScripts=baselineAdapter.baselineRuns(baselineAdapter.slideXml(input));
 if(nativeScripts.unsupported.length)throw Error('Native baseline preview unsupported for '+nativeScripts.unsupported.length+' runs; do not certify formula fidelity');
 if(nativeScripts.runs.length&&artifactVersion!=='2.8.59')throw Error('Revalidate native baseline projection for Artifact Tool '+artifactVersion);
 const projected=await baselineAdapter.paintNativeBaselines(CanvasRenderingContext2D,nativeScripts.runs,()=>deck.export({slide:deck.slides.items[0],format:'png',scale:1}));
 if(projected.report.unmatched.length||projected.report.ambiguous.length)throw Error('Native baseline painting mismatch; preview cannot certify scripts: '+JSON.stringify(projected.report));
 const png=projected.result;
 await fs.writeFile(output,new Uint8Array(await png.arrayBuffer()),{flag:'wx'});
 return {startupMs:jobs++===0?startupMs:0,importMs:imported-started,drawAndWriteMs:performance.now()-imported,workerJobMs:performance.now()-started,baselineProjection:projected.report};
}
if(input==='--worker'){
 // Private stdin owned by the local server, never a user-facing shell/path API.
 const lines=readline.createInterface({input:process.stdin,crlfDelay:Infinity});
 for await(const line of lines){if(line.length>16000)throw Error('Invalid worker job');const job=JSON.parse(line);
   if(!path.isAbsolute(job.input)||!path.isAbsolute(job.output)||path.dirname(job.input)!==path.dirname(job.output))throw Error('Invalid worker workspace');
   let report;try{report={ok:true,timings:await render(job.input,job.output)};}catch(e){report={ok:false,error:String(e.stack||e.message).slice(0,1800)};}
   const reportPath=job.output+'.json';await fs.writeFile(reportPath+'.pending',JSON.stringify(report),{flag:'wx'});await fs.rename(reportPath+'.pending',reportPath);
 }
}else{const timings=await render(input,output);process.stdout.write(JSON.stringify({timings})+'\n');}
