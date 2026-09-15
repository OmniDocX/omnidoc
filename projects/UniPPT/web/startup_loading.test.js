"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const app = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const materialCss = fs.readFileSync(
  path.join(__dirname, "vendor", "material-symbols", "material-symbols-rounded.css"),
  "utf8",
);
const materialFontPath = path.join(
  __dirname,
  "vendor",
  "material-symbols",
  "material-symbols-rounded.woff2",
);
const katexCss = fs.readFileSync(
  path.join(__dirname, "vendor", "katex", "katex.min.css"),
  "utf8",
);

test("the inline startup scheduler remains valid JavaScript", () => {
  const source = html.match(/<script>([\s\S]*?)<\/script>/)?.[1];
  assert.ok(source, "the startup script must be present");
  assert.doesNotThrow(() => new vm.Script(source, { filename: "index-startup.js" }));
});

test("the demo request starts before editor modules and optional features stay off the critical path", () => {
  const requestAt = html.indexOf('fetch("/api/demo"');
  const optimizedRequestAt = html.indexOf('if (startupProfile === "optimized" && !handoffRequested) requestDemo()');
  const initialLoadAt = html.indexOf("initialFeatures.map(loadFeature)");
  const appAt = html.indexOf("{{asset:/app.js}}");
  assert.ok(requestAt > 0 && requestAt < optimizedRequestAt && optimizedRequestAt < initialLoadAt);
  assert.ok(requestAt < appAt);
  assert.match(html, /startupProfile.*legacy/);
  assert.match(html, /if \(startupProfile === "optimized" && !handoffRequested\) requestDemo\(\)/);
  assert.match(html, /handoffRequested[\s\S]*?takeDemoResponse\(\)[\s\S]*?if \(handoffRequested\) return null/);
  assert.match(
    html,
    /startupProfile === "legacy"[\s\S]*?\["core", "render", "presentation", "ai", "export", "math"\]/,
  );

  const core = html.match(/core:\s*\{\s*scripts:\s*\[([\s\S]*?)\]/)?.[1] || "";
  assert.match(core, /presentation_scene_runtime\.js/);
  assert.match(core, /document_cache\.js/);
  assert.match(core, /document_handoff\.js/);
  for (const optional of ["ai_runtime", "screen_recorder_runtime", "presentation_transition_runtime", "katex.min"]) {
    assert.doesNotMatch(core, new RegExp(optional));
  }
  assert.match(html, /markFirstPaint\(\)[\s\S]*?requestIdleCallback/);
  const firstPaintBlock = html.match(/markFirstPaint\(\)\s*\{([\s\S]*?)\n\s*\},\n\s*snapshot/)?.[1] || "";
  assert.match(firstPaintBlock, /window\.addEventListener\("load", warmRenderFeatures/);
  for (const onDemand of ["presentation", "export", "ai"]) {
    assert.doesNotMatch(firstPaintBlock, new RegExp(`loadFeature\\("${onDemand}"\\)`));
  }
  assert.match(html, /dataset\.unipptStartupMetrics = JSON\.stringify\(\{/);
  assert.match(html, /criticalTransferBytes/);
  assert.match(app, /takeDemoResponse\?\.\(\)/);
  assert.match(app, /document\.readyState === "loading"/,
    "dynamically loaded app.js must boot both before and after DOMContentLoaded");
});

test("Material Symbols and KaTeX are fully local and their font URLs are versioned", () => {
  assert.doesNotMatch(html, /fonts\.googleapis\.com|fonts\.gstatic\.com|cdn\.jsdelivr\.net/);
  assert.match(html, /vendor\/material-symbols\/material-symbols-rounded\.css/);
  assert.match(html, /vendor\/katex\/katex\.min\.css/);
  assert.match(html, /vendor\/katex\/katex\.min\.js/);
  assert.match(materialCss, /material-symbols-rounded\.woff2\?v=[0-9a-f]{16}/);
  assert.ok(fs.statSync(materialFontPath).size < 20 * 1024, "the editor icon subset must stay below 20 KiB");
  assert.equal((katexCss.match(/fonts\//g) || []).length, 60);
  assert.equal((katexCss.match(/\?v=[0-9a-f]{16}/g) || []).length, 60);
});

test("every shell asset is a server-resolved content hash token", () => {
  for (const asset of [
    "unippt-logo.svg",
    "style.css",
    "app.js",
    "presentation_scene_runtime.js",
    "document_cache.js",
    "document_handoff.js",
  ]) {
    assert.match(html, new RegExp(`\\{\\{asset:/${asset.replaceAll(".", "\\.")}\\}\\}`));
  }
});
