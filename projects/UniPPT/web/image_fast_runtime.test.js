'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
require('./presentation_host.js'); require('./image_reconstruction_runtime.js'); require('./image_native_tracer.js'); require('./image_precision_runtime.js');
const fast = require('./image_fast_runtime.js');
const anchors = [{id: 'a0', readings: ['Title'], bbox: [100, 100, 800, 250], rotation: -90}];
const fixture = () => ({attachment: {name: 'arbitrary.png', width: 100, height: 100, dataUrl: 'data:image/png;base64,AAA'},
  anchors, photos: [], pixels: new Uint8ClampedArray(40000).fill(255), renderPreview: async () => 'test-preview'});
test('sparse corrections preserve omitted OCR and reject unknown/delete-without-reason', () => {
  assert.equal(fast.mergeCorrections(anchors, {}).corrections.a0.text, 'Title');
  assert.throws(() => fast.mergeCorrections(anchors, {a9: {text: 'bad'}}));
  assert.throws(() => fast.mergeCorrections(anchors, {a0: {text: null}}));
  assert.equal(fast.mergeCorrections(anchors, [{anchorId: 'a0', text: 'Correct'}]).corrections.a0.text, 'Correct');
  assert.equal(fast.mergeCorrections(anchors, []).corrections.a0.text, 'Title');
  assert.equal(fast.mergeCorrections(anchors, {a0: 'Correct'}).corrections.a0.text, 'Correct');
  assert.throws(() => fast.mergeCorrections(anchors, [{id: 'a0', text: 'A'}, {id: 'a0', text: 'B'}]));
});
test('fast round requests no thinking, only one model call, and retains native rotation', async () => {
  let count = 0;
  const result = await fast.prepare({...fixture(), completion: async request => {
    count++; assert.equal(request.thinking, false); assert.ok(request.timeoutMs <= 18000);
    return {content: '{"corrections":{"a0":{"text":"𝓕_{x}"}}}'};
  }});
  assert.equal(count, 1); assert.equal(result.report.modelCompleted, true); assert.equal(result.review.passed, false);
  assert.equal(result.report.history.length, 0); assert.match(result.report.visualReview, /browser/);
  const text = result.args.slide.objects.find(o => o.kind === 'text');
  assert.equal(text.frame.rotation, -90); assert.ok(text.textRuns.some(r => r.baseline === 'sub'));
});
test('provider failure returns an honestly labelled OCR draft with no hidden retry', async () => {
  const result = await fast.prepare({...fixture(), completion: async () => {throw new Error('provider timeout');}});
  assert.equal(result.report.modelCompleted, false); assert.equal(result.report.modelCalls, 1);
  assert.match(result.report.warnings.join(' '), /OCR 草稿/); assert.equal(result.inventory.labels[0].text, 'Title');
});
test('expired budget skips model; cancellation never returns a late candidate', async () => {
  let called = false;
  const result = await fast.prepare({...fixture(), deadline: Date.now(), completion: async () => {called = true;}});
  assert.equal(called, false); assert.equal(result.report.modelCalls, 0);
  const controller = new AbortController(); controller.abort(new Error('cancelled'));
  await assert.rejects(fast.prepare({...fixture(), signal: controller.signal, deadline: Date.now()}), /cancelled/);
});
