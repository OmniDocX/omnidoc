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
    children: [],
    removed: false,
    parentElement: null,
    classList: {
      add(...values) { values.forEach((value) => classes.add(value)); },
      remove(...values) { values.forEach((value) => classes.delete(value)); },
      contains(value) { return classes.has(value); },
    },
    removeAttribute() {},
    querySelectorAll() { return []; },
    append(...nodes) {
      nodes.forEach((child) => {
        child.parentElement = node;
        node.children.push(child);
      });
    },
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
stage.clientWidth = 1280;
stage.clientHeight = 720;
stage.parentElement = presenter;
const underlay = makeNode(tracker, "outgoing-underlay");
underlay.style.transform = stage.style.transform;
underlay.style.width = stage.style.width;
underlay.style.height = stage.style.height;
underlay.parentElement = presenter;
underlay.classList.add("slide-transition-underlay");

const context = {
  document: {
    timeline: { currentTime: 480 },
    createElement(tag) { return makeNode(tracker, tag); },
  },
  clamp(value, minimum, maximum) {
    return Math.max(minimum, Math.min(maximum, value));
  },
  namespaceTransitionCloneIds(root) { return root; },
  presentationQualityTier() { return "high"; },
  runSlideTransition(_stage, create) { return create(); },
};

assert.match(
  functionSource("animateSlideTransition"),
  /normalizedKind\s*===\s*"wind"\s*\?\s*"right"\s*:\s*"left"/,
  "a directionless p15 Wind transition must bypass the generic leftward default",
);
vm.runInNewContext(
  `${functionSource("createSharedTransitionTexture")}\n`
    + `${functionSource("windSheetStateAt")}\n`
    + `${functionSource("windProjectiveTransform")}\n`
    + `${functionSource("synchronizeTransitionAnimations")}\n`
    + `${functionSource("animateWindTransition")}\n`
    + "this.windSheetStateAt = windSheetStateAt; "
    + "this.windProjectiveTransform = windProjectiveTransform; "
    + "this.animateWindTransition = animateWindTransition;",
  context,
);

// Native p15:wind in 9.pptx has no explicit direction. PowerPoint's default
// is a coherent sheet accelerating up and to the right.
const result = context.animateWindTransition(stage, 2000, undefined, underlay, "wind");
const strips = tracker.nodes.filter((node) => node.classList.contains("wind-transition-strip"));
const mesh = tracker.nodes.find((node) => node.classList.contains("wind-transition-mesh"));
const lighting = tracker.nodes.filter((node) => node.classList.contains("wind-transition-lighting"));
const shadowFrame = tracker.nodes.find((node) => node.classList.contains("wind-transition-shadow-frame"));
const sheetShadow = tracker.nodes.find((node) => node.classList.contains("wind-transition-sheet-shadow"));
const expectedStripCount = 10;

assert.equal(result.player.owner, strips[0], "Wind's lifecycle must be driven by the first compositor strip");
assert.equal(underlay.style.visibility, "hidden", "the expensive full outgoing DOM must not paint behind the mesh");
assert.equal(strips.length, expectedStripCount, "high-quality Wind must use ten smooth connected facets");
assert.equal(result.metrics.coherentSheet, true, "Wind metrics must identify the physical continuous-sheet path");
assert.ok(
  new Set(strips.map((strip) => strip.style.left)).size === expectedStripCount,
  "each wind band must occupy a distinct source slice",
);
assert.ok(
  strips.every((strip) => strip.animations.some((animation) => animation.frames.some((frame) => (
    /^matrix3d\(/i.test(frame.transform || "")
  )))),
  "every wind band must use the shared-boundary projective surface",
);
assert.ok(
  strips.every((strip) => strip.style.transformOrigin === "0 0"),
  "every projective facet must transform from its source top-left corner",
);
assert.ok(
  strips.every((strip) => strip.animations.every((animation) => animation.frames.every((frame) => !("filter" in frame)))),
  "wind bands must never allocate per-strip blur/brightness surfaces",
);
assert.ok(
  strips.every((strip) => strip.children[0]?.style.visibility === "visible"),
  "fallback strip textures must undo the hidden underlay visibility inherited by cloning",
);

const motionAnimations = strips.map((strip) => strip.animations[0]);
const expectedOffsets = motionAnimations[0].frames.map((frame) => frame.offset);
assert.ok(
  motionAnimations.every((animation) => (
    JSON.stringify(animation.frames.map((frame) => frame.offset)) === JSON.stringify(expectedOffsets)
  )),
  "all wind strips must share one physical timeline instead of staggered flight paths",
);
assert.ok(
  motionAnimations.every((animation) => animation.frames.every((frame) => frame.opacity === 1)),
  "the outgoing page must stay opaque until its physical sheet leaves the viewport",
);
assert.ok(
  motionAnimations.every((animation) => animation.frames.every((frame) => (
    /^matrix3d\(/i.test(frame.transform || "")
    && !/(?:translate|rotate)/i.test(frame.transform || "")
  ))),
  "the continuous sheet must use only projective matrices, never independent strip flight transforms",
);

const forward = context.windSheetStateAt(.86, expectedStripCount, 1280, 720);
assert.ok(forward.travelX > 1280, "directionless native Wind must carry the old sheet off the right edge");
assert.ok(forward.travelY < -720 * .5, "native Wind must accelerate the old sheet upward as it leaves");
const reverse = context.windSheetStateAt(.86, expectedStripCount, 1280, 720, "left");
assert.ok(reverse.travelX < -1280, "an explicit left direction must mirror the coherent sheet");

function matrixValues(transform) {
  const match = /^matrix3d\(([^)]+)\)$/i.exec(transform || "");
  assert.ok(match, `expected a finite matrix3d transform, received ${transform}`);
  const values = match[1].split(",").map(Number);
  assert.equal(values.length, 16, "projective transforms must contain sixteen matrix values");
  assert.ok(values.every(Number.isFinite), "projective transforms must never contain NaN or Infinity");
  return values;
}

for (const animation of motionAnimations) {
  animation.frames.forEach((frame) => matrixValues(frame.transform));
}

const identity = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
const stripWidth = 1280 / expectedStripCount;
const flatSheet = context.windSheetStateAt(0, expectedStripCount, 1280, 720);
flatSheet.poses.forEach((pose, index) => {
  const values = matrixValues(context.windProjectiveTransform(pose, stripWidth, 720, index * stripWidth));
  values.forEach((value, valueIndex) => {
    assert.ok(Math.abs(value - identity[valueIndex]) < 1e-8, `p=0 strip ${index} must start at identity`);
  });
});

// Every neighbouring projective quadrilateral is built from the exact same
// top and bottom boundary points. This stronger invariant keeps the whole
// vertical seam closed, rather than joining only the strips' centre lines.
for (const offset of expectedOffsets) {
  const sheet = context.windSheetStateAt(offset, expectedStripCount, 1280, 720);
  assert.ok(sheet.poses.every((pose) => Math.abs(pose.angle) <= 35), `all source bend angles must remain paper-like at ${offset}`);
  for (let index = 0; index < sheet.poses.length - 1; index += 1) {
    const pose = sheet.poses[index];
    const next = sheet.poses[index + 1];
    assert.equal(pose.quad[1], next.quad[0], `top seam ${index}/${index + 1} must share one boundary object at ${offset}`);
    assert.equal(pose.quad[2], next.quad[3], `bottom seam ${index}/${index + 1} must share one boundary object at ${offset}`);
    assert.equal(pose.quad[1].x, next.quad[0].x);
    assert.equal(pose.quad[1].y, next.quad[0].y);
    assert.equal(pose.quad[2].x, next.quad[3].x);
    assert.equal(pose.quad[2].y, next.quad[3].y);
  }
}

assert.equal(stage.style.opacity, "1", "the incoming page must remain fully opaque below the moving sheet");
assert.equal(stage.animations.length, 0, "Wind must reveal the incoming page geometrically, not by cross-fading it");
assert.equal(lighting.length, strips.length, "each connected sheet facet must carry attached fold lighting");
assert.ok(
  lighting.every((node) => node.parentElement?.classList.contains("wind-transition-strip")),
  "fold lighting must move with its owning paper facet",
);
assert.ok(
  lighting.every((node) => node.animations[0].frames.some((frame) => Number(frame.opacity) > 0)),
  "fold lighting must respond to non-flat portions of the sheet",
);
assert.ok(shadowFrame && sheetShadow, "Wind must include a cast shadow bound to the departing sheet");
assert.equal(sheetShadow.parentElement, shadowFrame);
assert.ok(sheetShadow.animations[0].frames.some((frame) => Number(frame.opacity) > 0));
const shadowPositions = sheetShadow.animations[0].frames.map((frame) => {
  const match = /translate3d\((-?[\d.]+)px,(-?[\d.]+)px,/i.exec(frame.transform || "");
  assert.ok(match, "every cast-shadow frame must expose its sheet-bound position");
  return { x: Number(match[1]), y: Number(match[2]) };
});
assert.ok(
  shadowPositions.at(-1).x > shadowPositions[0].x
    && shadowPositions.at(-1).y < shadowPositions[0].y,
  "the cast shadow must follow the sheet's rightward and upward displacement",
);
assert.ok(
  result.animations.every((animation) => animation.startTime === context.document.timeline.currentTime + 1),
  "all facet, lighting, and shadow animations must share one compositor start time",
);
assert.match(styleSource, /\.wind-transition-strip\b/);
assert.match(styleSource, /\.wind-transition-lighting\b[^}]*linear-gradient/i);
assert.match(styleSource, /\.wind-transition-sheet-shadow\b[^}]*radial-gradient/i);
assert.doesNotMatch(styleSource, /\.wind-transition-(?:lighting|sheet-shadow)\s*\{[^}]*filter\s*:/i);

result.cleanup();
assert.ok(strips.every((strip) => strip.removed), "Wind cleanup must remove every cloned old-page band");
assert.equal(mesh.removed, true, "Wind cleanup must remove the fallback sheet mesh");
assert.equal(shadowFrame.removed, true, "Wind cleanup must remove the sheet-bound shadow frame");

console.log("wind transition runtime tests passed");
