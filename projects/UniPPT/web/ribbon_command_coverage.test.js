"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const source = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");

test("enabled buttons declare an id, data command, theme action, or form value", () => {
  const tags = html.match(/<button\b[^>]*>/g) || [];
  const inert = tags.filter((tag) => !/\bdisabled\b/.test(tag)
    && !/\bid=/.test(tag)
    && !/\bdata-/.test(tag)
    && !/\btheme-chip\b/.test(tag)
    && !/\bvalue=/.test(tag));
  assert.deepEqual(inert, [], `visually enabled inert buttons:\n${inert.join("\n")}`);
});

test("home ribbon commands are bound to real scene operations", () => {
  const commands = [
    "pasteObject", "cutObject", "copyObject", "formatPainter", "applySlideLayout", "resetSlideLayout",
    "growFont", "shrinkFont", "ribbonUnderline", "ribbonStrike", "ribbonSubscript", "ribbonSuperscript",
    "paragraphBullets", "paragraphNumbering", "paragraphOutdent", "paragraphIndent", "paragraphSpacing",
    "alignLeft", "alignCenter", "alignRight", "alignJustify", "verticalAlign", "bringToFront",
    "shapeFill", "shapeOutline", "findText", "replaceText", "selectNextObject",
  ];
  for (const id of commands) {
    assert.match(html, new RegExp(`id="${id}"`), `${id} must exist`);
    assert.match(source, new RegExp(`\\$\\("#${id}"\\)\\.onclick\\s*=`), `${id} must be bound`);
  }
  assert.match(source, /navigator\.clipboard\?\.writeText/);
  assert.match(source, /renewIds\(\[clone\]\)/);
  assert.match(source, /reconcileRichTextParagraphs/);
});

test("commands without a loss-aware model are visibly disabled", () => {
  assert.match(html, /<button disabled title="节模型尚未纳入无损 PPTX 编辑投影">§ 节<\/button>/);
  assert.match(html, /<button disabled title="比较演示文稿尚未纳入无损差异模型">比较<\/button>/);
});
