"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const styleSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

function functionSource(name) {
  const start = appSource.indexOf(`function ${name}`);
  assert.ok(start >= 0, `${name} must remain a named function`);
  const end = appSource.indexOf("\nfunction ", start + name.length + 9);
  return appSource.slice(start, end < 0 ? appSource.length : end);
}

function makeNode(tracker, label) {
  const classes = new Set();
  const node = {
    label,
    style: {},
    dataset: {},
    animations: [],
    removed: false,
    parentElement: null,
    classList: {
      add(...values) { values.forEach((value) => classes.add(value)); },
      remove(...values) { values.forEach((value) => classes.delete(value)); },
      contains(value) { return classes.has(value); },
    },
    removeAttribute() {},
    querySelectorAll() { return []; },
    cloneNode() {
      const clone = makeNode(tracker, `${label}:clone`);
      Object.assign(clone.style, node.style);
      classes.forEach((value) => clone.classList.add(value));
      return clone;
    },
    animate(frames, options) {
      const player = {
        frames,
        options,
        owner: node,
        playState: "running",
        addEventListener() {},
        cancel() { this.playState = "idle"; },
      };
      node.animations.push(player);
      return player;
    },
    remove() { node.removed = true; },
  };
  Object.defineProperty(node, "className", {
    get() { return [...classes].join(" "); },
    set(value) {
      classes.clear();
      String(value).split(/\s+/).filter(Boolean).forEach((entry) => classes.add(entry));
    },
  });
  tracker.nodes.push(node);
  return node;
}

const tracker = { nodes: [] };
const presenter = {
  inserted: [],
  insertBefore(node) {
    node.parentElement = presenter;
    presenter.inserted.push(node);
    return node;
  },
};
const stage = makeNode(tracker, "incoming-stage");
stage.style.transform = "translate3d(-50%,-50%,0) scale(.8)";
stage.style.width = "1280px";
stage.style.height = "720px";
stage.parentElement = presenter;
const underlay = makeNode(tracker, "outgoing-underlay");
underlay.style.transform = stage.style.transform;
underlay.style.width = stage.style.width;
underlay.style.height = stage.style.height;
underlay.parentElement = presenter;
underlay.classList.add("slide-transition-underlay");

const context = {
  document: {
    createElement(tag) { return makeNode(tracker, tag); },
  },
  namespaceTransitionCloneIds(root) { return root; },
  presentationQualityTier() { return "high"; },
  runSlideTransition(_stage, create) { return create(); },
};
vm.runInNewContext(
  `${functionSource("createSharedTransitionTexture")}\n${functionSource("animateRippleTransition")}\nthis.animateRippleTransition = animateRippleTransition;`,
  context,
);

const result = context.animateRippleTransition(stage, 1400, underlay, "ripple");
const layers = tracker.nodes.filter((node) => node.classList.contains("ripple-refraction-layer"));
const caustic = tracker.nodes.find((node) => node.classList.contains("ripple-refraction-caustic"));

assert.equal(result.player.owner, underlay, "Ripple's primary animation must move the outgoing page");
assert.ok(underlay.animations.length >= 1, "the outgoing page must actively refract and fade out");
assert.ok(
  underlay.animations[0].frames.some((frame) => String(frame.transform || "").includes("scale(")),
  "the outgoing page must carry spatial distortion, not only opacity",
);
assert.equal(layers.length, 3, "the shipping fallback must stay bounded to three adjacent refraction bands");
assert.ok(layers.every((layer) => /outgoing-underlay:clone/.test(layer.label)), "the test must exercise the real no-texture-runtime fallback");
assert.ok(
  layers.every((layer) => layer.animations.some((animation) => animation.frames.every((frame) => frame["--ripple-radius"]))),
  "each adjacent refraction band must follow the same expanding wave radius",
);
assert.ok(
  new Set(layers.map((layer) => layer.style["--ripple-band-offset"])).size === 3,
  "the three wave bands must remain adjacent instead of becoming independent ripples",
);
assert.ok(caustic, "Ripple must include a moving refractive highlight");
assert.ok(stage.animations.length >= 1, "the incoming page must be progressively revealed below the ripple");
assert.equal(stage.animations.at(-1).frames.at(-1).opacity, 1);
assert.match(stage.animations.at(-1).frames[0].clipPath, /circle\(0%/);
assert.match(stage.animations.at(-1).frames.at(-1).clipPath, /circle\(78%/);
assert.ok(
  tracker.nodes.flatMap((node) => node.animations).every((animation) => animation.options.easing === "linear"),
  "all ripple layers must share a deterministic linear timeline",
);
assert.ok(
  tracker.nodes.flatMap((node) => node.animations).flatMap((animation) => animation.frames)
    .every((frame) => !("filter" in frame)),
  "Ripple must not animate expensive full-screen filters",
);
assert.match(styleSource, /\.ripple-refraction-layer\b/);
assert.match(styleSource, /\.ripple-refraction-caustic\b/);
assert.match(styleSource, /@property --ripple-radius/);
assert.match(styleSource, /circle farthest-corner/);
const rippleCss = styleSource.slice(
  styleSource.indexOf("@property --ripple-radius"),
  styleSource.indexOf(".wind-transition-strip"),
);
assert.doesNotMatch(rippleCss, /filter\s*:/, "Ripple CSS must not apply a full-frame blur/filter");

result.cleanup();
assert.ok(layers.every((layer) => layer.removed), "Ripple cleanup must remove every cloned old-page ring");
assert.equal(caustic.removed, true);

console.log("ripple transition runtime tests passed");
