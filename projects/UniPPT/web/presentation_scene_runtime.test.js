"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const runtimeSource = fs.readFileSync(path.join(__dirname, "presentation_scene_runtime.js"), "utf8");
const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const indexSource = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const losslessSource = fs.readFileSync(
  path.join(__dirname, "..", "crates", "unippt-server", "src", "lossless_html.rs"),
  "utf8",
);

function runtime() {
  const context = {};
  vm.runInNewContext(runtimeSource, context);
  return context.UniPptPresentationScene;
}

test("editor presenter and lossless HTML load the same scene runtime", () => {
  assert.match(indexSource, /core:\s*\{[\s\S]*?\{\{asset:\/presentation_scene_runtime\.js\}\}/);
  assert.match(
    losslessSource,
    /include_str!\("\.\.\/\.\.\/\.\.\/web\/presentation_scene_runtime\.js"\)/,
    "standalone HTML must embed the exact browser scene runtime",
  );
  assert.match(
    appSource,
    /UniPptPresentationScene;[\s\S]{0,500}runtime\.renderSlide\(slide, container/,
    "the editor presenter must delegate its read-only canvas to the shared renderer",
  );
  assert.match(appSource, /renderReadOnlySlide\(slide, stage, mediaContext\)/);
  assert.match(
    losslessSource,
    /UniPptPresentationScene\.renderSlide\(sl,s,\{clear:false,applyBackground:false/,
    "the emitted standalone player draw path must delegate to the shared renderer",
  );
});

test("shared renderer preserves master, layout, and slide stacking order", () => {
  const scene = runtime();
  const layers = scene.orderedLayerObjects({
    masterObjects: [{ id: "master-a" }, { id: "master-b" }],
    layoutObjects: [{ id: "layout-a" }],
    objects: [{ id: "slide-a" }, { id: "slide-b" }],
  });
  assert.deepEqual(
    JSON.parse(JSON.stringify(layers.map(({ layer, object, z }) => ({ layer, id: object.id, z })))),
    [
      { layer: "master", id: "master-a", z: 0 },
      { layer: "master", id: "master-b", z: 1 },
      { layer: "layout", id: "layout-a", z: 2 },
      { layer: "slide", id: "slide-a", z: 3 },
      { layer: "slide", id: "slide-b", z: 4 },
    ],
  );
});

test("shared renderer classifies native lines without flattening rectangles", () => {
  const scene = runtime();
  const line = {
    frame: { width: 40, height: 0 },
    style: { fill: "transparent", stroke: "#000000", strokeWidth: 1 },
    text: "",
    textParagraphs: [],
  };
  assert.equal(scene.isLinearGeometry(line), true);
  assert.equal(scene.isLinearGeometry({ ...line, frame: { width: 40, height: 20 } }), false);
  assert.equal(
    scene.isLinearGeometry({ ...line, geometry: "StraightConnector1", frame: { width: 40, height: 20 } }),
    true,
  );
});

test("PowerPoint vertical text modes retain East Asian column flow", () => {
  const scene = runtime();
  assert.equal(scene.textFlowMode("eaVert"), "vertical-rl-upright");
  assert.equal(scene.textFlowMode("vert270"), "vertical-lr-mixed");
  assert.equal(scene.textFlowMode("horz"), "horizontal");
  assert.match(appSource, /dataset\.textFlow = textFlowMode\(textFrame\.verticalType\)/);
  assert.match(runtimeSource, /writing-mode:vertical-rl;text-orientation:upright/);
  assert.match(runtimeSource, /\.ppt-run\{white-space:inherit\}/);
  assert.match(appSource, /whiteSpace = object\.textFrame\?\.wordWrap === false \? "pre" : "pre-wrap"/);
  assert.doesNotMatch(runtimeSource, /\.ppt-run\{white-space:pre-wrap\}/);
  assert.match(appSource, /dataset\.textWrap = textFrame\.wordWrap === false \? "none" : "wrap"/);
  assert.match(runtimeSource, /fitVerticalNoWrapText/);
  assert.match(runtimeSource, /paragraph\.scrollHeight <= availableHeight \+ 0\.5/);
});

test("PowerPoint frame width remains the outer border box in every renderer", () => {
  const scene = runtime();
  const properties = {};
  const node = {
    dataset: {},
    style: {
      setProperty(name, value) { properties[name] = value; },
    },
  };
  scene.applyObjectStyle(node, {
    id: "vertical-text",
    frame: { x: 557.4666666666667, y: 162.8, width: 319.6666666666667, height: 494.93333333333334, rotation: 0 },
    style: { fill: "transparent", stroke: "transparent", strokeWidth: 0 },
    textStyle: { align: "left", fontSize: 26.666666666666668 },
    textFrame: { marginLeft: 9.6, marginRight: 9.6, marginTop: 4.8, marginBottom: 4.8, verticalType: "eaVert" },
  }, 0);

  assert.equal(node.style.width, "319.6666666666667px");
  assert.equal(node.style.height, "494.93333333333334px");
  assert.equal(node.style.boxSizing, "border-box");
  assert.equal(node.style.padding, "4.8px 9.6px 4.8px 9.6px");
  assert.match(appSource, /boxSizing: "border-box", minWidth: "0", minHeight: "0"/);
  assert.match(appSource, /width: `\$\{Math\.max\(1, f\.width\)\}px`/);
  assert.match(appSource, /width: `\$\{geometry\.width\}px`/);
});

test("lossless preset animation dispatch receives node and slide geometry", () => {
  assert.match(
    losslessSource,
    /UniPptPresetAnimation\?\.frames\(a,\{node:e,slideWidth:d\.width,slideHeight:d\.height,baseTransform:base\}\)/,
    "generic p:anim interpolation requires the actual target node and slide dimensions",
  );
});
