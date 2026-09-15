"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const htmlSource = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const cssSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

for (const id of [
  "slideContextMenu",
  "slideContextNew",
  "slideContextDuplicate",
  "slideContextMoveUp",
  "slideContextMoveDown",
  "slideContextTransition",
  "slideContextDelete",
]) {
  assert.match(htmlSource, new RegExp(`id="${id}"`), `${id} should exist in the slide menu`);
}
assert.match(htmlSource, /id="slideContextMenu"[^>]*role="menu"[^>]*hidden/, "the menu starts hidden and exposes menu semantics");
assert.match(htmlSource, /id="slideContextTransition"[\s\S]*?设置切换效果/, "the menu links slide operations to transition settings");

const renderStart = appSource.indexOf("function renderSlideList");
const renderEnd = appSource.indexOf("\nfunction renderCanvas", renderStart);
const renderSource = appSource.slice(renderStart, renderEnd);
assert.ok(renderStart >= 0 && renderEnd > renderStart, "slide list and context menu helpers should be extractable");
assert.equal((renderSource.match(/addEventListener\("contextmenu"/g) || []).length, 2, "thumbnail and outline rows both open the menu");
assert.match(renderSource, /function openSlideContextMenu[\s\S]*state\.activeSlide\s*=\s*index[\s\S]*renderAll\(\)/, "right-click selects the target slide before applying a command");
assert.match(renderSource, /slideContextMoveUp[\s\S]*index === 0/, "the first slide cannot move upward");
assert.match(renderSource, /slideContextMoveDown[\s\S]*slides\.length - 1/, "the last slide cannot move downward");
assert.match(renderSource, /slideContextDelete[\s\S]*slides\.length === 1/, "the last remaining slide cannot be deleted");
assert.match(renderSource, /clamp\(event\.clientX[\s\S]*clamp\(event\.clientY/, "the menu stays inside the viewport");
assert.match(renderSource, /function openCurrentSlideTransitionSettings[\s\S]*activateRibbon\("transitions"\)/, "transition settings open the dedicated ribbon");

assert.match(appSource, /slideContextNew"\)\.onclick\s*=\s*\(\)\s*=>\s*runSlideContextAction\(addSlide\)/, "new slide reuses the existing slide command");
assert.match(appSource, /slideContextDuplicate"\)\.onclick\s*=\s*\(\)\s*=>\s*runSlideContextAction\(duplicateSlide\)/, "duplicate reuses the existing slide command");
assert.match(appSource, /slideContextDelete"\)\.onclick\s*=\s*\(\)\s*=>\s*runSlideContextAction\(deleteSlide\)/, "delete reuses the existing slide command");
assert.match(appSource, /if \(!\$\("#slideContextMenu"\)\?\.hidden\)[\s\S]*event\.key === "Escape"/, "Escape closes the slide context menu");

assert.match(cssSource, /\.slide-context-menu\s*\{[^}]*position:fixed[^}]*z-index:136/s, "the menu floats above the workspace");
assert.match(cssSource, /\.slide-context-menu\[hidden\]\s*\{[^}]*display:none/s, "the hidden menu does not intercept input");
assert.match(cssSource, /\.slide-context-menu\s+\.danger\s*\{[^}]*color:/s, "delete is visually distinguished");
assert.doesNotMatch(cssSource, /\.ribbon-group\s*>\s*label\s*\{[^}]*display\s*:\s*none/s, "transition timing labels must not be hidden with decorative ribbon captions");
assert.match(cssSource, /\.ribbon-group\s*>\s*label:not\(\.transition-field\):not\(\.transition-check\)/, "only decorative ribbon captions are hidden in the compact M3 ribbon");

console.log("slide context menu tests passed");
