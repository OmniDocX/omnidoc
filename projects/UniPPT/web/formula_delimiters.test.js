"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const htmlSource = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");

const helperStart = appSource.indexOf("function parseLatexEnvelope");
const helperEnd = appSource.indexOf("\nfunction openFormulaDialog", helperStart);
assert.ok(helperStart >= 0 && helperEnd > helperStart, "LaTeX delimiter helpers should be extractable");
const context = {};
vm.runInNewContext(
  `${appSource.slice(helperStart, helperEnd)}\nthis.api = { parseLatexEnvelope, formatLatexSource };`,
  context,
);

test("single-dollar delimiters select inline math", () => {
  assert.deepEqual(
    JSON.parse(JSON.stringify(context.api.parseLatexEnvelope("$a+b$", true))),
    { latex: "a+b", display: false, delimited: true },
  );
  assert.equal(context.api.formatLatexSource("a+b", false), "$a+b$");
});

test("double-dollar delimiters select display math", () => {
  assert.deepEqual(
    JSON.parse(JSON.stringify(context.api.parseLatexEnvelope("$$E=mc^2$$", false))),
    { latex: "E=mc^2", display: true, delimited: true },
  );
  assert.equal(context.api.formatLatexSource("E=mc^2", true), "$$E=mc^2$$");
});

test("formula dialog visibly documents both delimiter modes", () => {
  assert.match(htmlSource, /<code>\$\.\.\.\$<\/code>\s*行内公式/);
  assert.match(htmlSource, /<code>\$\$\.\.\.\$\$<\/code>\s*行间公式/);
  assert.match(htmlSource, /id="formulaSource"[^>]*>\$\$E=mc\^2\$\$<\/textarea>/);
});
