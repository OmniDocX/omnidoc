'use strict';
const test=require('node:test'),assert=require('node:assert/strict');
const {baselineRuns,paintNativeBaselines}=require('./native_baseline_preview.cjs');
const xml='<p:sp><p:nvSpPr><p:cNvPr id="2" name="test"/></p:nvSpPr><p:spPr><a:xfrm><a:off x="95250" y="190500"/><a:ext cx="952500" cy="285750"/></a:xfrm></p:spPr><p:txBody><a:p><a:r><a:rPr sz="1200" baseline="-35000"><a:latin typeface="Times New Roman"/></a:rPr><a:t>x&amp;y</a:t></a:r></a:p></p:txBody></p:sp>';
test('native DrawingML offsets are parsed without relying on dropped imported run properties',()=>{
 const p=baselineRuns(xml);assert.equal(p.unsupported.length,0);assert.deepEqual(p.runs[0].box,[10,20,100,30]);assert.equal(p.runs[0].fontPx,16);assert.equal(p.runs[0].text,'x&y');assert.equal(p.runs[0].baseline,-35000);
 assert.equal(baselineRuns(xml.replace('<a:xfrm>','<a:xfrm rot="900000">')).unsupported.length,1);
 assert.equal(baselineRuns(xml.replace('baseline="-35000"','baseline="0"')).runs.length,0);assert.throws(()=>baselineRuns('<!DOCTYPE bad>'+xml),/entities/);
});
test('canvas projection moves only the exact run, tracks failures and always restores painter',async()=>{
 class Context {constructor(){this.font='italic 16px "Times New Roman"';this.calls=[];}getTransform(){return {a:1,b:0,c:0,d:1,e:0,f:0};}fillText(...args){this.calls.push(args);}}
 const original=Context.prototype.fillText,c=new Context(),r=baselineRuns(xml).runs;
 const good=await paintNativeBaselines(Context,r,async()=>{c.fillText('other',12,40);c.fillText('x&y',12,40);return 'png';});assert.equal(good.report.painted,1);assert.equal(c.calls[0][2],40);assert.equal(c.calls[1][2],45.6);assert.equal(Context.prototype.fillText,original);
 const empty=await paintNativeBaselines(Context,r,async()=>{});assert.equal(empty.report.unmatched.length,1);
 await assert.rejects(paintNativeBaselines(Context,r,async()=>{throw Error('draw failed');}),/draw failed/);assert.equal(Context.prototype.fillText,original);
});
