"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const app = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const style = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

test("public editor branding does not disclose the server implementation language", () => {
  assert.doesNotMatch(html, /\bRust\b/i);
  assert.doesNotMatch(app, /Rust (?:PPTX|服务|正在|解析)/i);
  assert.match(html, /正在解析 PPTX/);
  assert.match(app, /UniPPT 文档引擎已连接/);
});

test("tablet and phone layouts keep commands reachable instead of deleting ribbon groups", () => {
  assert.match(style, /@media \(max-width: 900px\)[\s\S]*\.ribbon-tabs\s*\{[^}]*overflow-x:auto/);
  assert.doesNotMatch(style, /ribbon-tabs button:nth-of-type\([^}]*display:none/);
  assert.doesNotMatch(style, /ribbon-group:nth-child\([^}]*display:none/);
  assert.match(style, /@media \(max-width: 720px\)[\s\S]*\.office-workspace[^}]*grid-template-columns:minmax\(0,1fr\)/);
  assert.match(style, /@media \(pointer: coarse\)[\s\S]*\.resize-handle\s*\{[^}]*width:14px[^}]*height:14px/);
  assert.match(style, /height:\s*100dvh/);
  assert.match(style, /env\(safe-area-inset-bottom\)/);
});
