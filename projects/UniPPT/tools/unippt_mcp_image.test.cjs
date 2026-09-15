'use strict';
const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path');
const {compileLayout,textRuns,createStore,sharpRuntime}=require('./unippt_mcp_image.cjs');
const {preparePatch}=require('../web/mcp_bridge.js');
const {sceneOperations}=require('../web/mcp_scene.js');
const page={width:100,height:100},source={width:200,height:100};
const row=(id='label')=>[id,'text',20,10,80,20,'z_{0}^{2}',{size:20,rotate:270}];
test('new image protocol is model neutral, exposes combined layout and end clock',()=>{
 const {TOOLS}=require('./unippt_mcp.cjs');assert.equal(TOOLS.find(t=>t.name==='unippt_apply_image_layout').annotations.readOnlyHint,false);for(const name of ['begin_image','finish_image'])assert.ok(TOOLS.find(t=>t.name==='unippt_'+name));
});
test('compact rows scale native frames/styles, preserve rotation, baseline and dash',async()=>{
 const compiled=await compileLayout([row(),['border','box',0,0,200,100,'roundRect',{dash:'dash',lineWidth:2}]],source,page);
 assert.deepEqual(compiled.fit,{scale:.5,left:0,top:25});const o=compiled.objects[0];assert.deepEqual(o.frame,{x:10,y:30,width:40,height:10,rotation:270});assert.equal(o.textStyle.fontSize,10);assert.equal(compiled.objects[1].style.strokeDash,'dash');
 const native=preparePatch({slides:[]},sceneOperations({slides:[]},{compiledModules:[{id:'x',objects:compiled.objects,replaceObjectIds:[]}]}));
 assert.equal(native[0].slide.objects[0].textParagraphs[0].runs[1].baseline,'sub');assert.equal(native[0].slide.objects[0].textParagraphs[0].runs[2].baseline,'super');
});
test('shorthand is text baselines, not an executable or LaTeX evaluator',()=>{
 assert.deepEqual(textRuns('A_{ij}+x^{2}'),[{text:'A'},{text:'ij',baseline:'sub'},{text:'+x'},{text:'2',baseline:'super'}]);assert.deepEqual(textRuns('f(a)'),[{text:'f(a)'}]);assert.throws(()=>textRuns({}));
});
test('editable tunable presets and packed snowflake expand deterministically',async()=>{
 const rows=[['f','fan',10,10,100,60,8,{fill:'#ffeeee'}],['u','unet',10,10,100,60,6,{}],['fire','flame',10,10,20,30,null],['snow','snowflake',10,10,20,20,null]];
 const a=await compileLayout(rows,source,page);assert.equal(a.objects.length,16);assert.ok(a.objects.every(o=>o.kind==='shape'));assert.deepEqual(a,await compileLayout(rows,source,page));
});
test('line, arrow and fractional paths are native editable geometry',async()=>{
 const c=await compileLayout([['l','line',10,10,1,1,[30,30]],['a','arrow',10,30,1,1,[70,30]],['p','path',5,5,10,10,'M 0 0 Q 4.5 2 10 10']],source,page);
 assert.equal(c.objects.length,4);assert.ok(c.objects.every(o=>o.customGeometry&&!o.customGeometry.pathData.includes('Q')));
});
test('photo crop capability receives bounded integer source rect, order is stable',async()=>{
 const rects=[];const c=await compileLayout([['p','photo',10,20,30,40,null],row()],source,page,async r=>{rects.push(r);return 'PNG';});assert.deepEqual(rects,[[10,20,30,40]]);assert.equal(c.objects[0].kind,'image');
 for(const rect of [[0,0,201,10],[-1,0,10,10],[0,0,1.5,10]])await assert.rejects(compileLayout([['p','photo',0,0,20,20,rect]],source,page,()=>''),/outside/);
});
test('invalid rows, cross-preset collisions and expansion limits reject before writes',async()=>{
 for(const rows of [[row(),row()],[row('__proto__')],[['x','shell',1,1,1,1,'']],[[...row().slice(0,4),0,20,'x']],[['p','path',0,0,10,10,'m 0 0 l 10 10']],[['u','unet',0,0,100,100,2,{}],row('u_0')],Array.from({length:17},(_,i)=>['s'+i,'layers',0,0,2,100,32])])await assert.rejects(compileLayout(rows,source,page));
});
test('image jobs persist across adapters, snapshot bytes, deduplicate and repair only known regions',async t=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'unippt-image-unit-')),sharp=sharpRuntime(),imagePath=path.join(root,'source.png');
 t.after(()=>fs.rmSync(root,{recursive:true,force:true}));await sharp({create:{width:200,height:100,channels:3,background:'#ffffff'}}).png().toFile(imagePath);
 const context={document:{size:page},revision:0,writeAccess:true},store=createStore(path.join(root,'jobs'),sharp),begin=await store.begin(imagePath,'session',async()=>context),jobId=begin.jobId;
 assert.equal(begin.modelInclusive,true);assert.equal(begin.source.width,200);assert.equal(begin.budgetPassed,null);
 const next=createStore(path.join(root,'jobs'),sharp);fs.unlinkSync(imagePath);
 const args={sessionId:'session',jobId,requestId:'first-layout-001',expectedRevision:0,rows:[row(),['photo','photo',0,0,10,10,null]]};
 const first=await next.plan(args);assert.deepEqual(first,await next.plan(args));assert.equal(first.command.compiledModules[0].objects[1].imageData.mimeType,'image/png');
 assert.ok(next.recover(args.requestId,{writeCommitted:true,revision:1,activeSlideId:'slide',nativeRendered:true,sessionId:'session'}).elapsedMs>=0);assert.equal(next.load(jobId).slideId,'slide');assert.equal(next.recover('unknown-request',{}),null);assert.throws(()=>next.recover(args.requestId,{sessionId:'other'}),/another/);
 assert.deepEqual(first,await next.plan(args));
 await assert.rejects(next.plan({...args,rows:[row('different')]}),/different/);
 await assert.rejects(next.plan({...args,sessionId:'other'}),/another/);
 await assert.rejects(next.plan({...args,requestId:'bad-repair-001',slideId:'foreign'}),/Only the slide/);
 await assert.rejects(next.plan({...args,requestId:'bad-repair-002',slideId:'slide',replaceRegions:['unknown']}),/Unknown/);
 const repair=await next.plan({...args,requestId:'region-repair-001',expectedRevision:1,slideId:'slide',rows:[row()],replaceRegions:['label']});
 assert.deepEqual(repair.command.compiledModules[0].replaceObjectIds,first.regions.label);assert.equal(repair.command.compiledModules[0].objects.length,1);
 assert.equal(repair.command.compiledModules[0].objects[0].id,first.regions.label[0]);
 next.committed(jobId,repair,{writeCommitted:true,revision:2,slideId:'slide',nativeRendered:false});
 assert.equal(next.previewed({sessionId:'other',slideId:'slide',expectedRevision:2},{nativeRendered:true}),null);
 assert.ok(next.previewed({sessionId:'session',slideId:'slide',expectedRevision:2},{nativeRendered:true}).firstNativeMs>=0);
 const diagnostic={revision:2,delivery:{passed:false},timings:{adapterTotalMs:12}};
 for(let i=0;i<2;i++)next.responded({jobId,requestId:'region-repair-001'},repair,diagnostic,Date.now());
 assert.equal(next.load(jobId).requestTimings.length,2,'Retry timings must not erase the original attempt');
 const review={matchesSource:false,unresolved:['Fine geometry mismatch']},finishArgs={sessionId:'session',jobId,expectedRevision:2,review};
 await assert.rejects(next.finish(finishArgs,async()=>({...context,revision:3})),/changed/);
 const finished=await next.finish(finishArgs,async()=>({...context,revision:2}));assert.equal(finished.budgetPassed,true);assert.equal(finished.visualFidelityPassed,false);assert.equal(finished.callerReviewedMatch,false);assert.equal(finished.independentFidelityCheck,false);
 assert.equal((await next.finish(finishArgs,async()=>({...context,revision:2}))).elapsedMs,finished.elapsedMs);
 await assert.rejects(next.plan({...args,requestId:'after-finish-001'}),/finished/);
});

test('different jobs with identical rows retain document-global IDs through the host and regional repair',async t=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'unippt-job-ids-')),sharp=sharpRuntime();t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
 const imagePath=path.join(root,'source.png');await sharp({create:{width:200,height:100,channels:3,background:'#ffffff'}}).png().toFile(imagePath);
 const store=createStore(path.join(root,'jobs'),sharp),context={document:{size:page},revision:0,writeAccess:true};
 const a=await store.begin(imagePath,'session',async()=>context),b=await store.begin(imagePath,'session',async()=>context);
 const plan=async(jobId,rid)=>store.plan({sessionId:'session',jobId,expectedRevision:0,requestId:rid,rows:[row()]});
 const one=await plan(a.jobId,'job-one-0001'),two=await plan(b.jobId,'job-two-0001');assert.notEqual(one.regions.label[0],two.regions.label[0]);
 require('../web/presentation_host.js');let deck={width:100,height:100,slides:[]};
 for(const p of [one,two]){const result=global.UniPptPresentationHost.applyChangeSet(deck,{operations:preparePatch(deck,sceneOperations(deck,p.command))});deck=result.deck;assert.ok(deck.slides.at(-1).objects.some(o=>o.id===p.regions.label[0]));}
 store.committed(b.jobId,two,{writeCommitted:true,revision:1,slideId:deck.slides.at(-1).id,nativeRendered:true});
 const repair=await store.plan({sessionId:'session',jobId:b.jobId,expectedRevision:1,requestId:'job-two-repair',slideId:deck.slides.at(-1).id,rows:[row()],replaceRegions:['label']});
 const untouched=JSON.stringify(deck.slides[0]);const result=global.UniPptPresentationHost.applyChangeSet(deck,{operations:preparePatch(deck,sceneOperations(deck,repair.command))});assert.equal(JSON.stringify(result.deck.slides[0]),untouched);assert.equal(result.deck.slides[1].objects.length,1);
});
test('composite scientific rows expand editable regions with explicit source crops',async()=>{
 const rows=[['net','network',0,0,100,80,{label:'F_{x}',title:'Network',symbol:'snowflake'}],['diff','diffusion',0,0,80,90,{title:'Diffusion',loop:'× T',symbol:'flame'}],['encoder','encoder',0,0,100,80,{label:'Encoder',symbol:'snowflake'}],['latent','tensor',0,0,30,80,'z_{0}'],['input','labelBox',0,0,100,30,'Input'],['card','card',0,0,90,90,{title:'Card',formula:'a_{b}',rgb:'Image',footer:'Condition',crops:[[0,0,10,10],[10,10,10,10]]}],['out','output',0,0,10,10,{crop:[20,20,10,10],label:'Output'}],['links','arrows',0,0,1,1,[[0,0,50,50],[50,50,100,50]]]];
 const c=await compileLayout(rows,source,page,async()=> 'PNG');assert.equal(c.objects.filter(o=>o.kind==='image').length,3);assert.equal(c.regions.links.length,4);assert.ok(c.regions.net.length>8);assert.equal(Object.keys(c.regions).length,rows.length);
 const [wing]=c.objects;assert.equal(wing.frame.width,50);assert.ok(c.objects.every(o=>Number.isFinite(o.frame.width)));
 const presetCard=rows.find(r=>r[0]==='card');await assert.rejects(compileLayout(Array.from({length:13},(_,i)=>['card'+i,...presetCard.slice(1)]),source,page,async()=> 'PNG'),/Too many/);
});
test('roundRect uses bounded physical radius and fan plane overlap is explicit',async()=>{
 const a=await compileLayout([['box','box',0,0,200,100,'roundRect'],['fan','fan',0,0,100,100,11,{planeWidth:28}]],source,page);
 assert.ok(a.objects[0].customGeometry);assert.equal(a.objects[1].frame.width,14);assert.ok(a.objects[2].frame.x<a.objects[1].frame.x+a.objects[1].frame.width);assert.equal(a.objects[1].style.stroke,'transparent');
});
test('image boundary rejects URLs, traversal IDs, directories and non-raster input',async t=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'unippt-image-boundary-'));t.after(()=>fs.rmSync(root,{recursive:true,force:true}));const store=createStore(path.join(root,'jobs'));
 for(const p of ['https://example.com/a.png','relative.png','\\\\server\\share\\a.png','C:\\image.png:stream'])await assert.rejects(store.begin(p,'s',()=>{}));
 assert.throws(()=>store.load('../token'),/Invalid/);await assert.rejects(store.begin(root,'s',()=>{}));
 const svg=path.join(root,'file.svg');fs.writeFileSync(svg,'<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>');await assert.rejects(store.begin(svg,'s',()=>{}),/Only single-frame/);
});

test('rich formula runs keep fonts, baselines and explicit sizes through recursive scaling',async()=>{
 const runs=[{text:'ℱ',fontFamily:'Cambria Math',fontSize:30},{text:'remove',baseline:'sub',fontSize:16}];
 const c=await compileLayout([['n','network',0,0,100,80,{label:runs},{labelFrame:[10,20,85,35]}]],source,page);
 const label=c.objects.find(o=>o.textRuns);assert.equal(label.textRuns[0].fontSize,15);assert.equal(label.textRuns[1].fontSize,8);assert.equal(label.frame.width,42.5);
 const ops=preparePatch({slides:[]},sceneOperations({slides:[]},{compiledModules:[{id:'x',objects:c.objects,replaceObjectIds:[]}]}));
 const native=ops[0].slide.objects.find(o=>o.text==='ℱremove');assert.equal(native.textParagraphs[0].runs[0].fontFamily,'Cambria Math');assert.equal(native.textParagraphs[0].runs[1].baseline,'sub');
 for(const invalid of [[{text:'x',script:'bad'}],[{text:'x',fontSize:Infinity}],[{text:'x',baseline:'invalid'}]])assert.throws(()=>textRuns(invalid));
 const shifted=await compileLayout([['shift','text',0,0,80,20,[{text:'x',baseline:'sub',baselineOffset:-47.5,fontSize:12}]]],source,page);assert.equal(shifted.objects[0].textRuns[0].baselineOffset,-47.5);assert.equal(shifted.objects[0].textRuns[0].fontSize,6);
 for(const baselineOffset of [NaN,101,-101,'-50'])assert.throws(()=>textRuns([{text:'0',baselineOffset}]));
});

test('explicit card frames and encoder panes retain measured geometry',async()=>{
 const c=await compileLayout([['c','card',0,0,100,90,{crops:[[0,0,10,10],[10,10,10,10]],photoFrames:[[12,22,31,32],[25,60,22,20]]}],['e','encoder',0,0,80,90,{label:'Encoder'},{panes:[[0,0,22,90,25],[15,12,22,65,25]]}]],source,page,async()=> 'PNG');
 const photos=c.objects.filter(o=>o.kind==='image');assert.deepEqual(photos[0].frame,{x:6,y:36,width:15.5,height:16,rotation:0});assert.equal(c.regions.e.length,3);
 await assert.rejects(compileLayout([['e','encoder',0,0,80,90,{label:'E'},{panes:[[0,0,-1,90,25]]}]],source,page));
});

test('composite text overrides preserve editable rich runs, local coordinates and unrelated geometry',async()=>{
 const r=['n','network',20,10,100,80,{label:'F_{x}',title:'Net'},{textOverrides:{label:{frame:[8,27,90,22],value:[{text:'F',fontSize:25},{text:'x',fontSize:12,baseline:'sub'}],options:{italic:true}}}}];
 const c=await compileLayout([r],source,page),label=c.objects.find(o=>o.name==='MCP module n-label');assert.deepEqual(label.frame,{x:14,y:43.5,width:45,height:11,rotation:0});assert.equal(label.textRuns[0].fontSize,12.5);assert.equal(label.textRuns[1].fontSize,6);assert.equal(label.textRuns[1].baseline,'sub');assert.equal(c.objects[0].frame.x,10);
 await assert.rejects(compileLayout([[...r.slice(0,7),{textOverrides:{wing1:{value:'not text'}}}]],source,page),/Unknown/);
 await assert.rejects(compileLayout([[...r.slice(0,7),{textOverrides:{label:{script:'bad'}}}]],source,page),/Invalid/);
});

test('tensor fit reconstructs independent transparent panes and rejects absent evidence',async()=>{
 const {fitTensorPixels}=require('./unippt_mcp_image.cjs'),w=60,h=95,rgba=new Uint8Array(w*h*4).fill(255),alpha=1/3;
 for(let y=0;y<h;y++)for(let x=0;x<w;x++){let n=0;for(let i=0;i<4;i++){const u=(x+.5-i*5)/43,yy=y+.5-(4-i);if(u>=0&&u<=1&&yy>=20*(1-u)&&yy<=90-20*u)n++;}const c=Math.round(255-126*(1-(1-alpha)**n));rgba[(y*w+x)*4]=rgba[(y*w+x)*4+1]=rgba[(y*w+x)*4+2]=c;}
 const fit=fitTensorPixels(rgba,w,h);assert.equal(fit.panes.length,4);assert.ok(fit.report.maskedMseAfter<fit.report.maskedMseBefore);assert.equal(fit.report.visualFidelityPassed,false);
 const c=await compileLayout([['t','tensor',0,0,w,h,'z_{0}',{sourceFit:true}]],source,page,null,null,null,async()=>fit);assert.equal(c.objects.filter(o=>o.kind==='shape').length,4);assert.equal(c.measurements.length,1);
 assert.throws(()=>fitTensorPixels(new Uint8Array(w*h*4).fill(255),w,h),/overlap/);assert.throws(()=>fitTensorPixels(rgba,400,400),/bounded/);
 await assert.rejects(compileLayout([['t','tensor',0,0,w,h,'',{sourceFit:true}]],source,page),/source capability/);
});

test('curved arrows have tangent-aligned solid editable heads, not dashed heads',async()=>{
 const c=await compileLayout([['c','path',0,0,100,100,'M 0 100 C 0 0 50 0 100 0',{headEnd:true,headStart:true,dash:'dash',stroke:'#00aa00'}]],source,page);
 assert.equal(c.objects.length,3);assert.equal(c.objects[0].style.strokeDash,'dash');assert.equal(c.objects[1].style.strokeDash,undefined);assert.equal(c.objects[2].style.fill,'#00aa00');
 await assert.rejects(compileLayout([['c','path',0,0,10,10,'M 0 0 L 10 10 Z',{headEnd:true}]],source,page),/open path/);
 await assert.rejects(compileLayout([['c','path',0,0,10,10,'M 0 0 L 0 0',{headEnd:true}]],source,page),/nonzero tangent/);
});

test('source-fit capability is bounded, reported and rejected without evidence',async()=>{
 const fake=async()=>({objects:[{frame:{x:1,y:2,width:20,height:50},points:[[0,10],[20,0],[20,40],[0,50]],style:{fill:'#eab498',opacity:.28}}],report:{layerCount:1,method:'test'}});
 const c=await compileLayout([['d','diffusion',0,0,80,90,{title:'Diffusion'},{sourceFit:true}]],source,page,null,fake);
 assert.equal(c.measurements[0].region,'d-planes');assert.equal(c.measurements[0].layerCount,1);assert.equal(c.objects[0].kind,'shape');
 await assert.rejects(compileLayout([['d','fan',0,0,80,90,8,{sourceFit:true}]],source,page),/source capability/);
 await assert.rejects(compileLayout([['d','fan',180,0,80,90,8,{sourceFit:true}]],source,page,null,fake),/bounded integer/);
 assert.throws(()=>compileLayout(Array.from({length:17},(_,i)=>['f'+i,'fan',0,0,80,90,8,{sourceFit:true}]),source,page,null,fake),/Source-fit budget/);
 const {fitFanPixels}=require('./unippt_mcp_image.cjs');assert.throws(()=>fitFanPixels(new Uint8Array(20*20*4).fill(255),20,20),/unsupported/);
});

test('encoder fitting is deterministic, bounded and cannot certify visual fidelity',()=>{
 const {fitEncoderPixels}=require('./unippt_mcp_image.cjs'),w=64,h=80,rgba=new Uint8Array(w*h*4).fill(255);
 for(let y=0;y<h;y++)for(let x=0;x<w;x++){let n=0;for(const [xx,yy,ww,hh,skew] of [[1,1,26,70,18],[18,8,26,58,18]]){const u=(x+.5-xx)/ww;if(u>=0&&u<=1&&y+.5>=yy+skew*(1-u)&&y+.5<=yy+hh-skew*u)n++;}rgba.set(n===2?[200,221,190,255]:n===1?[224,236,218,255]:[255,255,255,255],(y*w+x)*4);}
 const result=fitEncoderPixels(rgba,w,h,2);assert.equal(result.panes.length,2);assert.ok(result.report.maskedMseAfter<=result.report.maskedMseBefore);assert.equal(result.report.visualFidelityPassed,false);assert.deepEqual(result,fitEncoderPixels(rgba,w,h,2));
 assert.throws(()=>fitEncoderPixels(rgba,w,h,7),/2–6/);assert.throws(()=>fitEncoderPixels(new Uint8Array(w*h*4).fill(255),w,h,2),/overlap tones/);
});
