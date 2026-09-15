'use strict';
// Read-only product diagnosis: actual repository mux functions + isolated media.
const fs=require('node:fs'),fsp=require('node:fs/promises'),path=require('node:path'),vm=require('node:vm'),{spawn}=require('node:child_process'),{performance}=require('node:perf_hooks');
function loadRendererAudioApi(overrides={}){
  const src=fs.readFileSync(path.join(__dirname,'html-video-renderer/render.mjs'),'utf8');
  const context=vm.createContext({spawn,path,process,Buffer,...fsp,emitProgress:()=>{},...overrides});
  vm.runInContext(src.slice(src.indexOf('function run(command'),src.indexOf('async function resolveVideoProfile'))+'\n'+src.slice(src.indexOf('function animationActiveDuration'),src.indexOf('async function launchRendererBrowser'))+'\nthis.api={run,runOutput,muxPresentationAudio,buildAudioPlan,prepareAudioInputs};',context);
  return context.api;
}
async function main(){
  const out=path.resolve(process.argv[2]||'.tmp/video-audit-20260909');
  fs.mkdirSync(out,{recursive:false});
  const {run,runOutput,muxPresentationAudio,buildAudioPlan}=loadRendererAudioApi();
  const ff=process.env.UNIPPT_FFMPEG_PATH||'ffmpeg',probe=process.env.UNIPPT_FFPROBE_PATH||'ffprobe';
  const silent=path.join(out,'silent-4s.mp4');
  await run(ff,['-y','-f','lavfi','-i','color=c=blue:s=640x360:r=30:d=4','-c:v','libx264','-threads','1','-pix_fmt','yuv420p',silent]);
  const result={scope:'Actual repository audio mux function on synthetic media; not user deck or full export.',cases:[]};
  for(const [name,duration,trimEndMs]of [['short-audio',1,0],['trim-end',4,2000]]){
    const audio=path.join(out,`${name}.wav`),output=path.join(out,`${name}.mp4`),workspace=path.join(out,name);await fsp.mkdir(workspace);
    await run(ff,['-y','-f','lavfi','-i',`sine=frequency=440:duration=${duration}`,'-c:a','pcm_s16le',audio]);
    const id='a'.repeat(64),asset={encoded:(await fsp.readFile(audio)).toString('base64'),mimeType:'audio/wav'};
    const slide={objects:[{id:'audio',media:{asset:'unippt-asset:'+id,trimEndMs}}],animations:[{targetObjectId:'audio',effect:'media',mediaAction:'play',durationMs:1000}]};
    const manifest={deck:{slides:[slide]},assets:new Map([[id,asset]])};
    const started=performance.now();await muxPresentationAudio(manifest,silent,output,[4],workspace);
    const metadata=JSON.parse(await runOutput(probe,['-v','error','-show_streams','-show_format','-of','json',output]));
    result.cases.push({name,elapsedMs:performance.now()-started,expectedVideoSeconds:4,sourceAudioSeconds:duration,requestedTrimEndMs:trimEndMs,plan:JSON.parse(JSON.stringify(buildAudioPlan(manifest,[4]),(k,v)=>k==='asset'?undefined:v)),formatDuration:Number(metadata.format.duration),streams:metadata.streams.map(s=>({type:s.codec_type,codec:s.codec_name,duration:Number(s.duration),frames:s.nb_frames}))});
    await fsp.writeFile(path.join(out,`${name}-probe.json`),JSON.stringify(metadata,null,2));
  }
  await fsp.writeFile(path.join(out,'result.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
}
if(require.main===module)main().catch(e=>{console.error(e.stack);process.exitCode=1;});
module.exports={loadRendererAudioApi};
