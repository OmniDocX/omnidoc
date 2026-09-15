"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const elements = new Map();
const document = {
  body: { append(node) { elements.set(node.id, node); } },
  createElement() { return { hidden: false, setAttribute() {}, append() {}, replaceChildren() {}, querySelectorAll() { return []; } }; },
  getElementById(id) { return elements.get(id) || null; },
};
const context = { document };
vm.runInNewContext(fs.readFileSync(path.join(__dirname, "media_runtime.js"), "utf8"), context);

function media(baseVolume) {
  return {
    dataset: { baseVolume: String(baseVolume), masterVolume: "1", masterMuted: "0" },
    volume: baseVolume,
    muted: false,
    matches() { return false; },
  };
}
function root(nodes) {
  return { matches() { return false; }, querySelectorAll() { return nodes; } };
}

const ordinary = media(0.8);
context.UniPptMedia.setOutput(root([ordinary]), 0.5, true);
assert.equal(ordinary.volume, 0.4, "master output must multiply the object's native volume");
assert.equal(ordinary.muted, true);
assert.equal(ordinary.dataset.masterVolume, "0.5");

const persistent = media(0.8);
elements.set("unippt-persistent-media", root([persistent]));
context.UniPptMedia.setOutput(root([]), 0.25, false, true);
assert.equal(persistent.volume, 0.2, "cross-slide persistent audio must follow presenter volume");
assert.equal(persistent.muted, false);

const source = fs.readFileSync(path.join(__dirname, "media_runtime.js"), "utf8");
assert.match(source, /loadedmetadata[\s\S]*?applyOutput\(media\)/, "metadata loading must preserve master output settings");
assert.doesNotMatch(source, /loadedmetadata[\s\S]{0,120}media\.volume\s*=\s*descriptor\.volume/, "metadata loading must not reset master volume");

console.log("media output tests passed");
