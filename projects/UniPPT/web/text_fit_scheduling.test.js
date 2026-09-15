const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const source = fs.readFileSync(require('node:path').join(__dirname, 'app.js'), 'utf8');
const body = source.slice(source.indexOf('function fitAutoTextInContainer('), source.indexOf('function fitVerticalNoWrapText('));
function fixture() {
  const frames = [], fitted = [];
  const sandbox = {requestAnimationFrame: fn => frames.push(fn), fitTextToShape: n => fitted.push(n), fitVerticalNoWrapText: n => fitted.push(n)};
  vm.runInNewContext(body, sandbox);
  const node = {isConnected: false, querySelectorAll: selector => selector.includes('auto-size') ? ['text'] : []};
  return {frames, fitted, node, fit: sandbox.fitAutoTextInContainer};
}
test('discarded text stages cannot schedule an infinite animation-frame retry', () => {
  const f = fixture(); f.fit(f.node); assert.equal(f.frames.length, 1);
  f.frames.shift()(); assert.equal(f.frames.length, 0); assert.equal(f.fitted.length, 0);
});
test('new thumbnail gets one attachment retry and connected text still fits', () => {
  const f = fixture(); f.fit(f.node); f.node.isConnected = true; f.frames.shift()();
  assert.deepEqual(f.fitted, ['text']); assert.equal(f.frames.length, 0);
  f.fit(f.node); assert.deepEqual(f.fitted, ['text', 'text']);
});
