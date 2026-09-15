'use strict';
// Internal, model-neutral exact-file verification. No public path/shell inputs.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const {execFile}=require('node:child_process');
const {ROOT}=require('./unippt_mcp_broker.cjs');
const sha=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const RID=/^[\w-]{8,100}$/;
function runtime(){
  const quality=JSON.parse(fs.readFileSync(path.join(ROOT,'.unippt-quality-runtime.json'),'utf8'));
  const skillDir=process.env.UNIPPT_PRESENTATION_SKILL_DIR||quality.skillDir;
  const pythonExecutable=process.env.UNIPPT_PYTHON||quality.pythonExecutable;
  if(!skillDir||!path.isAbsolute(skillDir))throw Error('Configure an absolute UNIPPT_PRESENTATION_SKILL_DIR or quality runtime skillDir');
  if(!pythonExecutable)throw Error('Configure UNIPPT_PYTHON or quality runtime pythonExecutable');
  return {...quality,skillDir,pythonExecutable};
}
function policyFor(page,objects){
  if(!page||![page.width,page.height].every(n=>Number.isFinite(n)&&n>0&&n<=8192))throw Error('Invalid validation canvas');
  const families=new Set();
  for(const o of objects){if(o.textStyle?.fontFamily)families.add(o.textStyle.fontFamily);for(const run of o.textRuns||[])if(run.fontFamily)families.add(run.fontFamily);}
  return {slideSizeEmu:[Math.round(page.width*9525),Math.round(page.height*9525)],families:[...families].sort(),slideCount:1};
}
function fingerprint(config=runtime()){
  const dir=path.join(config.skillDir,'container_tools'),parts=[JSON.stringify(config),'mcp-exact-delivery-v1'];
  const walk=base=>{for(const e of fs.readdirSync(base,{withFileTypes:true}).sort((a,b)=>a.name.localeCompare(b.name))){const p=path.join(base,e.name);if(e.isDirectory()&&e.name!=='__pycache__')walk(p);else if(e.isFile()&&/\.(?:py|mjs|js|json)$/.test(e.name))parts.push(path.relative(dir,p)+':'+sha(fs.readFileSync(p)));}};
  walk(dir);
  for(const p of [__filename,path.join(__dirname,'validate_mcp_delivery.mjs'),path.join(config.nodeModules,'@oai/artifact-tool/package.json')])parts.push(sha(fs.readFileSync(p)));
  return sha(parts.join('\n'));
}
function validate(job,config){
  return new Promise((resolve,reject)=>{
    execFile(config.nodeExecutable,[path.join(__dirname,'validate_mcp_delivery.mjs'),job],{cwd:ROOT,windowsHide:true,timeout:35000,maxBuffer:512*1024,env:{...process.env,RUNTIME_NODE_MODULES:config.nodeModules}},(error,stdout,stderr)=>{
      if(error)return reject(Error('Exact-file validation failed: '+String(stderr||stdout||error.message).slice(-1200)));
      resolve();
    });
  });
}
function compactReceipt(receipt,policy){
  return {packagePassed:receipt.packageIntegrity?.status==='pass',layoutPassed:receipt.presentationLayout?.finding_count===0,importPassed:receipt.firstPartyImport?.passed===true,
    fontsPassed:!policy?.families?.length||receipt.fontSelection?.passed===true,tablesPassed:receipt.nativeTableArithmetic?.passed===true,chartTitlesPassed:receipt.nativeChartTitles?.passed===true,
    chartsPassed:receipt.nativeChartValidation?.passed===true,warnings:(receipt.presentationLayout?.warnings||[]).slice(0,20)};
}
function createDelivery({root=path.join(ROOT,'.unippt-mcp','delivery-cache'),output=path.join(ROOT,'outputs','mcp'),configuration=runtime,validator=validate,version=fingerprint}={}){
  const pending=new Map();
  async function deliver(result,requestId,policy){
    const started=performance.now();
    if(!RID.test(requestId))throw Error('Invalid delivery requestId');
    const bytes=Buffer.from(result.pptxBase64||'','base64'),png=Buffer.from(result.pngBase64||'','base64');
    if(bytes.length>20*1024*1024||bytes.length<4||bytes[0]!==80||bytes[1]!==75)throw Error('Invalid delivery PPTX');
    const digest=sha(bytes);
    if(!result.nativeRendered||result.staleAfterRender||result.audit?.sha256!==digest||!png.subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10])))throw Error('Preview is not bound to the exact exported PPTX; request a current native preview');
    const config=configuration(),versionId=version(config),key=sha(JSON.stringify({digest,versionId,policy}));
    const cacheDir=path.join(root,key),manifest=path.join(cacheDir,'verified.json');
    const readVerified=()=>{
      if(!fs.existsSync(manifest))return null;
      const data=JSON.parse(fs.readFileSync(manifest,'utf8'));
      const final=fs.readFileSync(path.join(cacheDir,'validated','final.pptx')),receipt=fs.readFileSync(path.join(cacheDir,'receipt.json'));
      if(data.sha256!==digest||data.version!==versionId||sha(final)!==digest||sha(receipt)!==data.receiptSha256)throw Error('Verification cache was changed; refusing stale validation');
      return data;
    };
    let cached=readVerified(),cacheHit=!!cached;
    if(!cached){
      if(!pending.has(key))pending.set(key,(async()=>{
        fs.mkdirSync(root,{recursive:true});
        if(fs.readdirSync(root).length>=128)throw Error('Verification cache full; retain evidence and clear explicitly before continuing');
        const attempt=fs.mkdtempSync(path.join(root,'pending-')),candidatePath=path.join(attempt,'candidate.pptx'),finalPath=path.join(attempt,'validated','final.pptx'),receiptPath=path.join(attempt,'receipt.json'),jobPath=path.join(attempt,'job.json');
        fs.mkdirSync(path.dirname(finalPath));
        fs.writeFileSync(candidatePath,bytes,{flag:'wx'});
        fs.writeFileSync(jobPath,JSON.stringify({workspaceDir:ROOT,candidatePath,finalPath,receiptPath,policy,config}),{flag:'wx'});
        await validator(jobPath,config);
        const final=fs.readFileSync(finalPath),receiptBytes=fs.readFileSync(receiptPath),receipt=JSON.parse(receiptBytes),checks=compactReceipt(receipt,policy);
        if(sha(final)!==digest||receipt.finalSha256!==digest||Object.entries(checks).some(([k,v])=>k!=='warnings'&&v!==true))throw Error('Finalizer did not preserve exact bytes or required checks');
        const data={sha256:digest,version:versionId,receiptSha256:sha(receiptBytes),checks};
        fs.writeFileSync(path.join(attempt,'verified.json'),JSON.stringify(data),{flag:'wx'});
        // Different adapters may validate the same content concurrently. Never
        // overwrite another completed result; verify it before reusing it.
        try{fs.renameSync(attempt,cacheDir);}catch(error){if(!fs.existsSync(manifest))throw error;}
      })().finally(()=>pending.delete(key)));
      await pending.get(key);cached=readVerified();
    }
    fs.mkdirSync(output,{recursive:true});const destination=path.join(output,requestId+'-checked.pptx');
    try{fs.copyFileSync(path.join(cacheDir,'validated','final.pptx'),destination,fs.constants.COPYFILE_EXCL);}catch(error){if(error.code!=='EEXIST'||sha(fs.readFileSync(destination))!==digest)throw error;}
    if(sha(fs.readFileSync(destination))!==digest)throw Error('Final output changed during delivery');
    return {path:destination,bytes:bytes.length,sha256:digest,delivery:{passed:true,cacheHit,checks:cached.checks,previewBoundToFinal:true,previewSha256:sha(png),policyFingerprint:key,validatorVersion:versionId,receiptPath:path.join(cacheDir,'receipt.json'),visualFidelityPassed:false},timings:{...result.timings,verificationMs:performance.now()-started}};
  }
  function verifyFinal(delivery){
    if(!delivery?.path||!delivery.sha256)throw Error('Missing verified final artifact');
    const target=path.resolve(delivery.path);
    if(path.dirname(target)!==path.resolve(output)||!target.endsWith('-checked.pptx')||sha(fs.readFileSync(target))!==delivery.sha256)throw Error('Final file changed after preview; revalidate before finish');
    if(delivery.validatorVersion!==version(configuration()))throw Error('Validator changed after preview; revalidate before finish');
  }
  return {deliver,verifyFinal};
}
module.exports={createDelivery,policyFor,compactReceipt,runtime,fingerprint};
