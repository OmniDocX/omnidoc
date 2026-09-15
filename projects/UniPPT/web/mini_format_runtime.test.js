const {test}=require('node:test'),assert=require('node:assert/strict');
const mini=require('./mini_format_runtime.js');
test('toolbar fits above selection and flips below near the viewport top',()=>{
  const bounds={left:200,top:150,right:900,bottom:700},size={width:330,height:76};
  assert.deepEqual(mini.placement({left:420,top:350,bottom:390},size,bounds),{x:420,y:264,width:330});
  assert.deepEqual(mini.placement({left:850,top:155,bottom:195},size,bounds),{x:562,y:205,width:330});
});
test('non-text selection does not expose text controls; captured ranges cannot cross objects',()=>{
  assert.equal(mini.canFormat({kind:'image',text:'alt'}),false);
  assert.equal(mini.canFormat({kind:'text',text:''}),true);
  assert.equal(mini.canFormat({kind:'shape',text:'Label'}),true);
  const object={id:'a',text:'hello'},range={objectId:'a',start:1,end:3};
  assert.equal(mini.rangeFor(range,object),range);
  assert.equal(mini.rangeFor({...range,objectId:'b'},object),null);
  assert.equal(mini.rangeFor({...range,end:9},object),null);
});
test('web toolbar uses editable range operations and ordinary undo transactions',()=>{
  const fs=require('node:fs'),path=require('node:path'),app=fs.readFileSync(path.join(__dirname,'app.js'),'utf8'),html=fs.readFileSync(path.join(__dirname,'index.html'),'utf8');
  assert.match(html,/mini_format_runtime.js/);assert.match(html,/role="toolbar" aria-label="选区快捷格式" hidden/);
  assert.match(app,/bindMiniFormatToolbar\(\)/);assert.match(app,/applySelectedTextProperty\(property, resolveValue\)/);
  assert.match(app,/updateRichTextRange\(ensureRichTextParagraphs\(target\), range.start, range.end, property, value\)/);
  assert.match(app,/miniFormatContext.range = \{ \.\.\.state.textSelection \}/);
  assert.match(app,/const selectedRange = miniFormatRange\(object\);[\s\S]*?runtime\?\.canFormat\(object\) && Boolean\(selectedRange\)/, 'object selection alone must never display the mini toolbar');
});
test('saved character range changes font and size without changing adjacent runs',()=>{
  const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),app=fs.readFileSync(path.join(__dirname,'app.js'),'utf8');
  const start=app.indexOf('function richTextRunsInRange'),end=app.indexOf('\nfunction textBoundaryAtOffset',start);
  const context={primaryCssFont:value=>String(value).split(',')[0]};
  vm.runInNewContext(app.slice(start,end)+'\nthis.change=updateRichTextRange;',context);
  const p=[{runs:[{text:'abcdef',fontSize:20,fontFamily:'Aptos',nativeFontFamily:'Aptos'}]}];
  context.change(p,1,3,'fontSize',32);context.change(p,1,3,'fontFamily','Arial');
  assert.deepEqual(JSON.parse(JSON.stringify(p[0].runs.map(r=>[r.text,r.fontSize,r.nativeFontFamily]))),[['a',20,'Aptos'],['bc',32,'Arial'],['def',20,'Aptos']]);
});
