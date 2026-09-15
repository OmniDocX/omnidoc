"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "color_picker_runtime.js"), "utf8");
const app = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const css = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");
const storage = new Map();
const context = {
  console,
  document: { addEventListener() {} },
  addEventListener() {},
  localStorage: {
    getItem(key) { return storage.get(key) || null; },
    setItem(key, value) { storage.set(key, value); },
  },
};
context.globalThis = context;
vm.runInNewContext(source, context);

const picker = context.UniPptColorPicker;
assert.ok(picker, "runtime must publish one shared color picker");
assert.equal(picker.normalizeHex("#abc"), "#AABBCC");
assert.equal(picker.rgbToHex(255, 127.6, 0), "#FF8000");
assert.deepEqual([...picker.hexToRgb("#4472C4")], [68, 114, 196]);
assert.equal(picker.tint("#000000", 0.8), "#CCCCCC");
assert.equal(picker.DEFAULT_THEME.length, 10, "UniDoc theme palette has ten columns");
assert.equal(picker.STANDARD.length, 10, "UniDoc standard palette has ten swatches");

assert.match(source, /最近使用的颜色/);
assert.match(source, /主题颜色/);
assert.match(source, /标准色/);
assert.match(source, /更多颜色…/);
assert.match(source, /new global\.EyeDropper\(\)\.open\(\)/);
assert.match(source, /cpa-sv/);
assert.match(source, /cpa-hue/);
assert.match(source, /cpa-rgb/);
assert.match(html, /color_picker_runtime\.js/);
assert.match(css, /\.color-pop \.cp-grid/);
assert.match(css, /\.color-pop\.cp-advanced/);

for (const label of ["字体颜色", "形状填充", "形状轮廓", "幻灯片背景", "墨迹颜色"]) {
  assert.ok(app.includes(label), `${label} must use the shared picker`);
}
assert.doesNotMatch(app, /openColorInput\(/, "color commands must not reopen browser-native pickers");
assert.match(app, /attachInput\(\$\("#propFill"\)/);
assert.match(app, /attachInput\(\$\("#propColor"\)/);
assert.match(app, /attachInput\(\$\("#drawColor"\)/);

console.log("global UniDoc-compatible color picker tests passed");
