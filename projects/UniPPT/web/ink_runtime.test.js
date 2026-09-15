"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

function runtime() {
  const context = { globalThis: {} };
  vm.runInNewContext(fs.readFileSync(`${__dirname}/ink_runtime.js`, "utf8"), context);
  return context.globalThis.UniPptInk;
}

test("ink path is compact, editable and hit-testable", () => {
  const ink = runtime();
  const points = [[10, 10, .5], [20, 20, .5], [30, 30, .5], [40, 40, .5]];
  const simplified = ink.simplify(points, 1);
  assert.equal(simplified.length, 2);
  const path = ink.pointsToPath(simplified);
  assert.equal(path, "M10 10 L40 40");
  assert.deepEqual(Array.from(ink.pathToPoints(path), p => Array.from(p).slice(0, 2)), [[10, 10], [40, 40]]);
  assert.equal(ink.hitTest(ink.pathToPoints(path), [25, 26], 2), true);
  assert.equal(ink.hitTest(ink.pathToPoints(path), [25, 35], 2), false);
});

test("ink recognizer maps common strokes to native PowerPoint shapes", () => {
  const ink = runtime();
  assert.equal(ink.recognizeShape([[0, 0], [100, 2], [200, 0]]).kind, "line");
  const ellipse = Array.from({ length: 49 }, (_, index) => {
    const angle = index / 48 * Math.PI * 2;
    return [100 + Math.cos(angle) * 80, 80 + Math.sin(angle) * 50];
  });
  assert.equal(ink.recognizeShape(ellipse).kind, "ellipse");
  assert.equal(ink.recognizeShape([[0, 0], [100, 0], [100, 60], [0, 60], [0, 0]]).kind, "rectangle");
});

test("draw ribbon exposes real commands rather than placeholder labels", () => {
  const html = fs.readFileSync(`${__dirname}/index.html`, "utf8");
  for (const id of ["drawSelect", "drawPen", "drawHighlighter", "drawEraser", "convertInkShape", "clearSlideInk"]) {
    assert.match(html, new RegExp(`id="${id}"`));
  }
  assert.match(html, /render:\s*\{[\s\S]*?\{\{asset:\/ink_runtime\.js\}\}/);
  assert.doesNotMatch(html, /ribbon-placeholder" data-panel="draw"/);
});

test("freehand strokes only hit near their visible path and drag with one pointer", () => {
  const app = fs.readFileSync(`${__dirname}/app.js`, "utf8");
  const css = fs.readFileSync(`${__dirname}/style.css`, "utf8");
  assert.match(app, /hitPath\.classList\.add\("ink-hit-path"\)/);
  assert.match(app, /Math\.max\(12, Number\(object\.style\.strokeWidth \|\| 0\) \+ 8\)/);
  assert.match(css, /\.scene-object\.is-ink-object\s*\{[^}]*pointer-events\s*:\s*none/s);
  assert.match(css, /\.ink-hit-path\s*\{[^}]*pointer-events\s*:\s*stroke/s);
  assert.match(app, /const pointerId = event\.pointerId[\s\S]*e\.pointerId !== pointerId[\s\S]*pointercancel/);
});
