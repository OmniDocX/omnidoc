'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {create,pngInfo}=require('./mcp_image_jobs.js');
const {compileLayout}=require('./mcp_image_layout.js');
const {preparePatch}=require('./mcp_bridge.js');
const context={writeAccess:true,revision:0,document:{size:{width:1280,height:720},slides:[{id:'s'}]}};
const source={width:1536,height:697,memoryBytes:5000000,crop:async()=>'',dispose(){}};
const row=['a','text',40,80,200,25,'z_{0}',{size:22,rotate:270}];
function fixture(extra={}){let clock=1000,id=0;const runtime=create({decode:async()=>({...source}),now:()=>clock,uuid:()=>`00000000-0000-4000-8000-${String(++id).padStart(12,'0')}`,hash:async s=>crypto.createHash('sha256').update(s).digest('hex'),...extra});return {runtime,tick:ms=>clock+=ms};}
const begin=(r,requestId='begin_0001')=>r.begin({requestId,imageBase64:'fixture',name:'sample.png'},context);
const input=(job,more={})=>({jobId:job.jobId,expectedRevision:0,requestId:'layout_0001',rows:[row],...more});
const result={writeCommitted:true,revision:1,activeSlideId:'new-slide',nativeRendered:true,checks:{structuralPassed:true},audit:{sha256:'hash'},timings:{sourcePackageUnchanged:true}};
test('public image timer includes caller work, exact-package review, immutable completion',async()=>{
 const {runtime:r,tick}=fixture(),j=await begin(r);assert.equal(j.context.revision,0);assert.ok(j.layoutSchema.kinds.includes('network'));
 tick(50000);const p=await r.plan(input(j),0);assert.equal(p.command.compiledModules[0].objects[0].kind,'text');assert.equal(p.command.compiledModules[0].objects[0].textFrame.wordWrap,false);
 tick(5000);const ready=r.committed(p,result);assert.equal(ready.imageJob.firstNativeMs,55000);assert.equal(ready.delivery.previewBoundToFinal,true);
 tick(40000);const args={jobId:j.jobId,expectedRevision:1,review:{matchesSource:false,unresolved:['glyph']}};const final=r.finish(args,1);
 assert.equal(final.elapsedMs,95000);assert.equal(final.budgetPassed,true);assert.equal(final.visualFidelityPassed,false);assert.equal(final.callerReviewedMatch,false);
 tick(3000);assert.equal(r.finish(args,1).elapsedMs,95000);assert.throws(()=>r.finish({...args,review:{matchesSource:true,unresolved:[]}},1),/immutable/);
});
test('begin and layout retry are idempotent and differing content is rejected',async()=>{
 const {runtime:r}=fixture(),j=await begin(r);assert.equal((await begin(r)).jobId,j.jobId);
 await assert.rejects(r.begin({requestId:'begin_0001',imageBase64:'different'},context),/different/);
 const p=await r.plan(input(j),0);assert.strictEqual(await r.plan(input(j),0),p);
 await assert.rejects(r.plan(input(j,{rows:[['x',...row.slice(1)]]}),0),/different/);
});
test('sparse repair retains untouched rows, replaces observed IDs and namespaces new jobs',async()=>{
 const {runtime:r}=fixture(),j=await begin(r),p=await r.plan(input(j,{rows:[row,['b','box',10,10,80,80,'rect']]}),0);r.committed(p,result);
 const patch={jobId:j.jobId,slideId:'new-slide',expectedRevision:1,requestId:'layout_0002',patches:[{id:'a',options:{size:23}}]};
 const q=await r.plan(patch,1);assert.equal(q.rows[0][7].rotate,270);assert.equal(q.rows[0][7].size,23);assert.deepEqual(q.command.compiledModules[0].replaceObjectIds,p.regions.a);
 const other=await begin(r,'begin_0002'),otherPlan=await r.plan(input(other,{requestId:'other_0001'}),0);assert.notEqual(otherPlan.regions.a[0],p.regions.a[0]);
 await assert.rejects(r.plan({...patch,requestId:'layout_0003',patches:[{id:'foreign',value:'x'}]},1),/existing/);
});
test('revision, cross-job, finished and missing-preview boundaries fail closed',async()=>{
 const {runtime:r}=fixture(),j=await begin(r);await assert.rejects(r.plan(input(j),2),/revision/);
 await assert.rejects(r.plan(input(j,{slideId:'foreign'}),0),/this image/);
 const p=await r.plan(input(j),0);r.committed(p,{...result,nativeRendered:false});
 const a={jobId:j.jobId,expectedRevision:1,review:{matchesSource:true,unresolved:[]}};assert.throws(()=>r.finish(a,1),/preview/);
 r.previewed({slideId:'new-slide',expectedRevision:1},result);assert.throws(()=>r.finish(a,2),/revision/);assert.equal(r.finish(a,1).independentFidelityCheck,false);
 await assert.rejects(r.plan(input(j,{requestId:'layout_0002'}),1),/finished/);
 r.clear();assert.throws(()=>r.finish(a,1),/expired|another/);
});
test('PNG preflight refuses non-PNG, huge dimensions, animation and excessive bytes',()=>{
 const png=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jMZkAAAAASUVORK5CYII=','base64');assert.equal(pngInfo(png.toString('base64')).width,1);
 assert.throws(()=>pngInfo(Buffer.from('<svg/>').toString('base64')),/signature/);
 const bomb=Buffer.from(png);bomb.writeUInt32BE(99999,16);assert.throws(()=>pngInfo(bomb.toString('base64')),/4096/);
 const anim=Buffer.from(png);anim.write('acTL',37);assert.throws(()=>pngInfo(anim.toString('base64')),/Animated/);
 assert.throws(()=>pngInfo('a'.repeat(2800004)),/bounded/);
});
test('source quotas, decode cancellation and expiration release memory',async()=>{
 let releases=0;const {runtime:r,tick}=fixture({decode:async()=>({...source,dispose:()=>releases++})});
 for(let i=0;i<4;i++)await begin(r,'begin_000'+i);await assert.rejects(begin(r,'begin_0005'),/limit/);
 tick(3600001);await begin(r,'begin_0006');assert.equal(releases,4);r.clear();assert.equal(releases,5);
 let complete;const delayed=fixture({decode:()=>new Promise(resolve=>complete=resolve)}).runtime;const waiting=begin(delayed);await new Promise(resolve=>setImmediate(resolve));delayed.clear();complete({...source,dispose:()=>releases++});await assert.rejects(waiting,/disconnected/);
});
test('over-budget timer never passes and unresolved review is not accepted as a match',async()=>{
 const {runtime:r,tick}=fixture(),j=await begin(r),p=await r.plan(input(j),0);r.committed(p,result);tick(120001);
 const x=r.finish({jobId:j.jobId,expectedRevision:1,review:{matchesSource:true,unresolved:['unfixed']}},1);assert.equal(x.budgetPassed,false);assert.equal(x.callerReviewedMatch,false);
});
test('compact rotated text retains no-wrap through actual native bridge normalization',async()=>{
 const expanded=await compileLayout([row],source,{width:1280,height:720});const ops=preparePatch({slides:[]},[{op:'insertSlide',slide:{objects:expanded.objects}}]);
 assert.equal(ops[0].slide.objects[0].textFrame.wordWrap,false);assert.equal(ops[0].slide.objects[0].frame.rotation,270);assert.equal(ops[0].slide.objects[0].textParagraphs[0].runs[1].baseline,'sub');
});

test('older writes retain replay identity without retaining full binary packages',async()=>{
 const {runtime:r}=fixture(),j=await begin(r),p=await r.plan(input(j),0);r.committed(p,{...result,pptxBase64:'package',pngBase64:'preview'});
 const q=await r.plan({jobId:j.jobId,slideId:'new-slide',expectedRevision:1,requestId:'layout_0002',patches:[{id:'a',value:'new'}]},1);r.committed(q,{...result,revision:2,pptxBase64:'new-package',pngBase64:'new-preview'});
 assert.equal(p.result.pptxBase64,undefined);assert.equal(p.result.binaryReplayUnavailable,true);assert.equal(q.result.pptxBase64,'new-package');assert.strictEqual(await r.plan(input(j),2),p);
});

test('browser image modules remain functional when parallel downloads execute in reverse order',async()=>{
 const vm=require('node:vm'),fs=require('node:fs'),path=require('node:path');
 const context={console,performance,TextEncoder,Uint8Array,Uint8ClampedArray,Float64Array,crypto:crypto.webcrypto,structuredClone};vm.createContext(context);
 for(const file of ['mcp_bridge.js','mcp_image_jobs.js','mcp_image_layout.js','mcp_row_patch.js','mcp_path_dash.js','image_semantic_module_runtime.js','mcp_scene.js'])vm.runInContext(fs.readFileSync(path.join(__dirname,file),'utf8'),context);
 const runtime=context.UniPptMcpImageJobs.create({decode:async()=>({...source})});const job=await begin(runtime);
 const args=JSON.stringify(input(job,{rows:[row,['border','box',10,10,200,80,'roundRect',{radius:8,dashPattern:[6,4],fill:'transparent',stroke:'#808080'}]]}));
 const record=await runtime.plan(vm.runInContext('('+args+')',context),0);assert.ok(record.command.compiledModules[0].objects.length>=2);
 assert.equal(record.command.compiledModules[0].objects[0].textFrame.wordWrap,false);
 const ops=context.UniPptMcpBridge.preparePatch({slides:[]},[{op:'insertSlide',slide:{objects:record.command.compiledModules[0].objects}}]);assert.ok(ops.length);
});
