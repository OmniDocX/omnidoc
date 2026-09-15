'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto');
const {createDelivery,policyFor}=require('./unippt_mcp_delivery.cjs');
const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
const bytes=Buffer.from('PK-native-unit-fixture'),png=Buffer.from([137,80,78,71,13,10,26,10,0]);
const result=()=>({nativeRendered:true,writeCommitted:true,pptxBase64:bytes.toString('base64'),pngBase64:png.toString('base64'),audit:{sha256:hash(bytes)}});
const policy={slideSizeEmu:[12192000,6858000],families:['Times New Roman'],slideCount:1};
function setup(t,transform=x=>x){
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'mcp-delivery-test-'));
  t.after(()=>{assert.ok(path.dirname(root)===os.tmpdir()&&path.basename(root).startsWith('mcp-delivery-test-'));fs.rmSync(root,{recursive:true,force:true});});
  let calls=0,version='v1';
  const api=createDelivery({root:path.join(root,'cache'),output:path.join(root,'output'),configuration:()=>({}),version:()=>version,validator:async file=>{
    calls++;const j=JSON.parse(fs.readFileSync(file,'utf8'));assert.notEqual(path.dirname(j.finalPath),path.dirname(j.receiptPath),'Private receipt cannot be inside the output directory');const candidate=fs.readFileSync(j.candidatePath);fs.writeFileSync(j.finalPath,candidate,{flag:'wx'});
    const receipt=transform({finalSha256:hash(candidate),packageIntegrity:{status:'pass'},presentationLayout:{finding_count:0,warnings:[]},firstPartyImport:{passed:true},fontSelection:{passed:true},nativeTableArithmetic:{passed:true},nativeChartTitles:{passed:true},nativeChartValidation:{passed:true}});
    fs.writeFileSync(j.receiptPath,JSON.stringify(receipt),{flag:'wx'});
  }});
  return {api,root,count:()=>calls,setVersion:v=>version=v};
}
test('exact preview and final hashes stay bound; cache avoids revalidation but never certifies visual fidelity',async t=>{
  const s=setup(t),a=await s.api.deliver(result(),'delivery-test-01',policy),b=await s.api.deliver(result(),'delivery-test-02',policy);
  assert.equal(s.count(),1);assert.equal(a.delivery.cacheHit,false);assert.equal(b.delivery.cacheHit,true);assert.equal(b.delivery.visualFidelityPassed,false);assert.equal(b.sha256,hash(bytes));assert.equal(b.delivery.previewSha256,hash(png));
  s.api.verifyFinal({...b.delivery,path:b.path,sha256:b.sha256});assert.deepEqual(fs.readFileSync(b.path),bytes);
});
test('same-content concurrent verification shares work',async t=>{
  const s=setup(t);await Promise.all(['concurrent-test-1','concurrent-test-2'].map(id=>s.api.deliver(result(),id,policy)));assert.equal(s.count(),1);
});
test('different validator or policy invalidates verification cache',async t=>{
  const s=setup(t),a=await s.api.deliver(result(),'version-test-1',policy);s.setVersion('v2');assert.throws(()=>s.api.verifyFinal({...a.delivery,path:a.path,sha256:a.sha256}),/Validator changed/);
  await s.api.deliver(result(),'version-test-2',policy);await s.api.deliver(result(),'version-test-3',{...policy,families:['Cambria Math']});assert.equal(s.count(),3);
});
test('changed output or cached receipt cannot inherit a pass and existing output cannot be overwritten',async t=>{
  const s=setup(t),a=await s.api.deliver(result(),'integrity-test-1',policy);
  fs.writeFileSync(a.path,'unrelated user changes');assert.throws(()=>s.api.verifyFinal({...a.delivery,path:a.path,sha256:a.sha256}),/changed/);
  await assert.rejects(s.api.deliver(result(),'integrity-test-1',policy));assert.equal(fs.readFileSync(a.path,'utf8'),'unrelated user changes');
  fs.writeFileSync(a.delivery.receiptPath,'{}');await assert.rejects(s.api.deliver(result(),'integrity-test-2',policy),/cache was changed/);
});
test('missing, mismatched, stale or invalid preview is rejected before validation',async t=>{
  const s=setup(t);
  for(const patch of [{nativeRendered:false},{staleAfterRender:true},{audit:{}},{audit:{sha256:'wrong'}},{pngBase64:'bad'},{pptxBase64:'bad'}])await assert.rejects(s.api.deliver({...result(),...patch},'preview-test-01',policy));
  await assert.rejects(s.api.deliver(result(),'../outside',policy));assert.equal(s.count(),0);
});
test('required checks failing do not become a cached pass',async t=>{
  const s=setup(t,r=>({...r,firstPartyImport:{passed:false}}));await assert.rejects(s.api.deliver(result(),'failed-test-01',policy),/required checks/);assert.equal(s.count(),1);
});
test('validation policy uses intended canvas and authored run fonts',()=>{
  const p=policyFor({width:1280,height:720},[{textStyle:{fontFamily:'Times New Roman'},textRuns:[{text:'F',fontFamily:'Cambria Math'}]}]);assert.deepEqual(p,{...policy,families:['Cambria Math','Times New Roman']});assert.throws(()=>policyFor({width:NaN,height:720},[]));
});
test('broker replay retains historical execution timings outside current-call totals',()=>{
  const {reusedResult}=require('./unippt_mcp.cjs'),old={state:'completed',result:{timings:{renderMs:6000},writeCommitted:true,revision:2}};
  const next=reusedResult(old);assert.equal(next.result.cachedExecutionTimings.renderMs,6000);assert.deepEqual(next.result.timings,{});assert.equal(next.result.executionCacheHit,true);assert.equal(old.result.timings.renderMs,6000);
  assert.deepEqual(reusedResult({state:'pending'}),{state:'pending'});
});
