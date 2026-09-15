'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const {create,KEY}=require('./mcp_connection_policy.js');
test('local MCP global policy defaults on, remembers off, and rejects remote origins',()=>{
  const values=new Map(),storage={getItem:k=>values.get(k)??null,setItem:(k,v)=>values.set(k,v)},changes=[];
  const first=create({origin:'http://127.0.0.1:8141',storage,onChange:v=>changes.push(v)});assert.equal(first.enabled,true);
  first.setEnabled(false);assert.equal(create({origin:'http://127.0.0.1:8141',storage}).enabled,false);
  first.storageChanged({key:KEY,newValue:'true'});assert.equal(first.enabled,true);first.storageChanged({key:'other',newValue:'false'});assert.equal(first.enabled,true);
  const remote=create({origin:'https://example.com',storage});assert.equal(remote.setEnabled(true),false);assert.equal(remote.enabled,false);assert.deepEqual(changes,[false,true]);
});
test('automatic bridge connects without a dialog, rotates session on document load and revokes on disable',async()=>{
  const events={},calls=[],listeners={},context={console,URL,AbortSignal,performance,setTimeout,clearTimeout,location:{origin:'http://127.0.0.1:8141'},document:{visibilityState:'visible',addEventListener:(n,f)=>listeners[n]=f}};
  let sequence=0,deck={title:'One',slides:[{id:'s'}]};
  context.fetch=async(url,options)=>{
    const body=JSON.parse(options.body);calls.push({url,body});
    return {ok:true,json:async()=>url.endsWith('/connect')?{sessionId:'s'+(++sequence),secret:'private'}:url.endsWith('/poll')?{command:null}:{ok:true}};
  };
  vm.createContext(context);vm.runInContext(fs.readFileSync(path.join(__dirname,'mcp_scene.js'),'utf8'),context);vm.runInContext(fs.readFileSync(path.join(__dirname,'mcp_bridge.js'),'utf8'),context);
  const host={on:(n,f)=>events[n]=f,selection:{get:()=>({slideId:'s'})}},bridge=context.UniPptMcpBridge.create({host,getDeck:()=>deck,undo:()=>{}});
  try{
    await bridge.setAutoConnect(true);assert.equal(sequence,1);assert.equal(calls.find(c=>c.url.endsWith('/connect')).body.writeAccess,true);
    deck={title:'Two',slides:[{id:'t'}]};events.load();await new Promise(r=>setTimeout(r,20));assert.equal(sequence,2);assert.ok(calls.some(c=>c.url.endsWith('/disconnect')&&c.body.sessionId==='s1'));
    await bridge.setAutoConnect(false);listeners.visibilitychange();await new Promise(r=>setTimeout(r,20));assert.equal(sequence,2);assert.ok(calls.some(c=>c.url.endsWith('/disconnect')&&c.body.sessionId==='s2'));
    context.location.origin='https://public.example';await bridge.setAutoConnect(true);assert.equal(sequence,2);
  }finally{await bridge.disconnect();}
});
test('MCP startup loads a small dedicated feature after a deck exists, with a visible switch',()=>{
  const html=fs.readFileSync(path.join(__dirname,'index.html'),'utf8'),app=fs.readFileSync(path.join(__dirname,'app.js'),'utf8');
  const feature=html.match(/mcp:\s*\{\s*scripts:\s*\[([\s\S]*?)\]/)[1];assert.match(feature,/mcp_connection_policy/);assert.doesNotMatch(feature,/ai_runtime|image_native_tracer/);
  assert.match(html,/id="mcpConnect"[^>]*role="switch"/);assert.match(app,/if\(state.deck\)requestAnimationFrame/);assert.match(app,/setEnabled\(!mcpPolicy.enabled\)/);
});

test('remote origins cannot open a manual or automatic local bridge, including legacy remote options',async()=>{
 const calls=[],context={console,URL,AbortSignal,performance,setTimeout,clearTimeout,location:{origin:'https://example.com'},document:{addEventListener(){}},confirm:()=>true,fetch:async url=>{calls.push(url);throw Error('Unexpected network');}};
 vm.createContext(context);for(const name of ['mcp_scene.js','mcp_bridge.js'])vm.runInContext(fs.readFileSync(path.join(__dirname,name),'utf8'),context);
 const bridge=context.UniPptMcpBridge.create({remote:true,canAutoConnect:()=>true,host:{on(){},selection:{get:()=>({slideId:'s'})}},getDeck:()=>({title:'Local',slides:[{id:'s'}]})});
 await bridge.setAutoConnect(true);await assert.rejects(bridge.connect({isTrusted:true}),/仅支持本机/);assert.deepEqual(calls,[]);
 assert.equal(create({origin:context.location.origin,remote:true,isEligible:()=>true}).enabled,false);
 await bridge.disconnect();
});
