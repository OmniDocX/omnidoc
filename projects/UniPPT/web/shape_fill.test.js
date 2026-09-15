"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const start = source.indexOf("function imageCropMetrics");
const end = source.indexOf("\nfunction isLinearGeometry", start);
assert.ok(start >= 0 && end > start, "picture-fill metric helpers should be extractable");
const context = { clamp: (value, min, max) => Math.max(min, Math.min(max, Number(value))) };
vm.runInNewContext(`${source.slice(start, end)}\nthis.imageFillMetrics = imageFillMetrics;`, context);

const metrics = context.imageFillMetrics(
  { left: 0, top: 0, right: 0, bottom: 0 },
  { left: -0.15048, top: -0.15048, right: -0.15048, bottom: -0.15048 },
  100,
  100,
);
assert.ok(Math.abs(metrics.x + 15.048) < 1e-9);
assert.ok(Math.abs(metrics.y + 15.048) < 1e-9);
assert.ok(Math.abs(metrics.width - 130.096) < 1e-9);
assert.ok(Math.abs(metrics.height - 130.096) < 1e-9);

assert.match(source, /image\.className = "shape-fill-image"/, "stretched fills must use an img element, not a CSS data URL");
assert.match(source, /node\.prepend\(image\)/, "the fill bitmap must remain behind text and controls");
assert.match(source, /object\.shapeFillAsset && !tiledImageFill \? "transparent"/, "large data URLs must not be repeated through the CSS fill variable");
console.log("shape fill tests passed");
