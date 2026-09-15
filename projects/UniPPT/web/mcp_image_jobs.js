(function(global){
  'use strict';
  const node=typeof module!=='undefined'&&module.exports;
  const digest=async value=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(value))),b=>b.toString(16).padStart(2,'0')).join('');
  const validId=id=>typeof id==='string'&&/^[\w-]{8,100}$/.test(id);
  const canonical=v=>JSON.stringify(v,(_,x)=>x&&typeof x==='object'&&!Array.isArray(x)?Object.fromEntries(Object.keys(x).sort().map(k=>[k,x[k]])):x);
  function pngInfo(base64){
    if(typeof base64!=='string'||base64.length>2800000||base64.length%4||!/^[A-Za-z0-9+/]+={0,2}$/.test(base64))throw Error('Source must be bounded PNG base64 (max 2 MiB)');
    const bytes=Uint8Array.from(atob(base64),c=>c.charCodeAt(0));
    const sig=[137,80,78,71,13,10,26,10];if(bytes.length<45||sig.some((b,i)=>bytes[i]!==b))throw Error('PNG signature required');
    const dv=new DataView(bytes.buffer),width=dv.getUint32(16),height=dv.getUint32(20);
    if(dv.getUint32(8)!==13||String.fromCharCode(...bytes.slice(12,16))!=='IHDR'||!width||!height||width>4096||height>4096||width*height>4000000)throw Error('PNG exceeds 4096 per edge / 4M pixels');
    let at=8,chunks=0,ended=false;
    while(at+12<=bytes.length){const n=dv.getUint32(at),type=String.fromCharCode(...bytes.slice(at+4,at+8));if(++chunks>4096||at+n+12>bytes.length)throw Error('Invalid PNG chunks');if(type==='acTL')throw Error('Animated PNG is unsupported');at+=n+12;if(type==='IEND'){ended=true;break;}}
    if(!ended||at!==bytes.length)throw Error('Invalid PNG end');return {bytes,width,height};
  }
  async function decodePng(base64){
    const info=pngInfo(base64),bitmap=await createImageBitmap(new Blob([info.bytes],{type:'image/png'}));
    if(bitmap.width!==info.width||bitmap.height!==info.height){bitmap.close();throw Error('PNG dimensions mismatch');}
    const canvas=document.createElement('canvas');canvas.width=info.width;canvas.height=info.height;canvas.getContext('2d').drawImage(bitmap,0,0);bitmap.close();
    const roi=r=>{const c=document.createElement('canvas');c.width=r[2];c.height=r[3];c.getContext('2d').drawImage(canvas,...r,0,0,r[2],r[3]);return c;};
    return {...info,bytes:undefined,memoryBytes:info.width*info.height*4+base64.length,
      crop:async r=>{const c=roi(r),out=c.toDataURL('image/png').split(',')[1];c.width=c.height=0;if(out.length>750000)throw Error('Photo crop exceeds 750000 base64 characters');return out;},
      pixels:async r=>canvas.getContext('2d',{willReadFrequently:true}).getImageData(...r).data,
      dispose:()=>{canvas.width=canvas.height=0;}};
  }
  function create({decode=decodePng,now=Date.now,uuid=()=>crypto.randomUUID(),hash=digest}={}){
    const layout=node?require('./mcp_image_layout.js'):global.UniPptMcpImageLayout;
    const {resolveRows}=node?require('./mcp_row_patch.js'):global.UniPptMcpRowPatch;
    const jobs=new Map(),begins=new Map();let epoch=0;
    const drop=j=>{j.source.dispose?.();jobs.delete(j.id);};
    const sweep=()=>{for(const j of jobs.values())if(now()-j.startedAt>3600000)drop(j);for(const [r,b]of begins)if(!jobs.has(b.id))begins.delete(r);};
    const load=id=>{sweep();const j=jobs.get(id);if(!j)throw Error('Image job expired or belongs to another document connection');return j;};
    const summary=j=>({jobId:j.id,revision:j.revision,slideId:j.slideId,startedAt:new Date(j.startedAt).toISOString(),elapsedMs:(j.finishedAt||now())-j.startedAt,firstNativeMs:j.firstNativeAt?j.firstNativeAt-j.startedAt:null,budgetMs:120000,budgetPassed:j.finishedAt?j.finishedAt-j.startedAt<=120000:null,modelInclusive:true,clockScope:'Public begin_image received through finish_image; includes model description, review and all waits; excludes client upload preparation before receipt.',visualFidelityPassed:false});
    async function begin(args,context){
      if(!validId(args.requestId)||!context.writeAccess)throw Error('Image begin requires a stable requestId and current document write grant');
      const startedAt=now(),fingerprint=await hash(canonical(args));sweep();
      if(begins.has(args.requestId)){const b=begins.get(args.requestId);if(b.fingerprint!==fingerprint)throw Error('requestId reused with different source');return beginResult(load(b.id),context);}
      if(jobs.size>=4)throw Error('Image job limit reached (4 per document connection)');
      const capturedEpoch=epoch,source=await decode(args.imageBase64);
      if(epoch!==capturedEpoch){source.dispose?.();throw Error('MCP disconnected during image decode');}
      if(jobs.size>=4||source.memoryBytes+[...jobs.values()].reduce((n,j)=>n+j.source.memoryBytes,0)>64000000){source.dispose?.();throw Error('Image source memory quota exceeded');}
      const j={id:uuid(),startedAt,source,sourceBase64:args.imageBase64,name:String(args.name||'source.png').slice(0,120),page:context.document.size,revision:context.revision,context,regions:{},layoutRows:{},requests:new Map(),firstNativeAt:null};
      jobs.set(j.id,j);begins.set(args.requestId,{id:j.id,fingerprint});return beginResult(j,context);
    }
    function beginResult(j,context){return {...summary(j),source:{name:j.name,width:j.source.width,height:j.source.height},context:{...context,moduleSchema:undefined},layoutSchema:layout.schema(),imageBase64:j.sourceBase64,mimeType:'image/png',workflow:'Submit compact rows with apply_image_layout. Native editable PPTX, exact-file PNG and XML checks return together. Inspect the original and preview, repair only named regions with patches, then finish_image. No OCR/model calls or automatic visual acceptance.'};}
    async function plan(args,currentRevision){
      const j=load(args.jobId);if(j.finishedAt)throw Error('Image job is finished');if(!validId(args.requestId))throw Error('Stable requestId required');
      const fingerprint=await hash(canonical(args));
      if(j.requests.has(args.requestId)){const r=j.requests.get(args.requestId);if(r.fingerprint!==fingerprint)throw Error('requestId reused with different layout');return r;}
      if(j.requests.size>=16)throw Error('Image job write limit reached');
      if(args.expectedRevision!==currentRevision||args.expectedRevision!==j.revision)throw Error('Image layout needs current document and job revision');
      if(j.slideId&&args.slideId!==j.slideId||args.slideId&&!j.slideId)throw Error('Use only this image job slide for repair');
      const {rows,replace,authoring}=resolveRows(args,j);
      if(replace.length&&!j.slideId)throw Error('Replacement requires a committed slide');
      for(const id of replace)if(!Object.hasOwn(j.regions,id))throw Error('Unknown replacement region');
      const src=j.source,start=performance.now();
      const expanded=await layout.compileLayout(rows,src,j.page,src.crop,async r=>layout.fitFanPixels(await src.pixels(r),r[2],r[3]),async(r,n)=>layout.fitEncoderPixels(await src.pixels(r),r[2],r[3],n),async(r,n)=>layout.fitTensorPixels(await src.pixels(r),r[2],r[3],n));
      for(const id of Object.keys(expanded.regions))if(Object.hasOwn(j.regions,id)&&!replace.includes(id))throw Error('Existing region requires explicit replacement');
      const ids=new Map(await Promise.all(expanded.objects.map(async o=>[o.id,'mcpj-'+j.id.replaceAll('-','')+'-'+(await hash(o.id)).slice(0,20)])));
      for(const o of expanded.objects)o.id=ids.get(o.id);for(const region of Object.values(expanded.regions))for(let i=0;i<region.length;i++)region[i]=ids.get(region[i]);
      const command={expectedRevision:args.expectedRevision,slideId:args.slideId,afterSlideId:args.afterSlideId,name:args.name||j.name+' — editable',preview:true,compileMs:performance.now()-start,compiledModules:[{id:'image-layout',objects:expanded.objects,replaceObjectIds:replace.flatMap(id=>j.regions[id])}]};
      if(JSON.stringify(command).length>1000000)throw Error('Expanded layout exceeds 1 MB');
      const record={jobId:j.id,fingerprint,rows,replace,authoring,regions:expanded.regions,fit:expanded.fit,measurements:expanded.measurements,command};j.requests.set(args.requestId,record);return record;
    }
    function committed(record,result){
      // The broker owns bounded exact-response replay. Retain only the latest
      // browser binary payload, not 16 full PPTX/PNG packages per job.
      for(const job of jobs.values())for(const previous of job.requests.values())if(previous!==record&&previous.result){
        if(previous.result.pptxBase64||previous.result.pngBase64){delete previous.result.pptxBase64;delete previous.result.pngBase64;delete previous.result.reviewPngBase64;previous.result.binaryReplayUnavailable=true;previous.result.replayInstruction='Query command_status for the original response; never repeat the write.';}
      }
      const j=load(record.jobId);if(result.writeCommitted){
        for(const id of record.replace){delete j.regions[id];delete j.layoutRows[id];}
        Object.assign(j.regions,record.regions);for(const row of record.rows)j.layoutRows[row[0]]=structuredClone(row);
        j.revision=result.revision;j.slideId=result.activeSlideId||result.slideId;
        j.nativeRendered=!!result.nativeRendered&&!result.staleAfterRender;j.structuralPassed=!!result.checks?.structuralPassed;
        j.sha256=result.audit?.sha256;j.delivery={verificationLevel:'native-xml-and-exact-package-render',passed:j.nativeRendered&&j.structuralPassed,previewBoundToFinal:j.nativeRendered&&result.timings?.sourcePackageUnchanged===true,sha256:j.sha256,visualFidelityPassed:false};
        if(j.nativeRendered)j.firstNativeAt??=now();
      }
      const out={...result,delivery:j.delivery,imageJob:{...summary(j),regions:Object.keys(record.regions),fit:record.fit,measurements:record.measurements,authoring:record.authoring}};delete out.moduleMap;record.result=out;return out;
    }
    function previewed(args,result){for(const j of jobs.values())if(j.slideId===args.slideId&&j.revision===args.expectedRevision&&!j.finishedAt&&result.nativeRendered){j.nativeRendered=true;j.sha256=result.audit?.sha256;j.firstNativeAt??=now();j.delivery={...j.delivery,passed:!!j.structuralPassed,previewBoundToFinal:result.timings?.sourcePackageUnchanged===true,sha256:j.sha256,visualFidelityPassed:false};}}
    function finish(args,currentRevision){
      const j=load(args.jobId),r=args.review;
      if(!r||typeof r.matchesSource!=='boolean'||!Array.isArray(r.unresolved)||r.unresolved.length>100||r.unresolved.some(s=>typeof s!=='string'||s.length>2000))throw Error('Explicit visual review required');
      if(args.expectedRevision!==j.revision||currentRevision!==j.revision||!j.nativeRendered||!j.sha256)throw Error('Finish requires current native preview and document revision');
      if(j.finishedAt&&canonical(j.review)!==canonical(r))throw Error('Finished review is immutable');
      j.finishedAt??=now();j.review=structuredClone(r);return {...summary(j),review:r,callerReviewedMatch:r.matchesSource&&r.unresolved.length===0,independentFidelityCheck:false,delivery:j.delivery};
    }
    return {begin,plan,committed,previewed,finish,clear(){epoch++;for(const j of jobs.values())j.source.dispose?.();jobs.clear();begins.clear();}};
  }
  global.UniPptMcpImageJobs={create,pngInfo};if(node)module.exports={create,pngInfo};
})(globalThis);
