const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const root = __dirname;
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");

test("dynamic HTML and SVG are inserted as namespaced portable content", () => {
  const index = read("index.html");
  const app = read("app.js");
  assert.match(index, /id="addDynamicContent"/);
  assert.match(app, /extension\["org\.unippt\.dynamic"\]/);
  assert.match(app, /kind === "svg"/);
  assert.match(app, /allow-scripts allow-forms allow-popups/);
  assert.doesNotMatch(app, /allow-same-origin/);
});

test("editor and lossless player share the dynamic scene renderer", () => {
  const app = read("app.js");
  const runtime = read("presentation_scene_runtime.js");
  assert.match(app, /dynamicObjects: state\.deck\.extensions/);
  assert.match(runtime, /function renderDynamicContent/);
  assert.match(runtime, /options\.dynamicObjects/);
  assert.match(runtime, /frame\.srcdoc/);
});

test("lossless HTML forwards the same extension map to the shared renderer", () => {
  const lossless = fs.readFileSync(
    path.join(root, "..", "crates", "unippt-server", "src", "lossless_html.rs"),
    "utf8",
  );
  assert.match(lossless, /org\.unippt\.dynamic/);
  assert.match(lossless, /dynamicObjects/);
});
