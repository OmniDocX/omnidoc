'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const {dashGeometry}=require('./native_path_dash.cjs');
const {compileLayout}=require('./unippt_mcp_image.cjs');
test('explicit dash length/phase are source pixels and preserve one native path',async()=>{
 const g=dashGeometry({width:20,height:1,pathData:'M 0 0 L 20 0'},20,1,[7,7]);assert.equal(g.pathData,'M 0 0 L 7 0 M 14 0 L 20 0');
 const shifted=dashGeometry({width:20,height:1,pathData:'M 0 0 L 20 0'},20,1,[7,7],4);assert.equal(shifted.pathData,'M 0 0 L 3 0 M 10 0 L 17 0');
 const c=await compileLayout([['outline','box',0,0,200,100,'roundRect',{dashPattern:[7,7],lineWidth:2}]],{width:200,height:100},{width:100,height:50});
 assert.equal(c.objects.length,1);assert.equal(c.objects[0].frame.width,100);assert.equal(c.objects[0].style.strokeWidth,1);assert.equal(c.objects[0].style.strokeDash,undefined);assert.ok(c.objects[0].customGeometry.pathData.split('M').length>10);
});
test('explicit curved dashes retain arrow heads, reset at subpaths and reject bad inputs',async()=>{
 const c=await compileLayout([['curve','path',0,0,100,100,'M 0 100 C 0 0 50 0 100 0',{dashPattern:[7,7],headEnd:true}]],{width:100,height:100},{width:100,height:100});assert.equal(c.objects.length,2);assert.equal(c.objects[1].style.strokeWidth,0);assert.notEqual(c.objects[1].style.fill,'transparent');
 const multi=dashGeometry({width:20,height:2,pathData:'M 0 0 L 10 0 M 0 2 L 10 2'},20,2,[7,7]);assert.equal(multi.pathData,'M 0 0 L 7 0 M 0 2 L 7 2');
 for(const p of [[0,1],[-1,2],[1],[NaN,1],[.01,2]])assert.throws(()=>dashGeometry({width:20,height:1,pathData:'M 0 0 L 20 0'},20,1,p));
 await assert.rejects(compileLayout([['filled','box',0,0,20,20,'rect',{fill:'#fff',dashPattern:[2,2]}]],{width:100,height:100},{width:100,height:100}),/unfilled/);
});
