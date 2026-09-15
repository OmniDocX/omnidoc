"use strict";
const test=require("node:test"),assert=require("node:assert/strict");
require("./image_native_tracer.js");
test("pixel tracer emits native paths and excludes explicitly bounded photos",()=>{
  const w=80,h=50,data=new Uint8ClampedArray(w*h*4).fill(255);
  for(let y=10;y<30;y++)for(let x=10;x<30;x++){const i=(y*w+x)*4;data[i]=30;data[i+1]=120;data[i+2]=180;}
  for(let y=10;y<30;y++)for(let x=40;x<60;x++){const i=(y*w+x)*4;data[i]=120;data[i+1]=30;data[i+2]=40;}
  const result=global.UniPptImageNativeTracer.trace(data,w,h,{labels:[],photos:[{box:[40,10,20,20]}]});
  assert.equal(result.shapes.length,1);assert.deepEqual(result.shapes[0].slice(2,6),[10,10,20,20]);
  assert.equal(result.shapes[0][1],"path");assert.ok(result.shapes[0][9].endsWith("Z"));
  assert.equal(data[(10*w+10)*4],30);
});
test("thin pixel strokes recover as independent native connectors",()=>{
  const w=40,h=20,data=new Uint8ClampedArray(w*h*4).fill(255);
  for(let x=4;x<35;x++){const i=(10*w+x)*4;data[i]=data[i+1]=data[i+2]=0;}
  const result=global.UniPptImageNativeTracer.trace(data,w,h,{labels:[],photos:[]});
  assert.equal(result.lines.length,1);assert.equal(result.shapes.length,0);
  assert.equal(result.lines[0][8],false);
});
test("diagonal connectivity preserves single-pixel scientific strokes without inventing area",()=>{
  const w=30,h=30,data=new Uint8ClampedArray(w*h*4).fill(255);
  for(let i=4;i<24;i++)data.set([105,169,132,255],(i*w+i)*4);
  const before=data.slice(),inventory={labels:[],photos:[]};
  assert.equal(global.UniPptImageNativeTracer.trace(data,w,h,inventory).shapes.length,0);
  const result=global.UniPptImageNativeTracer.trace(data,w,h,inventory,{connectThinColor:true,connectDiagonals:true});
  assert.equal(result.shapes.length,1);assert.deepEqual(result.shapes[0].slice(2,6),[4,4,20,20]);
  assert.deepEqual(data,before);
});
test("chromatic sparse-stroke normalization leaves solid panels unchanged",()=>{
  const w=35,h=35,data=new Uint8ClampedArray(w*h*4).fill(255);
  for(let y=5;y<30;y++)for(let x=5;x<30;x++)data.set([192,218,180,255],(y*w+x)*4);
  const inventory={labels:[],photos:[]},a=global.UniPptImageNativeTracer.trace(data,w,h,inventory),b=global.UniPptImageNativeTracer.trace(data,w,h,inventory,{connectThinColor:true,connectDiagonals:true});
  assert.deepEqual(a.shapes,b.shapes);
});
