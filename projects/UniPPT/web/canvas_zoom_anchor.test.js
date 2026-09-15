"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const helperStart = appSource.indexOf("function canvasZoomCorrection");
const helperEnd = appSource.indexOf("\nfunction adjustZoom", helperStart);
assert.ok(helperStart >= 0 && helperEnd > helperStart, "cursor-anchor correction should be extractable");

const context = {};
vm.runInNewContext(
  `${appSource.slice(helperStart, helperEnd)}\nthis.api = { canvasZoomCorrection };`,
  context,
);

const anchor = { clientX: 300, clientY: 200 };
const before = { left: 100, top: 80 };
const after = { left: 70, top: 60 };
const previousZoom = .5;
const nextZoom = 1;
const correction = context.api.canvasZoomCorrection(anchor, before, after, previousZoom, nextZoom);
assert.deepEqual(JSON.parse(JSON.stringify(correction)), { left: 170, top: 100 });
assert.equal(after.left - correction.left + ((anchor.clientX - before.left) / previousZoom) * nextZoom, anchor.clientX, "horizontal scroll compensation must keep the slide point beneath the cursor");
assert.equal(after.top - correction.top + ((anchor.clientY - before.top) / previousZoom) * nextZoom, anchor.clientY, "vertical scroll compensation must keep the slide point beneath the cursor");

assert.match(appSource, /adjustZoom\(event\.deltaY < 0 \? ZOOM_STEP : -ZOOM_STEP, \{[\s\S]*?clientX: event\.clientX[\s\S]*?clientY: event\.clientY/, "Ctrl+wheel must forward its exact pointer position");
assert.match(appSource, /viewport\.scrollLeft \+= correction\.left[\s\S]*?viewport\.scrollTop \+= correction\.top/, "manual zoom must apply two-axis cursor-anchor compensation");
assert.match(appSource, /#canvasViewport"\)\.addEventListener\("pointermove", rememberCanvasPointer\)/, "zoom buttons and slider must reuse the latest canvas pointer position");

console.log("canvas cursor-centered zoom tests passed");
