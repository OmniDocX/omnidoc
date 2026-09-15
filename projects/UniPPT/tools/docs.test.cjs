'use strict';
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),test=require('node:test'),assert=require('node:assert/strict'),{execFileSync}=require('node:child_process');
const root=path.resolve(__dirname,'..'),{render,inline}=require('./docs_html.cjs');
const read=p=>fs.readFileSync(path.join(root,p),'utf8');
const pages=['web/docs/index.html','web/docs/integration.html','web/mcp-protocol.html'];
test('published documentation links, assets and section anchors resolve locally',()=>{
 for(const file of pages){
  const html=read(file),ids=[...html.matchAll(/\bid="([^"]+)"/g)].map(m=>m[1]);
  assert.equal(new Set(ids).size,ids.length);assert.match(html,/<main id="content"/);assert.match(html,/aria-current="page"/);
  assert.doesNotMatch(html,/<script|<iframe|onerror=/i);assert.match(html,/width=device-width,initial-scale=1/);
  for(const [,url]of html.matchAll(/(?:href|src)="([^"]+)"/g)){
   if(url.startsWith('#')){assert.ok(ids.includes(url.slice(1)),file+': '+url);continue;}
   if(!url.startsWith('/'))continue;
   const asset=path.join(root,'web',url.endsWith('/')?url+'index.html':url);
   assert.ok(fs.statSync(asset).isFile(),file+': '+url);
  }
 }
 assert.match(read('web/index.html'),/href="\/docs\/"/);
});
test('renderer escapes source content and supports semantic tables, lists and safe links',()=>{
 const value=render('# T\n\n## Section\n\n1. First\n2. Second\n\n| A | B |\n| --- | --- |\n| x | y |\n\n<script>alert(1)</script>').html;
 assert.match(value,/<ol>/);assert.match(value,/<th scope="col">/);assert.match(value,/&lt;script&gt;/);assert.doesNotMatch(value,/<script/);
 assert.doesNotMatch(inline('[bad](javascript:alert) [bad](//evil.test) [bad](data:abc)'),/<a /);
 assert.match(inline('[good](/docs/)'),/<a href="\/docs\/">good<\/a>/);
});
test('all example calls validate against current local tool schemas',()=>{
 const context={};vm.createContext(context);vm.runInContext(read('web/ai_runtime.js'),context);
 const tools=JSON.parse(read('web/mcp-tools.json'));let count=0;
 for(const file of ['docs/AI_MCP_INTEGRATION.md','docs/MCP_PROTOCOL.md']){
  const fence=String.fromCharCode(96).repeat(3),re=new RegExp(fence+'json\\r?\\n([\\s\\S]*?)'+fence,'g');
  for(const [,code]of read(file).matchAll(re)){
   const value=JSON.parse(code);
   if(value.method!=='tools/call')continue;
   const tool=tools.find(t=>t.name===value.params.name);assert.ok(tool,value.params.name);
   context.UniPptAiRuntime.validateArguments(value.params.arguments,tool.inputSchema);count++;
  }
 }
 assert.equal(count,5);
});
test('static schema snapshots preserve native operations and all module/row kinds',()=>{
 const runtime=JSON.parse(read('web/mcp-runtime-schemas.json'));
 assert.equal(runtime.native.allowedOperations.length,14);
 assert.deepEqual(runtime.modules,require('../web/mcp_scene.js').moduleSchema());
 assert.deepEqual(runtime.imageRows,require('../web/mcp_image_layout.js').schema());
 assert.equal(runtime.imageRows.kinds.length,20);
 const advertised=JSON.parse(read('web/mcp-tools.json')).map(t=>t.name);
 assert.deepEqual(advertised,require('./unippt_mcp.cjs').TOOLS.map(t=>t.name));assert.ok(!advertised.some(n=>n.startsWith('unippt_ai_')));
 assert.match(read('docs/AI_MCP_INTEGRATION.md'),/调用方使用自己的模型/);
});
test('offline handoff is complete, portable and contains no actual credential pattern',()=>{
 const book=read('docs/UniPPT-MCP-AI-INTEGRATION-2026-09-11.md');
 assert.equal(book,read('web/docs/unippt-integration.md'));
 assert.match(book,/本机 stdio/);assert.doesNotMatch(book,/OAuth|PKCE/);assert.match(book,/调用方使用自己的模型/);
 assert.match(book,/附录 A：完整本机工具/);assert.match(book,/附录 B：原生操作/);
 assert.doesNotMatch(book,/Bearer (?!<)[A-Za-z0-9_-]{20,}|sk-[A-Za-z0-9_-]{20,}|-----BEGIN .*PRIVATE KEY|(?:\d{1,3}\.){3}\d{1,3}:(?:22|8141)/);
});
test('documentation build is deterministic',()=>{
 const paths=[...pages,'web/mcp-protocol.md','web/mcp-tools.json','web/mcp-runtime-schemas.json','web/docs/unippt-integration.md'];
 const before=paths.map(read);execFileSync(process.execPath,[path.join(__dirname,'build_mcp_docs.cjs')],{cwd:root});
 assert.deepEqual(paths.map(read),before);
});
