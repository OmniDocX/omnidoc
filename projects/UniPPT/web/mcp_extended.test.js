'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const {TOOLS}=require('../tools/unippt_mcp.cjs');
const {capabilities}=require('./mcp_bridge.js');
test('all advertised local tools and generated documentation stay synchronized',()=>{
 const declared=JSON.parse(fs.readFileSync(path.join(__dirname,'mcp-tools.json'),'utf8'));
 assert.deepEqual(declared,TOOLS);assert.equal(declared.length,22);
 assert.deepEqual(JSON.parse(fs.readFileSync(path.join(__dirname,'mcp-tools.json'),'utf8')),declared);
 const caps=capabilities();assert.equal(caps.exportFormats.length,7);assert.equal(caps.operations.length,14);assert.equal(caps.modelCalls,false);assert.equal(caps.boundaries.length,2);
 const html=fs.readFileSync(path.join(__dirname,'mcp-protocol.html'),'utf8');assert.doesNotMatch(html,/<script/i);assert.match(html,/expectedRevision/);
});
test('new commands stay revision-bound and export only fixed routes without mutation',async()=>{
 const timers=[],calls=[],results=[],events={};let command=null;
 const deck={title:'isolated',width:1000,height:600,slides:[{id:'s',objects:[]}]};
 const context={console,URL,AbortSignal,performance,structuredClone,Blob,atob,btoa,location:{origin:'http://127.0.0.1:8141'},
   document:{visibilityState:'visible',addEventListener:(n,f)=>events[n]=f},setTimeout:fn=>{timers.push(fn);return timers.length;},clearTimeout(){}};
 context.FileReader=class {readAsDataURL(blob){blob.arrayBuffer().then(b=>{this.result='data:application/octet-stream;base64,'+Buffer.from(b).toString('base64');this.onload();});}};
 context.fetch=async(url,options)=>{const body=JSON.parse(options.body);calls.push(url);
   if(url.endsWith('/connect'))return {ok:true,json:async()=>({sessionId:'test',secret:'synthetic',writeAccess:true})};
   if(url.endsWith('/poll')){const c=command;command=null;return {ok:true,json:async()=>({command:c})};}
   if(url.endsWith('/result')){results.push(body);return {ok:true,json:async()=>({ok:true})};}
   if(url.startsWith('/api/export-')){assert.equal(body.deck.slides[0].id,'s');return {ok:true,blob:async()=>new Blob(['isolated bytes'])};}
   return {ok:true,json:async()=>({})};};
 vm.createContext(context);for(const file of ['presentation_host.js','mcp_scene.js','mcp_animation.js','mcp_bridge.js'])vm.runInContext(fs.readFileSync(path.join(__dirname,file),'utf8'),context);
 const bridge=context.UniPptMcpBridge.create({host:{on(){},selection:{get:()=>({slideId:'s'})},document:{validate:()=>[],getSummary:()=>({slideCount:1})}},getDeck:()=>deck});
 const run=async(method,args)=>{command={method,args,requestId:'extended_test_001',deadline:Date.now()+10000};events.visibilitychange();
   for(let i=0;i<100&&!results.length;i++)await new Promise(r=>setTimeout(r,1));assert.ok(results.length);return results.shift();};
 try{
   await bridge.setAutoConnect(true);await new Promise(r=>setTimeout(r,2));
   assert.equal((await run('get_capabilities',{})).result.exportFormats.length,7);
   assert.equal((await run('validate',{expectedRevision:0})).result.visualFidelityPassed,false);
   assert.match((await run('validate',{expectedRevision:1})).error,/REVISION_CONFLICT/);
   for(const format of capabilities().exportFormats){const r=await run('export',{expectedRevision:0,format});assert.equal(r.result.format,format);assert.ok(r.result.fileBase64);assert.equal(r.result.revision,0);}
   const before=calls.filter(c=>c.startsWith('/api/export-')).length;
   assert.match((await run('export',{expectedRevision:0,format:'https://evil.invalid'})).error,/导出格式/);
   assert.equal(calls.filter(c=>c.startsWith('/api/export-')).length,before);
 }finally{await bridge.disconnect();}
});
