const {test} = require('node:test');
const assert = require('node:assert/strict');
const runtime = require('./image_semantic_module_runtime.js');
function fixture() {
  const width = 20, height = 20, pixels = new Uint8Array(width * height * 4).fill(255);
  for (let y = 2; y < 18; y++) for (let x = 3; x < 10; x++) {
    if (y >= 8 && y < 12) continue;
    pixels.set([36, 103, 156, 255], (y * width + x) * 4);
  }
  return runtime.measure(pixels, width, height, {minColorArea: 1, minRegionArea: 1});
}
test('source components remain evidence, not precommitted semantic shapes', () => {
  const measured = fixture(); assert.equal(measured.regions.length, 2);
  assert.equal(runtime.publicMeasurements(measured).regions[0].pixels, undefined);
  const args = runtime.build({objects: [{id: 'bar', kind: 'roundRect', regions: measured.regions.map(r => r.id), color: measured.palette[0].id}], texts: []}, measured);
  assert.equal(args.slide.objects.length, 1);
  assert.deepEqual(args.slide.objects[0].frame, {x: 3, y: 2, width: 7, height: 16});
});
test('convex hull restores an occluded native polygon from measured fragments', () => {
  const measured = fixture();
  const args = runtime.build({objects: [{id: 'panel', kind: 'hull', regions: measured.regions.map(r => r.id), color: measured.palette[0].id}], texts: []}, measured);
  assert.equal(args.slide.objects[0].points.length, 4);
  assert.ok(!JSON.stringify(args).includes('data:image'));
});
test('unknown source references and unbounded text are rejected', () => {
  assert.throws(() => runtime.build({objects: [{id: 'x', kind: 'rect', regions: ['invented'], color: 'color0'}], texts: []}, fixture()), /Unknown measured region/);
  assert.throws(() => runtime.build({objects: [], texts: [{id: 't', text: 'a', box: [0, 0, 10, 10], size: Infinity}]}, fixture()), /Invalid text size/);
});
test('native subscripts and explicit rotation survive semantic assembly', () => {
  const args = runtime.build({objects: [], texts: [{id: 't', text: 'F_{remove}', box: [0, 0, 20, 10], size: 9, rotation: -90}]}, fixture());
  assert.equal(args.slide.objects[0].frame.rotation, -90);
  assert.equal(args.slide.objects[0].textRuns[1].baseline, 'sub');
  assert.equal(args.slide.objects[0].textRuns[1].fontSize, 9 * .65);
});
test('pale colored bands and neutral feature layers are measured', () => {
  for (const rgb of [[238, 245, 252], [160, 160, 160]]) {
    const pixels = new Uint8Array(12 * 12 * 4);
    for (let i = 0; i < 144; i++) pixels.set([...rgb, 255], i * 4);
    const measured = runtime.measure(pixels, 12, 12);
    assert.equal(measured.regions.length, 1);
    assert.deepEqual(measured.palette[0].rgb, rgb);
  }
});
test('generic font families become explicit native typefaces', () => {
  const args = runtime.build({objects: [], texts: [{id: 't', text: 'x', box: [0, 0, 10, 10], size: 8, font: 'serif'}]}, fixture());
  assert.equal(args.slide.objects[0].textStyle.fontFamily, 'Times New Roman');
});
test('white-wash opacity is solved from observed source colors', () => {
  const measured = fixture();
  measured.palette.push({id:'pale', rgb:[146,179,206], fill:'#92b3ce'});
  const args = runtime.build({objects:[{id:'wash', kind:'wash', regions:[measured.regions[0].id], color:'pale', samples:[{base:'color0', observed:'pale'}]}], texts:[]}, measured);
  assert.equal(args.slide.objects[0].style.fill, '#ffffff');
  assert.ok(Math.abs(args.slide.objects[0].style.opacity - .5) < .005);
});
test('cubic curves remain native and reject magnified or invalid control points', () => {
  const curve = {id:'loop', start:[2,3], segments:[[[4,12],[16,12],[18,3]]], color:'#c09980', width:1};
  const args = runtime.build({objects:[], curves:[curve], texts:[]}, fixture());
  assert.match(args.slide.objects[0].path, /^M 0 0 C 2 9 14 9 16 0$/);
  assert.throws(() => runtime.build({objects:[], curves:[{...curve, start:[200,3]}], texts:[]}, fixture()), /original crop pixels/);
});
test('source overlap patches own their original color even in a mixed group',()=>{
 const m=fixture();m.palette.push({id:'wrong',rgb:[249,234,226],fill:'#f9eae2'});
 const s=runtime.build({objects:[{id:'patch',kind:'patches',color:'wrong',regions:m.regions.map(r=>r.id)}],texts:[]},m).slide.objects;
 assert.equal(s.length,2);assert.equal(s[0].style.fill,'#24679c');assert.match(s[0].path,/ Z$/);
});
test('duplicate cubic start notation is normalized only when unambiguous',()=>{
 const c={id:'c',start:[1,1],segments:[[[1,1],[2,5],[8,5],[9,1]]],color:'#123456',width:1};
 assert.match(runtime.build({objects:[],texts:[],curves:[c]},fixture()).slide.objects[0].path,/C 1 4 7 4 8 0$/);
 assert.equal(c.segments[0].length,4);
});
test('symmetric fan reconstructs separate transparent sheets from visible source islands',()=>{
 const m={width:100,height:100,palette:[{id:'light',rgb:[249,234,226],fill:'#f9eae2',area:5000},{id:'overlap',rgb:[245,219,205],fill:'#f5dbcd',area:4000}],regions:[]};
 const add=(id,box,area,pixels)=>m.regions.push({id,box,area,pixels,colorId:'light',fill:'#f9eae2'});
 add('left',[10,5,20,90],1000,[2510,7429]);add('right',[60,5,20,90],1000,[2560,7479]);
 add('a',[30,15,5,10],20,[]);add('b',[35,25,5,10],20,[]);add('c',[50,65,5,10],20,[]);add('d',[55,75,5,10],20,[]);
 const r=runtime.recoverFan({id:'fan',color:'light',regions:m.regions.map(r=>r.id)},m);
 assert.equal(r.objects.length,6);assert.equal(r.report.shear,20);assert.ok(r.report.alpha>.27&&r.report.alpha<.29);
 assert.ok(r.objects.every(o=>o.points.length===4&&o.style.opacity===r.report.alpha&&!o.path));
 assert.throws(()=>runtime.recoverFan({id:'bad',color:'light',regions:['a','b','c','d']},m),/source-measured/);
});
test('flat quadratic control/end notation becomes an equivalent native cubic',()=>{
 const c=runtime.normalizeCurve({start:[0,0],segments:[[3,6,9,0]]});
 assert.deepEqual(c.segments,[[[2,4],[5,4],[9,0]]]);
});
