"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const style = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

test("ribbon panels use a compact horizontal grid and return height to the workspace", () => {
  assert.match(style, /--ribbon-height:\s*68px/);
  assert.match(
    style,
    /\.ribbon-panel\.active\s*\{[^}]*display:\s*grid[^}]*grid-auto-flow:\s*column[^}]*grid-auto-columns:\s*max-content/s,
  );
  assert.match(style, /height:\s*calc\(100vh - var\(--workspace-chrome-height\)\)/);
  assert.match(style, /\.ribbon-large\s*\{[^}]*height:\s*44px/s);
  assert.match(style, /\.ribbon-placeholder[^}]*padding:\s*3px 10px 12px/s);
});
