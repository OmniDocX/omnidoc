const {test}=require('node:test');
const assert=require('node:assert/strict');
const p=require('./image_precision_runtime.js');
const s=require('./image_semantic_module_runtime.js');
require('./image_reconstruction_runtime.js');
require('./image_native_tracer.js');
test('rotated text is measured in visible image coordinates',()=>{
  const box=p.visibleFrame({x:-40,y:60,width:120,height:20,rotation:-90});
  box.forEach((v,i)=>assert.ok(Math.abs(v-[10,10,20,120][i])<1e-9));
});
test('colored typography matching uses source core color instead of guessed model darkness',()=>{
  const pixels=new Uint8ClampedArray(30*20*4).fill(255);
  for(let y=4;y<16;y++)for(let x=8;x<22;x++)pixels.set([182,65,48,255],(y*30+x)*4);
  assert.deepEqual(p.sourceTextColor(pixels,30,20,[5,2,20,16],[139,35,35]),[182,65,48]);
  assert.deepEqual(p.sourceTextColor(pixels,30,20,[0,0,3,3],[139,35,35]),[139,35,35]);
});
test('crop and clustering are source-driven and preserve disconnected modules',()=>{
  const data=Uint8ClampedArray.from({length:8*6*4},(_,i)=>i%256);
  const c=p.crop(data,8,[2,1,3,2]);assert.equal(c.length,24);assert.deepEqual(c.slice(0,12),data.slice(40,52));
  const grouped=p.clusters([{box:[0,0,5,5]},{box:[7,0,5,5]},{box:[80,0,5,5]}],3);assert.deepEqual(grouped.map(g=>g.length),[2,1]);
});
test('ink measurement excludes photos and separates blue bars from black text',()=>{
  const pixels=new Uint8ClampedArray(20*20*4).fill(255);
  for(let y=2;y<18;y++)for(let x=2;x<6;x++)pixels.set([36,103,156,255],(y*20+x)*4);
  for(let y=8;y<12;y++)for(let x=10;x<14;x++)pixels.set([23,23,23,255],(y*20+x)*4);
  assert.deepEqual(p.inkBounds(pixels,20,20,[0,0,20,20],[23,23,23]),[10,8,4,4]);
  assert.equal(p.inkBounds(pixels,20,20,[0,0,20,20],[23,23,23],[[9,7,6,6]]),null);
});
test('fitting preserves native scripts and rejects implausible matches',()=>{
  const args={slide:{objects:[{name:'t',frame:{x:10,y:10},textStyle:{fontSize:20},textRuns:[{text:'F',fontSize:20},{text:'x',baseline:'sub',fontSize:13}]}]}};
  p.fitTexts(args,[{name:'t',ratio:1.1,dx:2,dy:-1}]);assert.equal(args.slide.objects[0].textStyle.fontSize,22);assert.equal(args.slide.objects[0].textRuns[1].baseline,'sub');
  const before=JSON.stringify(args);p.fitTexts(args,[{name:'t',ratio:9,dx:50,dy:80}]);assert.equal(JSON.stringify(args),before);
});
test('corrected inventory preserves mathematical alphabets and checks anchor coverage',()=>{
  const anchors=[{id:'a0',bbox:[0,0,40,30],rotation:0}],source={width:100,height:100};
  const result=p.correctedInventory({corrections:{a0:{text:'𝓕_{𝒓𝒆𝒎𝒐𝒗𝒆}'}},extraLabels:[],modules:[]},anchors,[],source,global.UniPptImageReconstruction);
  assert.equal(result.labels[0].text,'𝓕_{𝒓𝒆𝒎𝒐𝒗𝒆}');
  assert.throws(()=>p.correctedInventory({corrections:{}},anchors,[],source,global.UniPptImageReconstruction),/id 不完整/);
});
test('standard model TeX becomes native script markup without evaluating code',()=>{
  assert.equal(p.mathText('\\hat{z}_0'), 'ẑ_{0}');
  assert.equal(p.mathText('T_{\\mathrm{rgb}}'), 'T_{rgb}');
  assert.equal(p.mathText('𝓕_{𝒓𝒆𝒎𝒐𝒗𝒆}'),'𝓕_{𝒓𝒆𝒎𝒐𝒗𝒆}');
  assert.throws(()=>p.mathText('\\input{evil}'),/未解析的公式命令/);
});
test('empty model text retains OCR as an explicitly uncertain label',()=>{
  const a=[{id:'a0',readings:['F'],bbox:[10,10,25,25],rotation:0}];
  const result=p.correctedInventory({corrections:{a0:{text:''}},extraLabels:[],modules:[]},a,[],{width:100,height:100},global.UniPptImageReconstruction);
  assert.equal(result.labels[0].text,'F');assert.equal(result.labels[0].uncertain,true);
  assert.equal(p.mathText('𝑧̂₀'),'𝑧̂_{0}');
});
test('font atlas retains native runs and stays within bounded renderer dimensions',()=>{
  const original={name:'t',kind:'text',text:'F1',frame:{x:1,y:2,width:20,height:20,rotation:-90},textStyle:{fontFamily:'Cambria Math',fontSize:20},textRuns:[{text:'F',fontSize:20},{text:'1',baseline:'sub',fontSize:13}]};
  const atlas=p.fontAtlas({slide:{objects:[original]}});assert.equal(atlas.cells.length,4);assert.equal(atlas.args.slide.objects[0].frame.rotation,0);
  assert.equal(atlas.args.slide.objects[0].textRuns[1].baseline,'sub');assert.equal(original.frame.rotation,-90);
  assert.ok(atlas.args.sourceSize.width<=4096&&atlas.args.sourceSize.height<=4096);
});
test('white input has no invented semantic modules or fans',()=>{
  const d=p.discover(new Uint8ClampedArray(100*100*4).fill(255),100,100,[],s);assert.deepEqual(d,{fans:[],modules:[]});
});
test('isolated quality deck has the same complete model defaults as editor insertion',()=>{
  require('./presentation_host.js');
  const args={sourceSize:{width:100,height:80},slide:{name:'test',objects:[{kind:'text',text:'A',frame:{x:1,y:2,width:40,height:20},textStyle:{fontSize:14}}],sourceCrops:[],verification:{labels:['A'],rotatedTextCount:0,diagram:false}}};
  const deck=global.UniPptPresentationHost.compileNativeImageDeck(args),slide=deck.slides[0],object=slide.objects[0];
  assert.ok(slide.id);assert.ok(object.id);assert.equal(object.style.opacity,1);assert.equal(object.textStyle.align,'left');
  assert.deepEqual(object.children,[]);assert.deepEqual(slide.animations,[]);
  assert.equal(args.slide.objects[0].style,undefined);
});
test('fan ownership does not erase a blue frozen icon within its bounds',()=>{
  const m={box:[0,0,100,100],report:{method:'two-sided-fan-source-island-geometry',fill:'#eab498'}},base={kind:'shape',frame:{x:10,y:10,width:15,height:15}};
  assert.equal(p.ownedByModule({...base,style:{fill:'#36a6cb'}},m),false);
  assert.equal(p.ownedByModule({...base,style:{fill:'#f9eadd'}},m),false);
  assert.equal(p.ownedByModule({...base,style:{fill:'#eab498'}},m),true);
});
test('production page loads precision runtime and uses the native quality endpoint',()=>{
  const fs=require('fs'),path=require('path'),app=fs.readFileSync(path.join(__dirname,'app.js'),'utf8'),index=fs.readFileSync(path.join(__dirname,'index.html'),'utf8');
  assert.match(index,/image_semantic_module_runtime.js/);assert.match(index,/image_precision_runtime.js/);
  assert.match(app,/const reconstruction = globalThis.UniPptImagePrecision/);assert.match(app,/renderNative: reconstruction.renderNative/);
  assert.match(app,/imageWorkflowStarted/);assert.match(app,/未调用付费识别/);
  assert.doesNotMatch(fs.readFileSync(path.join(__dirname,'image_precision_runtime.js'),'utf8'),/reviewed-blueprint|20260908|PolarFree|幻灯片2/);
});
