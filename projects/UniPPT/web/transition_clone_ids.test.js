"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const start = appSource.indexOf("function namespaceTransitionCloneIds");
const end = appSource.indexOf("\nfunction ", start + 1);
assert.ok(start >= 0 && end > start, "transition clone namespace helper must be extractable");

class MockNode {
  constructor(tagName = "div") {
    this.tagName = tagName;
    this.children = [];
    this.values = new Map();
    this.textContent = "";
  }
  get attributes() {
    return [...this.values].map(([name, value]) => ({ name, value }));
  }
  append(...nodes) { this.children.push(...nodes); }
  getAttribute(name) { return this.values.get(name) ?? null; }
  setAttribute(name, value) { this.values.set(name, String(value)); }
  querySelectorAll(selector) {
    const descendants = [];
    const visit = (node) => {
      for (const child of node.children) {
        descendants.push(child);
        visit(child);
      }
    };
    visit(this);
    if (selector === "*") return descendants;
    if (selector === "[id]") return descendants.filter((node) => node.getAttribute("id"));
    if (selector === "style") return descendants.filter((node) => node.tagName === "style");
    return [];
  }
}

const root = new MockNode("section");
const filter = new MockNode("filter");
const pattern = new MockNode("pattern");
const image = new MockNode("image");
const shape = new MockNode("path");
const use = new MockNode("use");
const label = new MockNode("label");
const timing = new MockNode("animate");
const style = new MockNode("style");
root.setAttribute("id", "presentStage");
filter.setAttribute("id", "duo-filter");
pattern.setAttribute("id", "shape-fill");
image.setAttribute("style", "filter:url(#duo-filter)");
shape.setAttribute("fill", "url(\"#shape-fill\")");
use.setAttribute("href", "#shape-fill");
label.setAttribute("aria-labelledby", "shape-fill duo-filter");
timing.setAttribute("begin", "shape-fill.click; duo-filter.end+1s");
timing.setAttribute("data-origin", "https://shape-fill.example/test");
style.textContent = "#shape-fill > .picture { filter:url('#duo-filter') }";
root.append(filter, pattern, image, shape, use, label, timing, style);

const context = { transitionCloneIdSequence: 0 };
vm.runInNewContext(`${appSource.slice(start, end)}\nthis.namespaceIds = namespaceTransitionCloneIds;`, context);
context.namespaceIds(root, "test");

const filterId = filter.getAttribute("id");
const patternId = pattern.getAttribute("id");
assert.match(filterId, /^unippt-test-1-duo-filter$/);
assert.match(patternId, /^unippt-test-1-shape-fill$/);
assert.equal(image.getAttribute("style"), `filter:url(#${filterId})`);
assert.equal(shape.getAttribute("fill"), `url(\"#${patternId}\")`);
assert.equal(use.getAttribute("href"), `#${patternId}`);
assert.equal(label.getAttribute("aria-labelledby"), `${patternId} ${filterId}`);
assert.equal(timing.getAttribute("begin"), `${patternId}.click; ${filterId}.end+1s`);
assert.equal(timing.getAttribute("data-origin"), "https://shape-fill.example/test");
assert.equal(style.textContent, `#${patternId} > .picture { filter:url('#${filterId}') }`);

console.log("transition clone id tests passed");
