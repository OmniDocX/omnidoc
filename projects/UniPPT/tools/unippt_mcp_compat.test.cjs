'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const {prepareContext,legacyCommand,applyLegacy}=require('./unippt_mcp_compat.cjs');
const {compileLayout,sharpRuntime}=require('./unippt_mcp_image.cjs');
const {reviewRegions,regionRects}=require('./unippt_mcp_review.cjs');
test('only an explicit unknown read command enables compatibility, never consent/timeout failure',async()=>{
  const methods=[];
  const execute=async method=>{methods.push(method);return {state:'completed',...(method==='prepare'?{error:'未知 MCP 命令'}:{result:method==='inspect'?{revision:3,writeAccess:true,document:{size:{width:100,height:100}}}:{allowedOperations:['insertSlide','addObject','updateObject','removeObject']}})};};
  assert.equal((await prepareContext(execute,'s')).protocolMode,'legacy-patch');assert.deepEqual(methods,['prepare','inspect','get_edit_schema']);
  for(const failure of [{state:'claimed'},{state:'completed',error:'Consent revoked'},{state:'completed',error:'REVISION_CONFLICT'}]){
    let n=0;await assert.rejects(prepareContext(async()=>{n++;return failure;},'s'));assert.equal(n,1);
  }
});
test('legacy authoring normalizes native runs, preserves IDs, and limits atomic region updates',async()=>{
  const compiled=await compileLayout([['formula','text',0,0,100,25,[{text:'F'},{text:'x',baseline:'sub',baselineOffset:-11}]]],{width:100,height:100},{width:100,height:100});
  const mod={id:'r',objects:compiled.objects,replaceObjectIds:[]},command={expectedRevision:3,compiledModules:[mod]};
  const insert=legacyCommand(command);assert.equal(insert.operations.length,1);assert.equal(insert.operations[0].slide.objects[0].textParagraphs[0].runs[1].baselineOffset,-11);
  mod.replaceObjectIds=[compiled.objects[0].id];const update=legacyCommand({...command,slideId:'s'});assert.equal(update.operations[0].op,'updateObject');assert.equal(update.operations[0].patch.id,undefined);
  assert.throws(()=>legacyCommand(command),/existing slide/);
  assert.throws(()=>legacyCommand({...command,slideId:'s',compiledModules:[{...mod,replaceObjectIds:Array.from({length:101},(_,i)=>'x'+i),objects:[]}]}));
});
test('one compatibility bundle preserves committed writes even when preview fails; child IDs are stable',async()=>{
  const calls=[],commits=[];const execute=async(method,session,args,id)=>{calls.push({method,args,id});return {state:'completed',requestId:id,...(method==='preview'?{error:'Renderer offline'}:{result:method==='apply_patch'?{revision:4,activeSlideId:'added'}:{pptxBase64:'UEs='}})};};
  const record={legacyCommand:{expectedRevision:3,operations:[]}};
  const result=await applyLegacy(execute,'s','original-write',record,r=>commits.push({...r}));assert.equal(commits.length,1);assert.equal(result.result.writeCommitted,true);assert.equal(result.result.nativeRendered,false);assert.equal(result.result.pptxBase64,'UEs=');assert.equal(result.result.revision,4);
  assert.equal(calls[0].id,'original-write');assert.equal(calls[1].args.expectedRevision,4);
  const first=calls.map(c=>c.id);await applyLegacy(execute,'s','original-write',record,()=>{});assert.deepEqual(calls.slice(3).map(c=>c.id),first);
  let n=0;const uncertain=await applyLegacy(async()=>{n++;return {state:'claimed'};},'s','uncertain',record,()=>assert.fail());assert.equal(n,1);assert.equal(uncertain.state,'claimed');
  const broken=await applyLegacy(async method=>{if(method==='apply_patch')return {state:'completed',result:{revision:4,activeSlideId:'added'}};throw Error('Session expired');},'s','disconnected',record,()=>{throw Error('Disk full');});
  assert.equal(broken.result.writeCommitted,true);assert.equal(broken.result.imageJournalError,'Disk full');assert.equal(broken.result.nativeRendered,false);assert.equal(broken.result.previewError,'Session expired');assert.equal(broken.result.exportError,'Session expired');
});
test('regional preview aligns letterboxed native output, ranks change, and never claims fidelity',async()=>{
  const sharp=sharpRuntime(),rows=[['a','text',10,10,20,15,'A'],['b','text',65,10,20,15,'B']],source={width:100,height:50},page={width:100,height:100};
  const compiled=await compileLayout(rows,source,page),regions=regionRects(compiled,rows,source,page);
  const src=await sharp({create:{width:100,height:50,channels:3,background:'#fff'}}).composite([{input:await sharp({create:{width:10,height:10,channels:3,background:'#000'}}).png().toBuffer(),left:12,top:12}]).png().toBuffer();
  const native=await sharp({create:{width:100,height:100,channels:3,background:'#fff'}}).composite([{input:src,left:0,top:25},{input:await sharp({create:{width:10,height:10,channels:3,background:'#f00'}}).png().toBuffer(),left:67,top:37}]).png().toBuffer();
  const result=await reviewRegions(sharp,src,native.toString('base64'),regions,{...compiled.fit,pageWidth:100},source);assert.equal(result.regionalReview.regions[0].id,'b');assert.equal(result.regionalReview.visualFidelityPassed,false);assert.ok(result.reviewPngBase64);assert.equal(result.regionalReview.regions.find(r=>r.id==='a').foregroundMeanAbsoluteRgb,0);
});
