"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const app = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const style = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

test("editor shell follows the Workspace Material 3 surface ladder", () => {
  // Chrome sits on a tinted surface; white is reserved for raised panels.
  assert.match(style, /--surface:#f8fafd/);
  assert.match(style, /--surface-strong:#fff/);
  assert.match(style, /--surface-container:#f0f4f9/);
  assert.match(style, /--on-surface:#1f1f1f/);
  assert.match(style, /--outline-variant:#c4c7c5/);
  assert.match(style, /\.office-titlebar\s*\{[^}]*background:var\(--surface\)/s);
  assert.match(style, /\.canvas-viewport\s*\{[^}]*background:var\(--surface\)[^}]*background-image:none/s);

  // Chrome height must stay the sum of its parts or the workspace overflows.
  assert.match(style, /--workspace-chrome-height:176px/);
  assert.match(style, /--ribbon-height:48px/);
  assert.match(style, /\.office-titlebar\s*\{[^}]*height:56px/s);
  assert.match(style, /\.ribbon-tabs\s*\{[^}]*height:34px/s);
  assert.match(style, /\.office-statusbar\s*\{[^}]*height:28px/s);

  // The command bar is a floating tonal capsule, the signature Workspace shape.
  assert.match(style, /--radius-bar:24px/);
  assert.match(style, /\.ribbon\s*\{[^}]*margin:2px 16px 8px[^}]*border-radius:var\(--radius-bar\)[^}]*background:var\(--surface-container\)/s);

  assert.match(style, /\.ribbon-file \.ribbon-large\s*\{[^}]*grid-template-columns:22px max-content/s);
  assert.match(style, /\.office-workspace,\s*\.office-workspace\.has-format-pane\s*\{[^}]*grid-template-columns:260px minmax\(0,1fr\) 78px/s);
  assert.match(style, /\.slide-thumb\s*\{[^}]*border-radius:var\(--radius-thumb\)/s);
  assert.match(style, /--radius-thumb:12px/);
  assert.match(style, /\.notes-strip\s*\{[^}]*background:var\(--surface-container\)/s);
});

test("shell keeps the UniPPT terracotta accent rather than adopting Google blue", () => {
  assert.match(style, /--brand:#c43b1b/);
  assert.match(style, /--brand-container:#fbe9e4/);
  assert.doesNotMatch(style, /--brand:\s*#0b57d0/);
  // Primary and tonal actions are pills, as in the Workspace app bar.
  assert.match(style, /--radius-pill:100px/);
  assert.match(style, /\.title-present\s*\{[^}]*border-radius:var\(--radius-pill\)!important[^}]*background:var\(--brand\)!important/s);
  assert.match(style, /\.title-ai\s*\{[^}]*background:var\(--brand-container\)!important/s);
  assert.match(style, /\.slide-item\.active \.slide-thumb\s*\{[^}]*border-color:var\(--brand\)/s);
});

test("interaction feedback uses on-surface state layers instead of ad-hoc greys", () => {
  assert.match(style, /--state-hover:#1f1f1f14/);
  assert.match(style, /--state-pressed:#1f1f1f1f/);
  assert.match(style, /\.ribbon-group button:hover\s*\{[^}]*background:var\(--state-hover\)/s);
  assert.match(style, /\.utility-rail button:hover\s*\{[^}]*background:var\(--state-hover\)/s);
  assert.match(style, /\.utility-rail button:active\s*\{[^}]*background:var\(--state-pressed\)/s);
  // Keyboard focus stays visible without showing a ring on every mouse click.
  assert.match(style, /focus-visible\s*\{\s*outline:2px solid var\(--brand\)/s);
  assert.match(style, /--motion:120ms cubic-bezier\(\.2,0,0,1\)/);
});

test("right utility rail exposes working presentation shortcuts", () => {
  for (const id of ["utilityTemplates", "utilityBlocks", "utilityMedia", "utilityUploads", "utilityConvert"]) {
    assert.match(html, new RegExp(`id="${id}"`), `${id} should be visible in the shell`);
  }
  assert.match(html, /class="material-symbols-rounded"/);
  assert.match(app, /\.utility-rail \[data-target-ribbon\]/);
  assert.match(app, /activateRibbon\(button\.dataset\.targetRibbon\)/);
  assert.match(app, /\$\("#utilityUploads"\)\.onclick\s*=\s*openDocumentPicker/);
});

test("quick save is a recognizable icon and command groups are visibly separated", () => {
  assert.match(html, /id="saveUdocQuick"[\s\S]*?material-symbols-rounded[^>]*>save<\/span>/);
  assert.match(style, /\.quick-access #saveUdocQuick\s*\{[^}]*color:var\(--brand\)/);
  // Group and pane boundaries are Material hairlines rather than dashed rules.
  assert.match(style, /\.ribbon-group\s*\{[^}]*border-right:1px solid var\(--outline-variant\)/s);
  assert.match(style, /\.slide-pane\s*\{[^}]*border-right:1px solid var\(--hairline\)/s);
  assert.match(style, /\.utility-rail\s*\{[^}]*border-left:1px solid var\(--hairline\)/s);
});

test("docked panes yield space while overlay format controls preserve canvas geometry", () => {
  assert.doesNotMatch(style, /\.office-workspace\.has-format-pane \.utility-rail[^}]*display:none/);
  assert.match(style, /\.office-workspace\.has-animation-pane \.utility-rail[^}]*display:none/);
  assert.match(style, /@media \(max-width:900px\)[\s\S]*\.utility-rail\s*\{\s*display:none/);
});
