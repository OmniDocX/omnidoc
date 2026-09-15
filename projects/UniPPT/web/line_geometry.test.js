"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const start = source.indexOf("function isLinearGeometry");
const end = source.indexOf("\nfunction renderLinearGeometry", start);
assert.ok(start >= 0 && end > start, "linear geometry classifier should be extractable");
const context = {};
vm.runInNewContext(`${source.slice(start, end)}\nthis.isLinearGeometry = isLinearGeometry;`, context);

const lineShape = {
  kind: "shape",
  frame: { width: 28.31, height: 0 },
  style: { fill: "transparent", stroke: "#000000", strokeWidth: 1 },
  text: "",
  textParagraphs: [],
};
assert.equal(context.isLinearGeometry(lineShape), true, "zero-height freeform must render as one line");
assert.equal(
  context.isLinearGeometry({ ...lineShape, frame: { width: 28.31, height: 10 } }),
  false,
  "ordinary outlined rectangles must keep their four-sided border",
);
assert.equal(
  context.isLinearGeometry({ ...lineShape, style: { ...lineShape.style, fill: "#ffffff" } }),
  false,
  "flat filled shapes must not be mistaken for connectors",
);
assert.equal(
  context.isLinearGeometry({ ...lineShape, style: { ...lineShape.style, stroke: "transparent", strokeWidth: 0 } }),
  false,
  "invisible degenerate shapes should stay invisible",
);
assert.equal(
  context.isLinearGeometry({ ...lineShape, kind: "connector", geometry: "Line", frame: { width: 80, height: 50 } }),
  true,
  "straight native connectors should use line geometry instead of CSS borders",
);
assert.equal(
  context.isLinearGeometry({ ...lineShape, kind: "connector", geometry: "BentConnector3", frame: { width: 80, height: 50 } }),
  false,
  "bent connectors must not be flattened into a straight line",
);

assert.match(source, /renderCustomGeometry\(node, object\) \|\| renderLinearGeometry/, "native custom paths must run before the line fallback");
assert.match(source, /geometryWidth > 0 \|\| geometryHeight > 0/, "zero-height and zero-width custom paths must remain renderable");
assert.match(source, /Math\.max\(1, geometryHeight\)/, "degenerate paths need a non-zero SVG viewport");
console.log("line geometry tests passed");
