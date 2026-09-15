"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const cssSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

const helperStart = appSource.indexOf("function pointerAngleDegrees");
const helperEnd = appSource.indexOf("\nfunction beginRotate", helperStart);
assert.ok(helperStart >= 0 && helperEnd > helperStart, "rotation helpers should be extractable");
const helperContext = {};
vm.runInNewContext(
  `${appSource.slice(helperStart, helperEnd)}\nthis.api = { pointerAngleDegrees, rotationFromPointerAngles };`,
  helperContext,
);

const geometryStart = appSource.indexOf("function multiplyAffine");
const geometryEnd = appSource.indexOf("\nfunction findObjectPath", geometryStart);
assert.ok(geometryStart >= 0 && geometryEnd > geometryStart, "selection geometry helpers should be extractable");
const geometryContext = {};
vm.runInNewContext(
  `${appSource.slice(geometryStart, geometryEnd)}\nthis.api = { selectionGeometryForPath };`,
  geometryContext,
);

const groupTargetStart = appSource.indexOf("function nextGroupedInteractionId");
const groupTargetEnd = appSource.indexOf("\nfunction groupedInteractionTargetId", groupTargetStart);
assert.ok(groupTargetStart >= 0 && groupTargetEnd > groupTargetStart, "group drill-down helper should be extractable");
const groupTargetContext = {};
vm.runInNewContext(
  `${appSource.slice(groupTargetStart, groupTargetEnd)}\nthis.api = { nextGroupedInteractionId };`,
  groupTargetContext,
);

test("rotation math crosses the 180 degree boundary and supports 15 degree snapping", () => {
  assert.equal(helperContext.api.pointerAngleDegrees(0, 0, 0, -10), -90);
  assert.equal(helperContext.api.rotationFromPointerAngles(350, 170, -170), 10);
  assert.equal(helperContext.api.rotationFromPointerAngles(7, 0, 17, true), 30);
});

test("selected objects expose eight resize handles, border move zones, and a rotation handle", () => {
  assert.match(appSource, /for \(const edge of \["n", "e", "s", "w"\]\)/);
  assert.match(appSource, /handles\.forEach\(\(dir\) =>/);
  assert.match(appSource, /"rotate-handle material-symbols-rounded", "rotate_right"/);
  assert.match(appSource, /beginRotate\(event, state\.selectedId\)/);
  assert.match(cssSource, /\.selection-frame\s*\{[^}]*pointer-events:none[^}]*border:/s);
  assert.match(cssSource, /\.selection-move-edge\s*\{[^}]*pointer-events:auto[^}]*cursor:move/s);
  assert.match(cssSource, /\.rotate-handle\s*\{[^}]*pointer-events:auto[^}]*cursor:grab/s);
});

test("selection controls retain a constant screen size as the slide zoom changes", () => {
  assert.match(appSource, /--selection-ui-scale", String\(1 \/ Math\.max\(\.01, state\.zoom\)\)/);
  assert.match(cssSource, /\.resize-handle\s*\{[^}]*scale\(var\(--selection-ui-scale,1\)\)/s);
  assert.match(cssSource, /\.rotate-handle\s*\{[^}]*scale\(var\(--selection-ui-scale,1\)\)/s);
});

test("live move and rotation updates target the main canvas rather than thumbnail clones", () => {
  const scopedLookups = appSource.match(/\$\("#slideStage"\)\?\.querySelector\(`\.scene-object\[data-id=/g) || [];
  assert.ok(scopedLookups.length >= 2, "move and rotate must update the stage object clone");
});

test("grouped descendants drill down from the outer group to the clicked child like PowerPoint", () => {
  const nextTarget = groupTargetContext.api.nextGroupedInteractionId;
  assert.equal(nextTarget("icon-a", ["outer"], null, []), "outer", "the first click should select the containing group");
  assert.equal(nextTarget("icon-a", ["outer"], "outer", ["outer"]), "icon-a", "the second click should select the clicked child");
  assert.equal(nextTarget("icon-b", ["outer"], "icon-a", ["outer", "icon-a"]), "icon-b", "group edit mode should allow selecting a sibling directly");
  assert.equal(nextTarget("icon-a", ["outer", "inner"], "outer", ["outer"]), "inner", "nested groups should advance one hierarchy level per click");
  assert.equal(nextTarget("icon-a", ["outer", "inner"], "inner", ["outer", "inner"]), "icon-a");
  assert.match(appSource, /const childSelectionGroupPath = \[\.\.\.selectionGroupPath, object\.id\]/);
  assert.match(appSource, /groupedInteractionTargetId\(object\.id, selectionGroupPath\)/);
  assert.match(appSource, /beginDrag\(event, interactionId\)/);
  assert.match(appSource, /openFormatPane\(interactionId\)/);
});

test("selection geometry composes child coordinates with the containing group", () => {
  const geometry = geometryContext.api.selectionGeometryForPath([
    { frame: { x: 227.26, y: 144.23, width: 825.48, height: 431.54, rotation: 0 } },
    { frame: { x: 0.28, y: 0, width: 44.37, height: 44.09, rotation: 0 } },
  ]);
  assert.ok(Math.abs(geometry.left - 227.54) < 0.001);
  assert.ok(Math.abs(geometry.top - 144.23) < 0.001);
  assert.equal(geometry.width, 44.37);
  assert.equal(geometry.height, 44.09);
  assert.ok(Math.abs(geometry.rotation) < 0.001);
});
