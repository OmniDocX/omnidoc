"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const cssSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

const syncStart = appSource.indexOf("function syncWorkspaceLayout");
const syncEnd = appSource.indexOf("\nconst directionalTransitions", syncStart);
assert.ok(syncStart >= 0 && syncEnd > syncStart, "workspace layout sync should be extractable");

const classes = new Set();
const workspace = {
  classList: {
    contains(name) { return classes.has(name); },
    toggle(name, force) { force ? classes.add(name) : classes.delete(name); },
  },
};
const formatPane = { hidden: true };
const animationPane = { hidden: true };
const fitCalls = [];
const context = {
  state: { deck: {}, autoFit: true },
  $(selector) {
    if (selector === ".format-pane") return formatPane;
    if (selector === "#animationPane") return animationPane;
    if (selector === ".office-workspace") return workspace;
    throw new Error(`unexpected selector ${selector}`);
  },
  scheduleFitSlide() { fitCalls.push("fit"); },
};
vm.runInNewContext(`${appSource.slice(syncStart, syncEnd)}\nthis.syncWorkspaceLayout = syncWorkspaceLayout;`, context);

formatPane.hidden = false;
context.syncWorkspaceLayout();
assert.equal(classes.has("has-format-pane"), true, "format pane state remains available for styling");
assert.equal(fitCalls.length, 0, "opening the overlay format pane must not alter auto-fit zoom");

formatPane.hidden = true;
context.syncWorkspaceLayout();
assert.equal(fitCalls.length, 0, "closing the overlay format pane must not alter auto-fit zoom");

animationPane.hidden = false;
context.syncWorkspaceLayout();
assert.equal(fitCalls.length, 1, "the docked animation pane still refits the genuinely narrower canvas");

assert.match(cssSource, /\.format-pane\s*\{[^}]*position:absolute[^}]*right:0/s, "the format pane overlays the workspace");
assert.match(cssSource, /\.office-workspace,\s*\.office-workspace\.has-format-pane\s*\{[^}]*grid-template-columns:260px minmax\(0,1fr\) 78px/s, "opening the format pane keeps the desktop workspace columns unchanged");
assert.doesNotMatch(cssSource, /\.office-workspace\.has-format-pane\s+\.utility-rail[^}]*display:none/s, "the overlay must not remove a grid column behind it");

console.log("format pane overlay layout tests passed");
