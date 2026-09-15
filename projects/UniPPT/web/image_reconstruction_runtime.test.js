"use strict";
const test = require("node:test"), assert = require("node:assert/strict");
require("./presentation_host.js"); require("./image_reconstruction_runtime.js");
const rt = global.UniPptImageReconstruction;
const attachment = {name:"unseen.png",width:400,height:200,dataUrl:"data:image/png;base64,AA=="};
const inventory = () => ({ labels:[{id:"vertical",text:"Step 1",box:[-10,80,100,20,-90],size:18,flags:"b"},{id:"math",text:"T_{rgb}",box:[150,50,60,24],size:20}],photos:[{id:"photo",box:[310,40,50,40]}],modules:[{id:"encoder",box:[60,20,80,80],description:"stack"}] });
const plan = () => ({shapes:[["layer","path",60,20,40,80,"#cdddcc","transparent",0,"M 0 10 L 40 0 L 40 70 L 0 80 Z",4,8,0]],lines:[["arrow",140,60,180,60,"#222222",1,"dash",true]]});
test("normalized xyxy boxes map independently on both axes and rotate around the observed center",()=>{
  const inv={coordinateSpace:"normalized_999",labels:[{id:"v",text:"Step 1",bbox:[10,200,30,700],rotation:-90}],photos:[],modules:[]};
  const out=rt.materialize({shapes:[],lines:[]},inv,{width:1998,height:999});
  assert.deepEqual(out.inventory.labels[0].box,[-210,430,500,40,-90]);
  assert.equal(out.inventory.labels[0].size,36);
  assert.throws(()=>rt.materialize({shapes:[],lines:[]},{...inv,labels:[{...inv.labels[0],bbox:[10,200,5,700]}]},attachment),/bbox/);
});
test("compact source-only plan preserves native rotation, subscripts, repeats and exact crop",()=>{
  const args=rt.compilePlan(plan(),inventory(),attachment);
  const compiled=global.UniPptPresentationHost.compileNativeImageSlide(args,attachment);
  assert.equal(compiled.report.rotatedText,1); assert.equal(compiled.report.photos,1);
  assert.equal(compiled.report.nativeShapes,5); assert.equal(compiled.report.nativeConnectors,1);
  assert.equal(args.slide.objects.find((o)=>o.name==="math").textRuns[1].baseline,"sub");
  assert.equal(args.slide.sourceCrops[0].crop.left,310/400);
  assert.equal(args.slide.objects.find((o)=>o.name==="arrow").style.strokeDash,"dash");
  assert.ok(!("ocrDetection" in args));
});
test("patches are atomic and identity-bound without modifying source inventory",()=>{
  const state={inventory:inventory(),plan:plan()};
  const fixed=rt.patchState(state,{labels:[{...state.inventory.labels[0],text:"Step 2"}],lines:[{id:"arrow",remove:true}]});
  assert.equal(fixed.inventory.labels[0].text,"Step 2"); assert.equal(fixed.plan.lines.length,0);
  assert.equal(state.inventory.labels[0].text,"Step 1"); assert.equal(state.plan.lines.length,1);
  assert.throws(()=>rt.patchState(state,{javascript:[]}));
  assert.throws(()=>rt.patchState(state,{labels:[{id:"unknown",remove:true}]}));
});
test("review cannot pass with missing checks, issues or unapplied patches",()=>{
  const checks={text:true,rotation:true,modules:true,connections:true,photos:true,layout:true};
  assert.doesNotThrow(()=>rt.assertReview({passed:true,checks,issues:[],patch:{}}));
  assert.throws(()=>rt.assertReview({passed:true,checks:{...checks,rotation:false},issues:[]}));
  assert.throws(()=>rt.assertReview({passed:true,checks,issues:[],patch:{labels:[{id:"a"}]}}));
});
test("pipeline renders every repair and never returns failed output for approval",async()=>{
  const checks={text:true,rotation:true,modules:true,connections:true,photos:true,layout:true};
  const replies=[inventory(),plan(),{passed:false,checks:{...checks,text:false},issues:["label mismatch"],patch:{labels:[{...inventory().labels[0],id:"labels-1",text:"Step 2"}]}},{passed:true,checks,issues:[],patch:{}}];
  let renders=0,calls=0;
  const result=await rt.prepare({attachment,completion:async()=>({content:JSON.stringify(replies[calls++])}),render:async(args)=>{renders++; return "data:image/png;base64,AA==";}});
  assert.equal(renders,2); assert.equal(calls,4); assert.equal(result.args.slide.objects.find((o)=>o.name==="labels-1").text,"Step 2");
  assert.equal(result.report.humanReviewRequired,true);
  let n=0;
  await assert.rejects(rt.prepare({attachment,completion:async()=>({content:JSON.stringify([inventory(),plan(),{passed:false,checks,issues:["missing curve"],patch:{}}][n++])}),render:async()=>"data:image/png;base64,AA=="}),/未提交页面/);
});
test("preview XML escapes labels and uses compiled native geometry",()=>{
  const inv=inventory(); inv.labels[1].text='<script>&"';
  const svg=rt.previewSvg(rt.compilePlan(plan(),inv,attachment));
  assert.match(svg,/&lt;script&gt;&amp;&quot;/);assert.doesNotMatch(svg,/<script>/);
  assert.match(svg,/rotate\(-90\)/);assert.match(svg,/stroke-dasharray/);assert.match(svg,/clipPath/);
});
test("rotated OCR coordinates return to original pixels and duplicate visual labels are merged",()=>{
  const anchors=rt.makeAnchors([{rotation:90,regions:[{text:"Vertical label",location:[20,10,120,10,120,30,20,30]}]}],{width:400,height:200});
  assert.deepEqual(anchors[0].pixelBox,[10,80,30,180]);assert.equal(anchors[0].rotation,-90);
  const a=anchors[0];
  const inv=rt.anchorInventory({corrections:{a0:{text:"Vertical label",flags:"b"}},extraLabels:[{text:"Vertical label",bbox:a.bbox,rotation:-90}]},anchors,[],attachment);
  assert.equal(inv.labels.length,1);assert.equal(inv.labels[0].anchored,true);
  const patched=rt.patchState({inventory:inv,plan:{shapes:[],lines:[]}},{labels:[{...inv.labels[0],bbox:[0,0,999,999],text:"Corrected"}]});
  assert.deepEqual(patched.inventory.labels[0].bbox,a.bbox);
});
test("common model math syntax becomes native script runs without literal TeX or styled letter substitutes",()=>{
  assert.equal(rt.plainMath("$\\hat{\\mathbf{z}}_0$"),"ẑ_{0}");
  assert.equal(rt.plainMath("𝑀₀"),"M_{0}");
  assert.equal(rt.plainMath("\\mathcal{F}_{remove}"),"ℱ_{remove}");
});

test("explicit local path space is scaled correctly and ambiguous model geometry fails before review",()=>{
  const inv={coordinateSpace:"normalized_999",labels:[],photos:[],modules:[]};
  const shape={id:"panel",geometry:"path",bbox:[100,100,300,500],path:"M 0 10 L 30 0 L 30 110 L 0 120 Z"};
  assert.throws(()=>rt.materialize({shapes:[shape],lines:[]},inv,{width:999,height:999}),/viewBox/);
  const output=rt.materialize({shapes:[{...shape,viewBox:[30,120]}],lines:[]},inv,{width:999,height:999});
  assert.match(output.plan.shapes[0][9],/L 200 0/);
  assert.throws(()=>rt.materialize({shapes:[{...shape,viewBox:[30,120],dash:[6,4]}],lines:[]},inv,attachment),/dash/);
});

test("unchanged repair does not spend another model request on an identical scene",async()=>{
  const inv=inventory(),checks={text:false,rotation:true,modules:true,connections:true,photos:true,layout:true};
  const replies=[inv,plan(),{passed:false,checks,issues:["same text"],patch:{labels:[{...inv.labels[0],id:"labels-1"}]}}];
  let calls=0,previews=0;
  await assert.rejects(rt.prepare({attachment,completion:async()=>({content:JSON.stringify(replies[calls++])}),render:async()=>"data:image/png;base64,AA==",onPreview:()=>previews++}),/未提交页面/);
  assert.equal(calls,3);assert.equal(previews,1);
});

test("preview and partial-stage evidence survive a failed review request",async()=>{
  let calls=0,previews=0,failedStage;
  await assert.rejects(rt.prepare({attachment,completion:async({onToken})=>{
    if(calls++===0)return {content:JSON.stringify(inventory())};
    if(calls===2)return {content:JSON.stringify(plan())};
    onToken('{"passed":');throw new Error("output limit");
  },render:async()=>"data:image/png;base64,AA==",onPreview:()=>previews++,onStage:stage=>{if(stage.error)failedStage=stage;}}),/output limit/);
  assert.equal(previews,1);assert.equal(failedStage.content,'{"passed":');
});
