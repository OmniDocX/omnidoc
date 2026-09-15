"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "presentation_transition_runtime.js"), "utf8");
const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const styleSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

function makeNode(tagName = "div") {
  const classes = new Set();
  const attributes = [];
  const node = {
    tagName,
    style: {},
    dataset: {},
    animations: [],
    children: [],
    parentElement: null,
    clientWidth: 1280,
    clientHeight: 720,
    attributes,
    classList: {
      add(...values) { values.forEach((value) => classes.add(value)); },
      remove(...values) { values.forEach((value) => classes.delete(value)); },
      contains(value) { return classes.has(value); },
    },
    getAttribute(name) {
      return attributes.find((entry) => entry.name === name)?.value || null;
    },
    setAttribute(name, value) {
      const existing = attributes.find((entry) => entry.name === name);
      if (existing) existing.value = String(value);
      else attributes.push({ name, value: String(value) });
    },
    removeAttribute() {},
    querySelectorAll(selector) {
      const all = [];
      const walk = (current) => {
        for (const child of current.children) {
          all.push(child);
          walk(child);
        }
      };
      walk(node);
      if (selector === "[id]") return all.filter((child) => child.getAttribute("id"));
      if (selector === "audio" || selector === "video") {
        return all.filter((child) => String(child.tagName).toLowerCase() === selector);
      }
      return all;
    },
    append(...nodes) {
      for (const child of nodes) {
        child.parentElement = node;
        node.children.push(child);
      }
    },
    insertBefore(child, reference) {
      child.parentElement = node;
      const index = reference ? node.children.indexOf(reference) : -1;
      if (index < 0) node.children.push(child);
      else node.children.splice(index, 0, child);
      return child;
    },
    cloneNode(deep = false) {
      const clone = makeNode(node.tagName);
      Object.assign(clone.style, node.style);
      classes.forEach((value) => clone.classList.add(value));
      if (deep) node.children.forEach((child) => clone.append(child.cloneNode(true)));
      return clone;
    },
    animate(frames, options) {
      const player = {
        frames,
        options,
        playState: "running",
        startTime: null,
        addEventListener() {},
        cancel() { this.playState = "idle"; },
      };
      node.animations.push(player);
      return player;
    },
    remove() {
      if (!node.parentElement) return;
      const index = node.parentElement.children.indexOf(node);
      if (index >= 0) node.parentElement.children.splice(index, 1);
      node.parentElement = null;
    },
  };
  Object.defineProperty(node, "className", {
    get() { return [...classes].join(" "); },
    set(value) {
      classes.clear();
      String(value).split(/\s+/).filter(Boolean).forEach((entry) => classes.add(entry));
    },
  });
  return node;
}

const injected = [];
const documentMock = {
  timeline: { currentTime: 240 },
  head: {
    append(node) { injected.push(node); },
  },
  getElementById(id) {
    return injected.find((node) => node.id === id) || null;
  },
  createElement(tagName) {
    const node = makeNode(tagName);
    if (tagName === "style") node.id = "";
    return node;
  },
};

const context = { document: documentMock };
vm.runInNewContext(source, context);
const runtime = context.UniPptPresentationTransitions;

for (const kind of ["pagecurl", "pagecurldouble", "peeloff", "origami", "fallover"]) {
  assert.equal(runtime.supports(kind), true, `${kind} must be owned by the shared editor/export runtime`);
}

assert.equal(runtime.resolveDirection({}, "fallover"), "down", "a directionless Fall Over drops toward the viewer around the bottom edge");
assert.equal(runtime.resolveDirection({}, "pagecurl"), "left");
assert.equal(runtime.resolveDirection({}, "peeloff"), "left");
assert.equal(runtime.resolveDirection({ direction: "right" }, "fallover"), "right");

const earlyCurl = runtime.curlGeometryAt(0, "left");
const midCurl = runtime.curlGeometryAt(.64, "left");
assert.ok(earlyCurl.flatBoundary >= .99, "the single-page curl must start with the whole outgoing sheet still flat");
assert.ok(earlyCurl.theta < .001, "the cylindrical roll has not started on the first frame");
assert.ok(midCurl.theta > 1, "the mid curl must have turned far enough to expose the reverse");
assert.ok(midCurl.bandWidth > .08, "the travelling crest must remain a visible band of paper, not a dark seam");

const startPeel = runtime.peelRemainderPolygon(0, "left");
const midPeel = runtime.peelRemainderPolygon(.5, "left");
const endPeel = runtime.peelRemainderPolygon(1, "left");
assert.match(startPeel, /100% 0/, "peel starts with the outgoing page fully stuck");
assert.notEqual(startPeel, midPeel, "the stuck remainder must recede along a diagonal");
assert.notEqual(midPeel, endPeel, "the peel must finish by releasing the last corner");
assert.notEqual(
  runtime.peelRemainderPolygon(.5, "left"),
  runtime.peelRemainderPolygon(.5, "right"),
  "left and right peels must mirror their remaining polygons",
);

const flaps = runtime.origamiFlaps();
assert.equal(flaps.length, 4, "origami folds the outgoing page into four triangular flaps");
assert.equal(new Set(flaps.map((flap) => flap.clip)).size, 4, "each origami flap occupies a distinct triangle");
assert.equal(new Set(flaps.map((flap) => flap.origin)).size, 4, "origami flaps fold from four different outer edges");

function flatten(root, result = []) {
  for (const child of root.children) {
    result.push(child);
    flatten(child, result);
  }
  return result;
}

function play(kind, direction) {
  const presenter = makeNode("main");
  const stage = makeNode("section");
  const underlay = makeNode("section");
  underlay.className = "slide-transition-underlay";
  underlay.style = { width: "1280px", height: "720px", transform: "translate(-50%,-50%)" };
  stage.style = { width: "1280px", height: "720px", transform: "translate(-50%,-50%)" };
  presenter.append(underlay, stage);
  const result = runtime.create(stage, { kind, durationMs: 900, direction }, underlay, { quality: "low" });
  assert.ok(result?.player, `${kind} must produce a WAAPI player instead of falling back to fade`);
  assert.ok(result.animations.length >= 2, `${kind} must drive more than a single opacity fade`);
  return { presenter, stage, underlay, result, nodes: flatten(presenter) };
}

const curl = play("pagecurl", "left");
assert.ok(curl.nodes.some((node) => node.className.includes("page-curl-mesh")), "pageCurl must build a cylindrical strip mesh");
assert.equal(curl.nodes.some((node) => node.className.includes("page-turn-peel-frame")), false);

const peel = play("peeloff", "left");
assert.ok(peel.nodes.some((node) => node.className.includes("page-turn-peel-frame")), "peelOff must lift a corner sheet, not reuse the book mesh");
assert.equal(peel.nodes.some((node) => node.className.includes("page-curl-mesh")), false);

const origami = play("origami");
assert.equal(
  origami.nodes.filter((node) => node.className.includes("page-turn-origami-flap")).length,
  4,
  "origami must fold four triangular flaps",
);

const fall = play("fallover", "down");
assert.ok(fall.nodes.some((node) => node.className.includes("page-turn-fall-sheet")), "fallOver must drop one rigid sheet toward the camera");
const fallSheet = fall.nodes.find((node) => node.className.includes("page-turn-fall-sheet"));
assert.match(fallSheet.animations[0].frames.at(-1).transform, /rotateX/, "the default fall rotates around the bottom edge");
assert.equal(fallSheet.animations[0].frames.at(-1).opacity, 0);

curl.result.cleanup();
peel.result.cleanup();
origami.result.cleanup();
fall.result.cleanup();
assert.equal(
  flatten(curl.presenter).filter((node) => /page-(?:curl|turn|book)/.test(node.className)).length,
  0,
  "page-turn cleanup must remove temporary layers",
);

assert.match(
  appSource,
  /UniPptPresentationTransitions\?\.supports\(normalizedKind\)/,
  "the editor must dispatch shared page-turn kinds before its local fallbacks",
);
assert.match(styleSource, /\.page-turn-peel-frame/, "editor chrome must style the new page-turn surfaces");
assert.match(source, /frontOpacity/, "exported HTML must cull the book front after vertical");
assert.doesNotMatch(
  source,
  /\.page-book-leaf-texture\{[^}]*backface-visibility/,
  "exported HTML must not let the reverse texture cull itself after rotateY(180deg)",
);
assert.match(source, /createPeel|function createPeel/, "peelOff must live in the shared runtime");

console.log("page turn runtime tests passed");
