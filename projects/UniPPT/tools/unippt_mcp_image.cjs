'use strict';
// Deterministic image/layout protocol. No OCR, inference, network or executable input.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const {compileModules,normalizeGeometry} = require('../web/mcp_scene.js');
const sourceGeometry = require('../web/image_semantic_module_runtime.js');
const {dashGeometry} = require('./native_path_dash.cjs');
const {legacyCommand} = require('./unippt_mcp_compat.cjs');
const {reviewRegions,regionRects} = require('./unippt_mcp_review.cjs');
const {resolveRows} = require('./unippt_mcp_row_patch.cjs');
const ROOT = path.resolve(__dirname, '..');
const UUID = /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
const RID = /^[\w-]{8,100}$/;
const {schema,textRuns,compileLayout,fitFanPixels,fitEncoderPixels,fitTensorPixels}=require('../web/mcp_image_layout.js');
const TTL = 60 * 60 * 1000;
const hash = value => crypto.createHash('sha256').update(typeof value === 'string' ? value : JSON.stringify(value)).digest('hex');
function sharpRuntime() {
  const modules=process.env.UNIPPT_PRESENTATION_NODE_MODULES;
  if(modules){
    if(!path.isAbsolute(modules))throw Error('UNIPPT_PRESENTATION_NODE_MODULES must be an absolute directory');
    return require(path.join(modules,'sharp'));
  }
  const config=path.join(ROOT,'.unippt-quality-runtime.json');
  if(fs.existsSync(config)){
    const settings=JSON.parse(fs.readFileSync(config,'utf8'));
    return require(path.join(settings.nodeModules,'sharp'));
  }
  return require('sharp');
}
function createStore(root=path.join(ROOT,'.unippt-mcp','image-jobs'),sharp=sharpRuntime()) {
  const dir=id=>{if(!UUID.test(id))throw Error('Invalid image job ID');return path.join(root,id);};
  const load=id=>{const data=JSON.parse(fs.readFileSync(path.join(dir(id),'job.json'),'utf8'));if(Date.now()-data.startedAt>TTL)throw Error('Image job expired; begin again from original');return data;};
  const save=data=>{const target=path.join(dir(data.jobId),'job.json'),tmp=target+'.'+crypto.randomUUID()+'.tmp';fs.writeFileSync(tmp,JSON.stringify(data),{flag:'wx'});fs.renameSync(tmp,target);};
  const summary=job=>({jobId:job.jobId,startedAt:new Date(job.startedAt).toISOString(),elapsedMs:(job.finishedAt||Date.now())-job.startedAt,firstNativeMs:job.firstNativeAt?job.firstNativeAt-job.startedAt:null,budgetMs:120000,budgetPassed:job.finishedAt?job.finishedAt-job.startedAt<=120000:null,modelInclusive:true,clockScope:'First original-image read through caller finish; excludes prior development, browser consent and setup. Includes model description/review and all tool waits.',visualFidelityPassed:false,review:job.review||null});
  async function begin(imagePath,sessionId,prepare) {
    if(typeof imagePath!=='string'||!path.isAbsolute(imagePath)||/^(?:\\\\|\/\/)|[\x00-\x1f]/.test(imagePath)||imagePath.slice(2).includes(':'))throw Error('Use an absolute, local, user-designated raster image path; no URLs, UNC or alternate streams');
    const startedAt=Date.now(),handle=fs.openSync(imagePath,'r');let bytes;
    try{const stat=fs.fstatSync(handle);if(!stat.isFile()||stat.size>8*1024*1024)throw Error('Source must be a regular image file at most 8 MiB');bytes=fs.readFileSync(handle);}finally{fs.closeSync(handle);}
    const decoder=sharp(bytes,{limitInputPixels:16000000,failOn:'error'}),meta=await decoder.metadata();
    if(!['png','jpeg','webp'].includes(meta.format)||(meta.pages||1)!==1||meta.width>8192||meta.height>8192)throw Error('Only single-frame PNG/JPEG/WebP up to 16M pixels/8192 per edge');
    const snapshot=await decoder.rotate().png().toBuffer({resolveWithObject:true});if(snapshot.data.length>16*1024*1024)throw Error('Decoded snapshot too large');
    const context=await prepare();if(!context.document?.size||!context.writeAccess)throw Error('Browser document requires explicit MCP write consent');
    fs.mkdirSync(root,{recursive:true});const entries=fs.readdirSync(root).filter(n=>UUID.test(n));
    if(entries.length>=64||entries.reduce((size,n)=>size+fs.statSync(path.join(root,n,'source.png')).size,0)+snapshot.data.length>128*1024*1024)throw Error('Image snapshot cache full (64 jobs/128 MiB); remove only expired job directories before continuing');
    if(entries.filter(n=>Date.now()-fs.statSync(path.join(root,n)).mtimeMs<TTL).length>=16)throw Error('Too many recent image jobs; limit 16/hour');
    const job={jobId:crypto.randomUUID(),sessionId,startedAt,protocolMode:context.protocolMode,source:{name:path.basename(imagePath),width:snapshot.info.width,height:snapshot.info.height,sha256:hash(snapshot.data.toString('base64'))},page:context.document.size,revision:context.revision,regions:{},lastResponseReadyAt:Date.now(),requestTimings:[]};
    fs.mkdirSync(dir(job.jobId));fs.mkdirSync(path.join(dir(job.jobId),'requests'));fs.writeFileSync(path.join(dir(job.jobId),'source.png'),snapshot.data,{flag:'wx'});save(job);
    const {moduleSchema:unused,...compactContext}=context;
    return {...summary(job),source:job.source,context:compactContext,layoutSchema:schema(),workflow:'Call apply_image_layout directly with rows. It returns checked final PPTX, exact-file bound preview, regional comparison and timings. When delivery.passed and previewBoundToFinal are true, no separate crop/finalizer/render scripts are needed. Inspect returned images BEFORE finish_image. Structural checks never prove source fidelity.',imageBase64:snapshot.data.toString('base64'),mimeType:'image/png'};
  }
  async function plan(args) {
    const job=load(args.jobId);if(job.sessionId!==args.sessionId)throw Error('Image job belongs to another browser session');if(job.finishedAt)throw Error('Image job is finished');if(!RID.test(args.requestId))throw Error('Invalid requestId');
    const requestFile=path.join(dir(job.jobId),'requests',args.requestId+'.json'),fingerprint=hash(args);
    if(fs.existsSync(requestFile)){const cached=JSON.parse(fs.readFileSync(requestFile,'utf8'));if(cached.fingerprint!==fingerprint)throw Error('requestId reused with different layout arguments');return cached;}
    if(fs.readdirSync(path.dirname(requestFile)).length>=16)throw Error('Image job request journal full (16 writes); begin a new task from original');
    if(args.patches?.length&&args.expectedRevision!==job.revision)throw Error('Patch needs the current image job revision');
    const {rows,replace,authoring}=resolveRows(args,job);
    if(args.slideId&&args.slideId!==job.slideId)throw Error('Only the slide created by this image job may be repaired');
    if(job.slideId&&!args.slideId)throw Error('Use slideId for regional repair, not a second full slide');
    if(replace.length&&!args.slideId)throw Error('Region replacement requires job slideId');
    const imagePath=path.join(dir(job.jobId),'source.png'),expanded=await compileLayout(rows,job.source,job.page,async r=>{
      const data=await sharp(imagePath).extract({left:r[0],top:r[1],width:r[2],height:r[3]}).resize({width:2048,height:2048,fit:'inside',withoutEnlargement:true}).png().toBuffer();if(data.length>560000)throw Error('Photo crop too large');return data.toString('base64');
    },async r=>fitFanPixels(await sharp(imagePath).extract({left:r[0],top:r[1],width:r[2],height:r[3]}).ensureAlpha().raw().toBuffer(),r[2],r[3]),async(r,count)=>fitEncoderPixels(await sharp(imagePath).extract({left:r[0],top:r[1],width:r[2],height:r[3]}).ensureAlpha().raw().toBuffer(),r[2],r[3],count),async(r,count)=>fitTensorPixels(await sharp(imagePath).extract({left:r[0],top:r[1],width:r[2],height:r[3]}).ensureAlpha().raw().toBuffer(),r[2],r[3],count));
    // Host IDs are document-global. Reusing compact names on another slide
    // otherwise makes the host silently rename objects, invalidating repairs.
    // A full job UUID plus a stable digest also handles the longest row IDs.
    if(!job.slideId || job.idNamespace){
      const prefix=job.idNamespace || 'mcpj-'+job.jobId.replaceAll('-','')+'-';
      const remap=new Map(expanded.objects.map(o=>[o.id,prefix+hash(o.id).slice(0,20)]));
      for(const o of expanded.objects)o.id=remap.get(o.id);
      for(const ids of Object.values(expanded.regions))for(let i=0;i<ids.length;i++)ids[i]=remap.get(ids[i]);
      job.idNamespace=prefix;save(job);
    }
    for(const id of replace)if(!job.regions[id])throw Error('Unknown previously returned region '+id);
    for(const id of Object.keys(expanded.regions))if(job.regions[id]&&!replace.includes(id))throw Error('Existing region requires explicit replaceRegions');
    // Removing a region is allowed only with its observed ID; unrelated user objects never enter this list.
    const command={expectedRevision:args.expectedRevision,name:args.name||job.source.name+' — editable',slideId:args.slideId,afterSlideId:args.afterSlideId,compiledModules:[{id:'image-layout',objects:expanded.objects,replaceObjectIds:replace.flatMap(id=>job.regions[id])}],preview:true};
    if(JSON.stringify(command).length>1000000)throw Error('Expanded layout payload exceeds 1 MB');
    const validationPolicy=require('./unippt_mcp_delivery.cjs').policyFor(job.page,expanded.objects);
    // Repairs validate the whole slide, including fonts from untouched regions.
    validationPolicy.families=[...new Set([...(job.validationPolicy?.families||[]),...validationPolicy.families])].sort();
    job.validationPolicy=validationPolicy;save(job);
    const record=JSON.parse(JSON.stringify({fingerprint,command,legacyCommand:job.protocolMode==='legacy-patch'?legacyCommand(command):undefined,reviewRegions:regionRects(expanded,rows,job.source,job.page),sourceRows:rows,authoring,regions:expanded.regions,replace,fit:expanded.fit,measurements:expanded.measurements}));
    const indexDir=path.join(root,'request-index');fs.mkdirSync(indexDir,{recursive:true});const indexPath=path.join(indexDir,args.requestId+'.json');
    if(fs.existsSync(indexPath)){if(JSON.parse(fs.readFileSync(indexPath,'utf8')).jobId!==job.jobId)throw Error('requestId already belongs to another image job');}
    else fs.writeFileSync(indexPath,JSON.stringify({jobId:job.jobId}),{flag:'wx'});
    fs.writeFileSync(requestFile,JSON.stringify(record),{flag:'wx'});return record;
  }
  function committed(jobId,record,result) {
    const job=load(jobId);if(result?.writeCommitted&&result.revision>=job.revision){
      job.layoutRows??={};for(const id of record.replace)delete job.layoutRows[id];for(const row of record.sourceRows||[])job.layoutRows[row[0]]=structuredClone(row);
      for(const id of record.replace)delete job.regions[id];Object.assign(job.regions,record.regions);job.revision=result.revision;job.slideId=result.slideId||result.activeSlideId||result.slideIds?.[0];job.nativeRendered=!!result.nativeRendered;job.structuralPassed=!!result.checks?.structuralPassed;job.staleAfterRender=!!result.staleAfterRender;if(job.nativeRendered&&!job.staleAfterRender)job.firstNativeAt??=Date.now();save(job);
    }
    return {...summary(job),regions:Object.keys(record.regions),fit:record.fit,measurements:record.measurements||[],authoring:record.authoring};
  }
  async function finish(args,inspect,verifyFinal) {
    const receivedAt=Date.now();
    const job=load(args.jobId);if(job.sessionId!==args.sessionId)throw Error('Image job belongs to another session');
    if(!job.slideId)throw Error('No committed image layout to review');
    if(!args.review||typeof args.review!=='object'||typeof args.review.matchesSource!=='boolean'||!Array.isArray(args.review.unresolved)||args.review.unresolved.some(s=>typeof s!=='string'||s.length>2000)||args.review.unresolved.length>100)throw Error('Review needs matchesSource:boolean and unresolved:string[]');
    if(args.expectedRevision!==job.revision)throw Error('Review must use the latest job revision');
    const context=await inspect();if(context.revision!==job.revision)throw Error('Document changed after native preview; inspect and re-render before closing timer');
    if(!job.nativeRendered||job.staleAfterRender)throw Error('A current native preview is required before closing timer');
    if(job.finishedAt){if(hash(job.review)!==hash(args.review))throw Error('Finished review is immutable');return summary(job);}
    if(job.delivery?.passed&&verifyFinal)await verifyFinal(job.delivery);
    job.finishedAt=Date.now();job.review=args.review;save(job);return {...summary(job),revision:job.revision,slideId:job.slideId,callerReviewedMatch:args.review.matchesSource&&args.review.unresolved.length===0,independentFidelityCheck:false,
      delivery:job.delivery||null,timingBreakdown:{requests:job.requestTimings||[],droppedTimingEvents:job.droppedTimingEvents||0,lastCallerIntervalMs:job.lastResponseReadyAt?receivedAt-job.lastResponseReadyAt:null,finishCheckMs:job.finishedAt-receivedAt,scope:'Caller intervals combine model inference, output generation, orchestration and transport; they are not pure model compute.'}};
  }
  function recover(requestId,result){
    if(!RID.test(requestId))return null;const indexPath=path.join(root,'request-index',requestId+'.json');if(!fs.existsSync(indexPath))return null;
    const {jobId}=JSON.parse(fs.readFileSync(indexPath,'utf8')),job=load(jobId);
    if(result.sessionId&&result.sessionId!==job.sessionId)throw Error('Result belongs to another session');
    const record=JSON.parse(fs.readFileSync(path.join(dir(jobId),'requests',requestId+'.json'),'utf8'));
    if(record.legacyCommand&&Number.isInteger(result.revision)&&result.revision===record.legacyCommand.expectedRevision+1&&(result.activeSlideId||result.slideId))result.writeCommitted=true;
    return committed(jobId,record,result);
  }
  function previewed(args,result){
    if(!result?.nativeRendered||result.staleAfterRender||!fs.existsSync(root))return null;
    for(const id of fs.readdirSync(root).filter(n=>UUID.test(n))){let job;try{job=load(id);}catch{continue;}
      if(job.finishedAt||job.sessionId!==args.sessionId||job.slideId!==args.slideId||job.revision!==args.expectedRevision)continue;
      job.nativeRendered=true;job.staleAfterRender=false;job.firstNativeAt??=Date.now();save(job);return summary(job);
    }return null;
  }
  async function review(jobId,record,result){const job=load(jobId);return reviewRegions(sharp,path.join(dir(jobId),'source.png'),result.pngBase64,record.reviewRegions||[],{...record.fit,pageWidth:job.page.width},job.source);}
  function responded(args,record,result,receivedAt){
    const job=load(args.jobId),readyAt=Date.now();
    if(result.revision!==job.revision)throw Error('Result no longer belongs to current image revision');
    job.delivery={...result.delivery,path:result.path,sha256:result.sha256,revision:result.revision};
    const timing={requestId:args.requestId,revision:result.revision,requestReceivedAt:new Date(receivedAt).toISOString(),responseReadyAt:new Date(readyAt).toISOString(),callerIntervalMs:job.lastResponseReadyAt?Math.max(0,receivedAt-job.lastResponseReadyAt):null,...result.timings};
    // Retries consume real time too. Do not overwrite the first attempt's cost.
    const history=[...(job.requestTimings||[]),timing];job.droppedTimingEvents=(job.droppedTimingEvents||0)+Math.max(0,history.length-64);
    job.requestTimings=history.slice(-64);job.lastResponseReadyAt=readyAt;save(job);
    return {...summary(job),regions:Object.keys(record.regions),fit:record.fit,measurements:record.measurements||[],authoring:record.authoring,timing};
  }
  return {begin,plan,committed,finish,load,summary,recover,previewed,review,responded};
}
module.exports={schema,textRuns,compileLayout,createStore,sharpRuntime,fitFanPixels,fitEncoderPixels,fitTensorPixels};
