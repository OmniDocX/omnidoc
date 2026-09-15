'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {resolveRows}=require('./unippt_mcp_row_patch.cjs');
const {createStore,sharpRuntime,compileLayout}=require('./unippt_mcp_image.cjs');
const row=['net','network',20,10,100,80,{label:[{text:'F',fontSize:25},{text:'x',baseline:'sub',fontSize:12}],title:'Network',symbol:'snowflake'},{labelFrame:[8,20,90,30],fill:'#246688',textOverrides:{label:{frame:[9,25,88,24],options:{font:'Cambria Math',italic:true}},title:{options:{size:15}}}}];
const job={slideId:'slide',regions:{net:['native']},layoutRows:{net:row}};
const args={slideId:'slide',patches:[{id:'net',options:{textOverrides:{label:{frame:[10,25,87,24]}}}}]};
test('sparse patches preserve nested roles, formulas, geometry and input immutability',async()=>{
 const before=JSON.stringify(job),resolved=resolveRows(args,job),full=structuredClone(row);full[7].textOverrides.label.frame=[10,25,87,24];
 assert.deepEqual(resolved.rows,[full]);assert.deepEqual(resolved.replace,['net']);assert.equal(JSON.stringify(job),before);
 const source={width:200,height:100},page={width:200,height:100};assert.deepEqual(await compileLayout(resolved.rows,source,page),await compileLayout([full],source,page));
 assert.ok(resolved.authoring.inputBytes<resolved.authoring.resolvedRowBytes);assert.equal(resolved.authoring.modelTimeMeasured,false);
});
test('frame/value/arrays replace explicitly while unchanged options survive',()=>{
 const p=resolveRows({slideId:'slide',patches:[{id:'net',frame:[1,2,60,70],value:{label:'y'},options:{labelFrame:[1,2,3,4]}}]},job).rows[0];
 assert.deepEqual(p.slice(2,6),[1,2,60,70]);assert.deepEqual(p[6],{label:'y'});assert.deepEqual(p[7].labelFrame,[1,2,3,4]);assert.equal(p[7].fill,row[7].fill);
});
test('patches reject unknown/old/foreign rows, duplicate edits, unsafe keys and expansion abuse',()=>{
 for(const a of [{}, {...args,slideId:'other'}, {...args,rows:[row]}, {...args,patches:[args.patches[0],args.patches[0]]}, {...args,patches:[{id:'missing',value:'x'}]}, {...args,patches:[{id:'net',shell:'x'}]}, {...args,patches:[{id:'net'}]}, {...args,patches:[{id:'net',frame:[0,0,0,1]}]}, {...args,patches:[{id:'net',options:null}]}, {...args,patches:[JSON.parse('{"id":"net","options":{"__proto__":{"polluted":true}}}')]}, {...args,patches:Array.from({length:161},()=>args.patches[0])}])assert.throws(()=>resolveRows(a,job));
 assert.throws(()=>resolveRows(args,{...job,layoutRows:{}}),/older jobs/);assert.equal({}.polluted,undefined);
});
test('source row journal changes only at commit, persists, replays exactly and rejects stale sparse repair',async t=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'unippt-patches-')),sharp=sharpRuntime();t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
 const source=path.join(root,'source.png');await sharp({create:{width:200,height:100,channels:3,background:'#fff'}}).png().toFile(source);
 const store=createStore(path.join(root,'jobs'),sharp),begin=await store.begin(source,'session',async()=>({writeAccess:true,document:{size:{width:200,height:100}},revision:0}));
 const firstArgs={sessionId:'session',jobId:begin.jobId,expectedRevision:0,requestId:'first-patch-test',rows:[row]},first=await store.plan(firstArgs);
 assert.equal(store.load(begin.jobId).layoutRows,undefined);
 store.committed(begin.jobId,first,{writeCommitted:false,revision:1});assert.equal(store.load(begin.jobId).layoutRows,undefined);
 store.committed(begin.jobId,first,{writeCommitted:true,revision:1,slideId:'slide'});
 const next=createStore(path.join(root,'jobs'),sharp),patchArgs={...args,sessionId:'session',jobId:begin.jobId,expectedRevision:1,requestId:'sparse-patch-test'},p=await next.plan(patchArgs);
 assert.deepEqual(p.command.compiledModules[0].replaceObjectIds,first.regions.net);assert.deepEqual(next.load(begin.jobId).layoutRows.net,row);
 next.committed(begin.jobId,p,{writeCommitted:true,revision:2,slideId:'slide'});
 assert.deepEqual(next.load(begin.jobId).layoutRows.net,p.sourceRows[0]);assert.deepEqual(await next.plan(patchArgs),p,'Identical request recovers its original plan after commit');
 await assert.rejects(next.plan({...patchArgs,requestId:'stale-patch-test'}),/current image job revision/);
 next.committed(begin.jobId,first,{writeCommitted:true,revision:1,slideId:'slide'});assert.deepEqual(next.load(begin.jobId).layoutRows.net,p.sourceRows[0]);
});
test('MCP advertises optional rows plus bounded patches without changing native quality gates',()=>{
 const tool=require('./unippt_mcp.cjs').TOOLS.find(t=>t.name==='unippt_apply_image_layout');assert.equal(tool.inputSchema.required.includes('rows'),false);assert.equal(tool.inputSchema.properties.patches.maxItems,160);assert.match(tool.description,/checked path/);
});
