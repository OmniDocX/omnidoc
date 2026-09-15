'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const scene=require('../web/mcp_scene.js');
const {preparePatch}=require('../web/mcp_bridge.js');
test('stdio adapter advertises combined write and module discovery',()=>{
 const {TOOLS}=require('./unippt_mcp.cjs');assert.equal(TOOLS.find(t=>t.name==='unippt_apply_scene').annotations.readOnlyHint,false);assert.ok(TOOLS.find(t=>t.name==='unippt_get_module_schema'));
});
test('module expansion is deterministic, compact and natively editable',()=>{
 const modules=[{id:'enc',kind:'layers',frame:{x:5,y:10,width:20,height:100},count:6,dx:12,shrink:10},{id:'flow',kind:'arrow',from:[0,50],to:[200,50]},{id:'frozen',kind:'snowflake',frame:{x:10,y:100,width:24,height:24}},{id:'label',kind:'text',textRuns:[{text:'z'},{text:'0',baseline:'sub'}]}];
 const a=scene.compileModules(modules);assert.deepEqual(a,scene.compileModules(modules));assert.deepEqual(a.map(m=>m.objects.length),[6,2,1,1]);
 assert.ok(a.flatMap(m=>m.objects).every(o=>o.kind!=='image'&&o.kind!=='connector'));
 const ops=scene.sceneOperations({slides:[]},{compiledModules:a});assert.equal(ops.length,1);
 const prepared=preparePatch({slides:[]},ops);assert.equal(prepared[0].slide.objects.at(-1).textParagraphs[0].runs[1].baseline,'sub');
});
test('opaque snowflake uses one editable path; alpha-overlap behavior and old module repair survive',()=>{
 const module={id:'snow',kind:'snowflake',frame:{x:21,y:30,width:24,height:24}};
 const packed=scene.compileModules([module])[0];assert.equal(packed.objects.length,1);
 assert.equal((packed.objects[0].customGeometry.pathData.match(/M /g)||[]).length,30);
 const old=scene.compileModules([{...module,style:{opacity:.5}}])[0];assert.equal(old.objects.length,30);
 const repair=scene.compileModules([{...module,replaceObjectIds:old.objects.map(o=>o.id)}]);
 const ops=scene.sceneOperations({slides:[{id:'s',objects:old.objects}]},{slideId:'s',compiledModules:repair});
 assert.equal(ops.filter(o=>o.op==='removeObject').length,29);assert.equal(ops.filter(o=>o.op==='updateObject').length,1);
});
test('fractional viewports, quadratic paths and custom connectors normalize before write',()=>{
 const g={width:4.435,height:10.5,pathData:'M 0 0 Q 2.2 3.1 4.435 10.5'};
 const n=scene.normalizeGeometry(g);assert.equal(n.width,4435);assert.match(n.pathData,/ C /);assert.doesNotMatch(n.pathData,/Q|\./);assert.deepEqual(scene.normalizeGeometry(n),n);
 const repairs=[],op=preparePatch({slides:[]},[{op:'insertSlide',slide:{objects:[{id:'line',kind:'connector',customGeometry:g}]}}],repairs)[0];
 assert.equal(op.slide.objects[0].kind,'shape');assert.equal(repairs.length,2);
 for(const bad of ['M 0 0 Q 1 2','M 0 0 A 5 5 0 0 0 3 3','m 0 0 l 1 2','M 0 0 L NaN 1','M 0 0 L 1e100 1'])assert.throws(()=>scene.normalizeGeometry({width:10,height:10,pathData:bad}));
});
test('region repair requires exact observed IDs and preserves unrelated objects',()=>{
 const original=scene.compileModules([{id:'one',kind:'layers',count:5}])[0].objects,keep={id:'user-text',kind:'text',text:'Keep me'};
 const deck={slides:[{id:'s',objects:[...original,keep]}]},modules=[{id:'one',kind:'layers',count:3,replaceObjectIds:original.map(o=>o.id)}];
 const ops=scene.sceneOperations(deck,{slideId:'s',compiledModules:scene.compileModules(modules)});
 assert.equal(ops.length,5);assert.equal(ops.filter(o=>o.op==='removeObject').length,2);assert.ok(ops.every(o=>o.objectId!=='user-text'));
 assert.throws(()=>scene.sceneOperations(deck,{slideId:'s',compiledModules:scene.compileModules([{id:'one',kind:'layers'}])}),/collision/);
 assert.throws(()=>scene.compileModules([{id:'a',kind:'layers',count:65}]));
 assert.throws(()=>scene.compileModules([{id:'a',kind:'layers'},{id:'a',kind:'text'}]));
 assert.throws(()=>scene.compileModules([{id:'a',kind:'layers',shrink:100}]));
});
test('bounds checks account for rotation and retain explicit visual uncertainty',()=>{
 const c=scene.checkSlide({objects:[{id:'r',kind:'text',frame:{x:-40,y:50,width:100,height:10,rotation:270}}]},100,200);
 assert.equal(c.issues.length,0);assert.equal(c.visualFidelityPassed,false);
 const b=scene.checkSlide({objects:[{id:'r',frame:{x:200,y:0,width:100,height:10}}]},100,200);assert.equal(b.issues[0].code,'outside_canvas');
});
