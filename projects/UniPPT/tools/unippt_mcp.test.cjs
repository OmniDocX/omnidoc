'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const {createBroker} = require('./unippt_mcp_broker.cjs');
const {create, validatePatch, publicValue, textPatch, preparePatch} = require('../web/mcp_bridge.js');
test('broker authenticates, scopes, deduplicates, cancels and expires commands', async t => {
  let clock = 1000;
  const broker = createBroker({token: 'test-token', port: 0, now: () => clock});
  await new Promise(resolve => broker.server.listen(0, '127.0.0.1', resolve));
  t.after(() => broker.server.close());
  const url = 'http://127.0.0.1:' + broker.server.address().port;
  const call = async (route, body = {}, headers) => {
    const r = await fetch(url + route, {method: 'POST', headers: {'Content-Type': 'application/json', ...(headers || (route.startsWith('/browser/') ? {Origin: 'http://127.0.0.1:8141'} : {Authorization: 'Bearer test-token'}))}, body: JSON.stringify(body)});
    return {status: r.status, ...await r.json()};
  };
  assert.equal((await call('/agent/sessions', {}, {})).status, 403);
  assert.equal((await call('/browser/connect', {}, {Origin: 'https://evil.example'})).status, 403);
  assert.equal((await call('/agent/sessions', {}, {Origin: 'http://127.0.0.1:8141', Authorization: 'Bearer test-token'})).status, 403);
  const auth = await call('/browser/connect', {title: 'Only test doc', writeAccess: true});
  const list = await call('/agent/sessions'); assert.equal(list.sessions.length, 1); assert.equal(list.sessions[0].secret, undefined);
  const submission = {sessionId: auth.sessionId, requestId: 'test-command-001', method: 'apply_patch', args: {expectedRevision: 0, operations: []}};
  assert.equal((await call('/agent/submit', submission)).state, 'pending');
  assert.equal((await call('/agent/submit', submission)).state, 'pending'); assert.equal(broker.commands.size, 1);
  assert.equal((await call('/agent/submit', {...submission, args: {different: true}})).status, 409);
  assert.equal((await call('/browser/poll', {sessionId: auth.sessionId, secret: 'wrong'})).status, 410);
  const claimed = await call('/browser/poll', auth); assert.equal(claimed.command.requestId, submission.requestId);
  assert.equal((await call('/browser/poll', auth)).command, null);
  await call('/browser/result', {...auth, requestId: submission.requestId, result: {revision: 1}});
  assert.equal((await call('/agent/submit', submission)).state, 'completed');
  const readOnly = await call('/browser/connect', {writeAccess: false});
  assert.equal((await call('/agent/submit', {...submission, sessionId: readOnly.sessionId, requestId: 'readonly-001'})).status, 403);
  assert.equal((await call('/agent/submit', {...submission, method:'apply_scene',sessionId:readOnly.sessionId,requestId:'readonly-modules-001'})).status,403);
  const moduleWrite={...submission,method:'apply_scene',requestId:'module-retry-001',args:{expectedRevision:1,compiledModules:[],compileMs:1}};
  assert.equal((await call('/agent/submit',moduleWrite)).state,'pending');
  assert.equal((await call('/agent/submit',{...moduleWrite,args:{...moduleWrite.args,compileMs:7}})).state,'pending');
  assert.equal((await call('/agent/submit',{...moduleWrite,args:{...moduleWrite.args,expectedRevision:2}})).status,409);
  await call('/agent/cancel',{requestId:moduleWrite.requestId});
  await call('/agent/submit', {...submission, requestId: 'cancel-me-001'});
  assert.equal((await call('/agent/cancel', {requestId: 'cancel-me-001'})).state, 'cancelled');
  assert.equal((await call('/browser/poll', auth)).command, null);
  await call('/agent/submit', {...submission, requestId: 'expire-me-001'});
  clock += 61000;
  assert.equal((await call('/agent/status', {requestId: 'expire-me-001'})).state, 'expired');
  assert.equal((await call('/agent/sessions')).sessions.length, 2);
  // Agent reads do not extend the browser heartbeat grace period.
  clock += 240001;
  assert.equal((await call('/agent/sessions')).sessions.length, 0);
});
test('browser patch boundary rejects imports, scripts, URLs and prototype pollution', () => {
  assert.doesNotThrow(() => validatePatch([{op: 'updateObject', objectId: 'x', patch: {frame: {rotation: -90}, textRuns: [{text: 'x', baseline: 'sub'}]}}]));
  assert.throws(() => validatePatch([{op: 'runScript', code: 'alert(1)'}]));
  assert.throws(() => validatePatch([{op: 'updateObject', patch: {imageUrl: 'http://private/'}}]));
  assert.throws(() => validatePatch(JSON.parse('[{"op":"updateObject","patch":{"__proto__":{}}}]')));
  assert.equal(publicValue({image: 'data:image/png;base64,xxx'}).image.omitted, true);
});
test('MCP rich-text edits compile native paragraph runs and style changes reach existing runs', () => {
  const old = {id: 't', textStyle: {fontSize: 20, fontFamily: 'Arial'}, textParagraphs: [{align: 'left', runs: [{text: 'x', fontSize: 13, baseline: 'sub', color: '#111111'}]}]};
  const styled = textPatch(old, {textStyle: {fontSize: 30, color: '#ff0000', fontFamily: 'Cambria Math'}});
  assert.equal(styled.textParagraphs[0].runs[0].fontSize, 19.5);
  assert.equal(styled.textParagraphs[0].runs[0].color, '#ff0000');
  const rich = textPatch(old, {textRuns: [{text: 'F'}, {text: 'x', baseline: 'sub'}]});
  assert.equal(rich.text, 'Fx'); assert.equal(rich.textParagraphs[0].runs[1].baseline, 'sub'); assert.equal(rich.textRuns, undefined);
  assert.throws(() => textPatch(old, {text: 'wrong', textRuns: [{text: 'F'}]}));
  assert.throws(() => preparePatch({slides: [{id: 's', objects: [old]}]}, [{op: 'updateObject', objectId: 't', patch: {id: 'other'}}]));
  assert.equal(old.textParagraphs[0].runs[0].fontSize, 13);
  const batch = preparePatch({slides: [{id: 's', objects: [old]}]}, [
    {op: 'updateObject', objectId: 't', patch: {textStyle: {color: '#ff0000'}}},
    {op: 'updateObject', objectId: 't', patch: {textRuns: [{text: 'Fx'}]}},
  ]);
  assert.equal(batch[1].patch.textParagraphs[0].runs[0].color, '#ff0000');
});
test('MCP native slide insertion normalizes rich text and only bounded PNG material', () => {
  const png = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg==';
  const operations = [{op:'insertSlide',slide:{objects:[
    {kind:'text', textRuns:[{text:'F'},{text:'x',baseline:'sub'}]},
    {kind:'image',imageData:{mimeType:'image/png',base64:png}},
  ]}}];
  const result = preparePatch({slides:[]},operations)[0].slide.objects;
  assert.equal(result[0].textParagraphs[0].runs[1].baseline,'sub');
  assert.equal(result[1].asset,'data:image/png;base64,'+png);
  assert.equal(result[1].imageData,undefined);
  assert.ok(operations[0].slide.objects[1].imageData);
  for (const bad of [
    {kind:'shape',imageData:{mimeType:'image/png',base64:png}},
    {kind:'image',imageData:{mimeType:'image/svg+xml',base64:png}},
    {kind:'image',imageData:{mimeType:'image/png',base64:'AAAA'}},
    {kind:'image',asset:'file:///secret',imageData:{mimeType:'image/png',base64:png}},
  ]) assert.throws(()=>preparePatch({slides:[]},[{op:'insertSlide',slide:{objects:[bad]}}]));
  const giant = Buffer.from(png,'base64'); giant.writeUInt32BE(4097,16);
  assert.throws(()=>preparePatch({slides:[]},[{op:'addObject',slideId:'s',object:{kind:'image',imageData:{mimeType:'image/png',base64:giant.toString('base64')}}}]),/尺寸/);
});
test('browser host bridge: schema, original image, atomic revisions and undo isolation', async t => {
  require('../web/presentation_host.js');
  const runtime = global.UniPptPresentationHost;
  let deck = runtime.compileNativeImageDeck({sourceSize: {width: 100, height: 100}, slide: {name: 'test', objects: [{kind: 'text', name: 't', text: 'Original', frame: {x: 5, y: 5, width: 85, height: 25}}], sourceCrops: [], verification: {labels: ['Original'], rotatedTextCount: 0, diagram: false}}});
  const objectId = deck.slides[0].objects[0].id; let hostRevision = 0; const history = [];
  const host = runtime.create({getDeck: () => deck, getRevision: () => hostRevision, getSelection: () => ({slideId: deck.slides[0].id}),
    commit(next) {history.push(deck); deck = next; hostRevision++;}});
  const oldFetch = global.fetch, oldDocument = global.document, oldConfirm = global.confirm, oldLocation = global.location;
  global.location = {origin:'http://127.0.0.1:8141'};
  const queue = [], waiting = new Map(); let requestOrdinal = 0, renderCount=0, renderFails=false;
  global.document = {activeElement: {tagName: 'BODY'}}; global.confirm = () => true;
  global.fetch = async (url, options) => {
    if(url==='/api/mcp/render-bundle'){renderCount++;return {ok:!renderFails,text:async()=>renderFails?'Renderer unavailable':JSON.stringify({pngBase64:'AAAA',pptxBase64:'UEs=',audit:{shapes:1},nativeRendered:true,visualFidelityPassed:false,timings:{serverTotalMs:5}})};}
    const body = JSON.parse(options.body), route = url.split('/').at(-1);
    const result = route === 'connect' ? {sessionId: 'test-session', secret: 'test-secret'} : route === 'poll' ? {command: queue.shift() || null} : {ok: true};
    if (route === 'result') { const waiter = waiting.get(body.requestId); waiting.delete(body.requestId); waiter(body); }
    return {ok: true, json: async () => result};
  };
  const bridge = create({host, getDeck: () => deck, getSourceImage: () => ({name: 'source.png', dataUrl: 'data:image/png;base64,AAAA', width: 100, height: 100}),
    undo() {deck = history.pop(); hostRevision++; host._emit('change', {});}});
  t.after(async () => {await bridge.disconnect(); global.fetch = oldFetch; global.document = oldDocument; global.confirm = oldConfirm; global.location = oldLocation;});
  const command = (method, args = {}) => new Promise(resolve => {
    const requestId = 'browser-test-' + (++requestOrdinal); waiting.set(requestId, resolve); queue.push({method, args, requestId, deadline: Date.now() + 20000});
  });
  await assert.rejects(bridge.connect({isTrusted: false}), /真实点击/);
  await bridge.connect({isTrusted: true});
  const prepared=(await command('prepare',{slideId:deck.slides[0].id})).result;
  assert.equal(prepared.revision,0);assert.ok(prepared.moduleSchema.types.includes('snowflake'));assert.equal(prepared.imageBase64,'AAAA');assert.equal(prepared.writeAccess,true);
  assert.ok((await command('get_edit_schema')).result.schema.properties.operations.items.oneOf.length);
  assert.equal((await command('source_image', {slideId: deck.slides[0].id})).result.imageBase64, 'AAAA');
  const patch = {expectedRevision: 0, operations: [{op: 'updateText', objectId, text: 'Changed'}]};
  assert.equal((await command('apply_patch', patch)).result.revision, 1);
  assert.equal(deck.slides[0].objects[0].text, 'Changed');
  assert.match((await command('apply_patch', patch)).error, /REVISION_CONFLICT/);
  assert.equal((await command('undo', {expectedRevision: 1})).result.revision, 2);
  assert.equal(deck.slides[0].objects[0].text, 'Original');
  const bad = await command('apply_patch', {expectedRevision: 2, operations: [{op: 'updateText', objectId, text: 'Must roll back'}, {op: 'removeObject', objectId: 'missing-id'}]});
  assert.ok(bad.error); assert.equal(deck.slides[0].objects[0].text, 'Original'); assert.equal(history.length, 0);
  const {compileModules}=require('../web/mcp_scene.js');
  const bundle=await command('apply_scene',{expectedRevision:2,compiledModules:compileModules([{id:'first',kind:'layers',count:4},{id:'flow',kind:'arrow'}])});
  assert.equal(bundle.result.revision,3);assert.equal(bundle.result.writeCommitted,true);assert.ok(bundle.result.pngBase64);assert.ok(bundle.result.pptxBase64);assert.equal(bundle.result.moduleMap[0].objectIds.length,4);assert.equal(deck.slides.length,2);assert.equal(renderCount,1);
  assert.equal((await command('preview',{expectedRevision:3,slideId:bundle.result.activeSlideId})).result.cacheHit,true);assert.equal(renderCount,1);
  const repair=await command('apply_scene',{expectedRevision:3,slideId:bundle.result.activeSlideId,compiledModules:compileModules([{id:'first',kind:'layers',count:2,replaceObjectIds:bundle.result.moduleMap[0].objectIds}])});
  assert.equal(repair.result.revision,4);assert.equal(deck.slides[1].objects.length,4);assert.equal(renderCount,2);assert.equal(deck.slides[0].objects[0].text,'Original');
  renderFails=true;
  const failed=await command('apply_scene',{expectedRevision:4,compiledModules:compileModules([{id:'new',kind:'text',text:'Still committed'}])});
  assert.equal(failed.result.writeCommitted,true);assert.equal(failed.result.revision,5);assert.match(failed.result.previewError,/unavailable/);assert.equal(deck.slides.length,3);
  renderFails=false;
  const combinedPatch=await command('apply_patch',{expectedRevision:5,preview:true,operations:[{op:'updateText',objectId:'mcp-new-0',text:'Patched and rendered'}]});
  assert.equal(combinedPatch.result.revision,6);assert.equal(combinedPatch.result.nativeRendered,true);assert.ok(combinedPatch.result.pptxBase64);assert.equal(combinedPatch.result.moduleMap,undefined);
  host._emit('change', {source: 'editor'});
  assert.match((await command('undo', {expectedRevision: 7})).error, /其他编辑/);
  host._emit('load', {});
});
