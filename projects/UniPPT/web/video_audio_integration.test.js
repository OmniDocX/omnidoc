'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),fsp=require('node:fs/promises'),path=require('node:path'),os=require('node:os'),{spawn,spawnSync}=require('node:child_process');
const {loadRendererAudioApi}=require('../tools/audit_video_export.cjs');
const ff=process.env.UNIPPT_FFMPEG_PATH||'ffmpeg',probe=process.env.UNIPPT_FFPROBE_PATH||'ffprobe';
function binary(command,args){return new Promise((resolve,reject)=>{const child=spawn(command,args,{windowsHide:true,stdio:['ignore','pipe','pipe']});const chunks=[];let err='';child.stdout.on('data',b=>chunks.push(b));child.stderr.on('data',b=>err+=b);child.on('error',reject);child.on('close',code=>code?reject(Error(err)):resolve(Buffer.concat(chunks)));});}
async function level(file,start){const pcm=await binary(ff,['-v','error','-ss',String(start),'-i',file,'-t','0.25','-vn','-ac','1','-ar','8000','-f','f32le','pipe:1']);let energy=0;for(let i=0;i<pcm.length;i+=4)energy+=pcm.readFloatLE(i)**2;assert.ok(pcm.length>0,'Audio track must cover the complete video');return Math.sqrt(energy/(pcm.length/4));}
test('real FFmpeg: short/trimmed/loop/delayed/mixed audio never truncates video and probes are shared',async t=>{
 if(spawnSync(ff,['-version'],{windowsHide:true}).status!==0||spawnSync(probe,['-version'],{windowsHide:true}).status!==0){t.skip('Requires FFmpeg and ffprobe');return;}
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'unippt-audio-qa-'));t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
 const silent=path.join(root,'silent.mp4');await binary(ff,['-y','-f','lavfi','-i','color=c=blue:s=640x360:r=30:d=4','-c:v','libx264','-threads','1','-pix_fmt','yuv420p',silent]);
 const assets=new Map();for(const [id,seconds]of [['a'.repeat(64),1],['b'.repeat(64),4]]){const wav=path.join(root,id+'.wav');await binary(ff,['-y','-f','lavfi','-i',`sine=frequency=440:duration=${seconds}`,'-c:a','pcm_s16le',wav]);assets.set(id,{encoded:fs.readFileSync(wav).toString('base64'),mimeType:'audio/wav'});}
 const track=(id,media={},delayMs=0)=>({objects:[{id:'audio-'+id,media:{asset:'unippt-asset:'+id,...media}}],animations:[{targetObjectId:'audio-'+id,effect:'media',mediaAction:'play',delayMs,durationMs:100}]});
 const cases=[
  {name:'short',slides:[track('a'.repeat(64))],durations:[4],sound:.5,silence:3},
  {name:'trim-end',slides:[track('b'.repeat(64),{trimEndMs:2000})],durations:[4],sound:.5,silence:3},
  {name:'trim-both',slides:[track('b'.repeat(64),{trimStartMs:1000,trimEndMs:2000})],durations:[4],sound:.5,silence:2},
  {name:'delayed',slides:[track('a'.repeat(64),{},1000)],durations:[4],sound:1.5,silence:3,leadingSilence:.25},
  {name:'loop-reuse',slides:[track('a'.repeat(64),{trimEndMs:500,loopPlayback:true}),track('a'.repeat(64),{trimEndMs:500,loopPlayback:true})],durations:[2,2],sound:3,probeCount:1,loopPreparations:1},
  {name:'mixed-short',slides:[(()=>{const a=track('a'.repeat(64)),b=track('b'.repeat(64),{trimEndMs:3000},500);return {objects:[...a.objects,...b.objects],animations:[...a.animations,...b.animations.map(e=>({...e,trigger:'withPrevious'}))]};})()],durations:[4],sound:.75,silence:3},
  {name:'fully-trimmed',slides:[track('a'.repeat(64),{trimEndMs:1000})],durations:[4],noAudio:true},
 ];
 for(const c of cases)await t.test(c.name,async()=>{
  const workspace=path.join(root,c.name);await fsp.mkdir(workspace);const input=path.join(workspace,'silent.mp4'),output=path.join(workspace,'final.mp4');await fsp.copyFile(silent,input);
  const calls=[],api=loadRendererAudioApi({spawn:(cmd,args,options)=>{calls.push({cmd,args});return spawn(cmd,args,options);}});
  await api.muxPresentationAudio({deck:{slides:c.slides},assets},input,output,c.durations,workspace);
  const metadata=JSON.parse((await binary(probe,['-v','error','-show_streams','-show_format','-of','json',output])).toString());
  const video=metadata.streams.find(s=>s.codec_type==='video');assert.equal(Number(video.nb_frames),120);assert.ok(Math.abs(Number(video.duration)-4)<.01);assert.ok(Math.abs(Number(metadata.format.duration)-4)<.05);
  if(c.noAudio)assert.equal(metadata.streams.some(s=>s.codec_type==='audio'),false);
  else {assert.ok(await level(output,c.sound)>.03,'Expected audible content');if(c.silence!==undefined)assert.ok(await level(output,c.silence)<.0001,'Expected silent padded tail');if(c.leadingSilence!==undefined)assert.ok(await level(output,c.leadingSilence)<.0001);}
  if(c.probeCount)assert.equal(calls.filter(x=>/ffprobe(?:\.exe)?$/i.test(x.cmd)).length,c.probeCount);
  if(c.loopPreparations)assert.equal(calls.filter(x=>/ffmpeg(?:\.exe)?$/i.test(x.cmd)&&x.args.at(-1)?.endsWith('.wav')).length,c.loopPreparations);
 });
 await t.test('failed probe is explicit, not silent audio loss',async()=>{
  const workspace=path.join(root,'probe-failed');await fsp.mkdir(workspace);
  const api=loadRendererAudioApi({spawn:(cmd,args,options)=>spawn(/ffprobe(?:\.exe)?$/i.test(cmd)?path.join(root,'missing-ffprobe.exe'):cmd,args,options)});
  await assert.rejects(api.muxPresentationAudio({deck:{slides:[track('a'.repeat(64))]},assets},silent,path.join(workspace,'final.mp4'),[4],workspace),/ENOENT/);
 });
});
