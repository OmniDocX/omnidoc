"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const styleSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");
const sharedTransitionSource = fs.readFileSync(
  path.join(__dirname, "presentation_transition_runtime.js"),
  "utf8",
);

const sharedTransitionContext = {};
vm.runInNewContext(sharedTransitionSource, sharedTransitionContext);
const sharedTransitions = sharedTransitionContext.UniPptPresentationTransitions;
assert.equal(
  sharedTransitions.resolveDirection({}, "pagecurldouble", { slideIndex: 3 }),
  "left",
  "a directionless transition into even slide 4 must turn the right leaf to the left",
);
assert.equal(
  sharedTransitions.resolveDirection({}, "pagecurldouble", { slideIndex: 4 }),
  "left",
  "a directionless native double curl must keep turning the right leaf to the left",
);
assert.equal(
  sharedTransitions.resolveDirection({ direction: "right" }, "pagecurldouble", { slideIndex: 3 }),
  "right",
  "an explicit OOXML direction must override the native directionless default",
);

function topLevelFunctionSource(name) {
  const start = appSource.indexOf(`function ${name}`);
  assert.ok(start >= 0, `${name} must be implemented as a testable named function`);
  const end = appSource.indexOf("\nfunction ", start + `function ${name}`.length);
  return appSource.slice(start, end < 0 ? appSource.length : end);
}

function loadStripPlanner() {
  const runtimePath = path.join(__dirname, "page_curl_runtime.js");
  if (fs.existsSync(runtimePath)) {
    const runtime = require(runtimePath);
    assert.equal(
      typeof runtime.buildPageCurlStripPlan,
      "function",
      "page_curl_runtime.js must export buildPageCurlStripPlan",
    );
    return runtime.buildPageCurlStripPlan;
  }

  const source = topLevelFunctionSource("buildPageCurlStripPlan");
  const context = {
    clamp(value, minimum, maximum) {
      return Math.max(minimum, Math.min(maximum, value));
    },
  };
  vm.runInNewContext(
    `${source}\nthis.buildPageCurlStripPlan = buildPageCurlStripPlan;`,
    context,
  );
  assert.equal(typeof context.buildPageCurlStripPlan, "function");
  return context.buildPageCurlStripPlan;
}

function loadTopLevelFunction(name) {
  const source = topLevelFunctionSource(name);
  const dependencies = name === "pageCurlGeometryAt"
    ? topLevelFunctionSource("pageCurlMonotoneValue")
    : "";
  const context = {
    clamp(value, minimum, maximum) {
      return Math.max(minimum, Math.min(maximum, value));
    },
  };
  vm.runInNewContext(`${dependencies}\n${source}\nthis.loaded = ${name};`, context);
  assert.equal(typeof context.loaded, "function", `${name} must be loadable in isolation`);
  return context.loaded;
}

function planStrips(plan) {
  if (Array.isArray(plan)) return plan;
  for (const key of ["strips", "segments"]) {
    if (Array.isArray(plan?.[key])) return plan[key];
  }
  for (const key of ["pages", "leaves", "surfaces"]) {
    if (!Array.isArray(plan?.[key])) continue;
    return plan[key].flatMap((surface) => {
      if (Array.isArray(surface?.strips)) return surface.strips;
      if (Array.isArray(surface?.segments)) return surface.segments;
      return [surface];
    });
  }
  return [];
}

function collectGeometryValues(value, property = "", output = []) {
  if (typeof value === "number") {
    if (/angle|rotate|curve|bend|depth|translate.?z|offset.?z/i.test(property)) {
      output.push(Number(value.toFixed(5)));
    }
    return output;
  }
  if (typeof value === "string") {
    if (/transform/i.test(property) || /rotateY|translateZ/i.test(value)) {
      for (const match of value.matchAll(/(?:rotateY|translateZ)\(\s*(-?\d+(?:\.\d+)?)/gi)) {
        output.push(Number(Number(match[1]).toFixed(5)));
      }
    }
    return output;
  }
  if (Array.isArray(value)) {
    value.forEach((entry) => collectGeometryValues(entry, property, output));
    return output;
  }
  if (value && typeof value === "object") {
    Object.entries(value).forEach(([key, entry]) => collectGeometryValues(entry, key, output));
  }
  return output;
}

const buildPageCurlStripPlan = loadStripPlanner();
const pageCurlGeometryAt = loadTopLevelFunction("pageCurlGeometryAt");
const pageCurlStripPoseAt = loadTopLevelFunction("pageCurlStripPoseAt");
const doublePlan = buildPageCurlStripPlan("pagecurldouble", "left", 64);
const strips = planStrips(doublePlan);
const semantics = JSON.stringify(doublePlan).toLowerCase();
const geometryValues = new Set(collectGeometryValues(doublePlan));

assert.equal(strips.length, 24, "pageCurlDouble must spend its 24 compositor strips on the moving half-page only");
assert.equal(
  planStrips(buildPageCurlStripPlan("pagecurldouble", "left", 1)).length,
  16,
  "adaptive quality may reduce the half-page mesh, but never below the continuity floor",
);
assert.equal(
  planStrips(buildPageCurlStripPlan("pagecurl", "left", 0)).length,
  14,
  "a zero adaptive budget must select the minimum mesh instead of expanding back to the maximum",
);
assert.equal(doublePlan.topology, "top-down-open-book-even-leaf");
assert.ok(
  strips.every((strip) => strip.left >= .5 && strip.left + strip.width <= 1.00001),
  "the front texture must come exclusively from the outgoing right/even page",
);
assert.ok(
  strips.every((strip) => strip.backLeft >= 0 && strip.backLeft + strip.width <= .50001),
  "the reverse texture must come exclusively from the incoming left page",
);
assert.match(semantics, /front/, "pageCurlDouble must model the visible front of the turning sheet");
assert.match(semantics, /back/, "pageCurlDouble must model the reverse side of the turning sheet");
assert.match(semantics, /shadow/, "pageCurlDouble must carry fold/spine shadow semantics");
assert.ok(
  geometryValues.size >= 4,
  "pageCurlDouble strips must have non-uniform curvature/rotation/depth values",
);

const quarterCurl = pageCurlGeometryAt(.333, "pagecurldouble", "left");
const spineCurl = pageCurlGeometryAt(.444, "pagecurldouble", "left");
const middleCurl = pageCurlGeometryAt(.556, "pagecurldouble", "left");
const sweptCurl = pageCurlGeometryAt(.667, "pagecurldouble", "left");
const lateCurl = pageCurlGeometryAt(.778, "pagecurldouble", "left");
assert.ok(
  quarterCurl.bandWidth >= .45 && quarterCurl.bandWidth <= .5,
  "the official double-page curl must raise almost the full right leaf, not a narrow seam",
);
assert.ok(
  spineCurl.flatBoundary === .5 && spineCurl.outerBoundary >= .91 && spineCurl.outerBoundary <= .95,
  "the first 44.4% must keep the centre spine fixed while the right leaf bows inward",
);
assert.ok(
  middleCurl.bandWidth >= .38 && middleCurl.bandWidth <= .42,
  "the middle frame must retain about two fifths of the slide as visible curled paper",
);
assert.ok(
  middleCurl.flatBoundary >= .42 && middleCurl.flatBoundary <= .46,
  "the middle fold must follow the measured PowerPoint spine trajectory",
);
assert.ok(
  lateCurl.bandWidth >= .23 && lateCurl.bandWidth <= .29,
  "the late cylindrical crest must remain visible while the printed front has turned away",
);
assert.ok(
  sweptCurl.outerBoundary >= .49 && sweptCurl.outerBoundary <= .54,
  "the two-thirds frame must follow the recorded cylindrical crest near the page centre",
);
const mirroredCurl = pageCurlGeometryAt(.556, "pagecurldouble", "right");
assert.ok(
  Math.abs(mirroredCurl.bandLeft - (1 - middleCurl.bandRight)) < .0001
    && Math.abs(mirroredCurl.bandRight - (1 - middleCurl.bandLeft)) < .0001,
  "rightward page curls must mirror the recorded leftward geometry exactly",
);
const transportedPose = pageCurlStripPoseAt(
  { center: .99, width: .02 },
  pageCurlGeometryAt(.667, "pagecurldouble", "left"),
  1280,
);
assert.ok(
  transportedPose.x < .42 && transportedPose.z > 20 && transportedPose.angle < -130,
  "curl material must translate left and rise in depth while turning; in-place rotation is not a page turn",
);
const earlyGeometry = pageCurlGeometryAt(.333, "pagecurldouble", "left");
assert.equal(
  pageCurlStripPoseAt({ center: 31.5 / 64, width: 1 / 64 }, earlyGeometry, 1280).active,
  false,
  "a strip ending exactly at the fixed centre hinge must stay on the flat page instead of duplicating half a strip",
);
assert.equal(
  pageCurlStripPoseAt({ center: 32.5 / 64, width: 1 / 64 }, earlyGeometry, 1280).active,
  true,
  "the first strip beginning at the centre hinge must form the moving leaf",
);

const transitionSource = topLevelFunctionSource("animateTopDownBookTransition");
const bookFramesSource = topLevelFunctionSource("topDownBookLeafFrames");
const synchronizeSource = topLevelFunctionSource("synchronizeTransitionAnimations");
const captureSource = topLevelFunctionSource("capturePresentationTransitionUnderlay");
assert.doesNotMatch(
  captureSource,
  /stage\.children\.length/,
  "background-only slides must still be captured as the outgoing page texture",
);
assert.match(
  transitionSource,
  /makeTopDownBookLeafFace\s*\(underlay[\s\S]*makeTopDownBookLeafFace\s*\(stage/,
  "the open book must use the outgoing even page on the front and incoming left page on the reverse",
);
assert.match(
  transitionSource,
  /underlay\.style\.clipPath\s*=\s*turnsRight\s*\?\s*"inset\(0 0 0 50%\)"\s*:\s*"inset\(0 50% 0 0\)"/,
  "the outgoing page must keep one statically clipped half while its even leaf turns",
);
assert.match(
  transitionSource,
  /pageCurlSpine\s*=\s*"fixed-50-percent"/,
  "the coherent page must remain attached to PowerPoint's fixed 50% book spine",
);
assert.match(
  transitionSource,
  /perspective:\s*`\$\{Math\.max\(1600,\s*stageWidth\s*\*\s*1\.65\)\.toFixed\(0\)\}px`/,
  "the top-down book needs real depth with a perspective floor of 1600px",
);
assert.match(
  transitionSource,
  /leaf\.append\(front,\s*back\)/,
  "one coherent leaf must carry exactly its physical front and reverse faces",
);
assert.match(
  transitionSource,
  /leaf\.animate\(samples\.map\([\s\S]*?opacity:\s*1,[\s\S]*?transform:\s*sample\.transform/,
  "the leaf must stay opaque throughout the physical turn instead of fading before it lands",
);
assert.match(
  transitionSource,
  /edgeX\s*=\s*stageWidth\s*\*\s*\.5[\s\S]*Math\.cos\(radians\)/,
  "the cast shadow must follow the moving free edge of the turning page",
);
assert.match(
  transitionSource,
  /synchronizeTransitionAnimations\(animations\)/,
  "all book, lighting and shadow animations must share one compositor start time",
);
assert.doesNotMatch(
  transitionSource,
  /page-book-leaf-segment|topDownBookSegmentPoses|segment\.animate|leafOpacity/,
  "the dedicated double-page preset must not regress to segmented strips or an end fade",
);
assert.match(
  bookFramesSource,
  /\[0,\s*0\][\s\S]*\[\.5,\s*90\][\s\S]*\[1,\s*180\]/,
  "the sampled trajectory must travel continuously from flat through vertical to fully landed",
);
assert.match(
  synchronizeSource,
  /animation\.startTime\s*=\s*sharedStartTime/,
  "WAAPI players must be phase-locked using one shared startTime",
);
assert.match(styleSource, /\.page-book-leaf-back\s*\{[^}]*rotateY\(180deg\)/, "the incoming left page must be the physical reverse face");
assert.doesNotMatch(
  styleSource,
  /\.page-book-leaf-texture\s*\{[^}]*backface-visibility/,
  "the printed texture must not self-cull after the reverse face's 180deg flip",
);
assert.match(
  styleSource,
  /\.page-book-leaf-face\s*\{[^}]*clip-path:\s*inset\(0\)/,
  "both physical faces must clip lighting without flattening the reverse texture",
);
assert.match(
  styleSource,
  /\.page-book-leaf-back\s*\{[^}]*backface-visibility:\s*visible/,
  "the reverse face must stay paintable after its local rotateY(180deg)",
);
assert.match(styleSource, /page-book-free-edge-shadow\.turns-right[\s\S]*linear-gradient\(270deg/, "rightward turns must mirror the free-edge shadow");
assert.doesNotMatch(styleSource, /page-book-free-edge-shadow[^}]*filter\s*:/i, "book shadows must avoid full-height blur surfaces");
// The highlight is swept with a transform rather than repainted stop by stop,
// so it must overhang the face it slides across.
assert.match(
  styleSource,
  /\.page-book-leaf-lighting\s*\{[^}]*left:-45%[^}]*right:-45%[^}]*will-change:transform,opacity/,
  "the swept lighting layer must overhang the leaf and stay on the compositor",
);

const topDownBookLeafFrames = loadTopLevelFunction("topDownBookLeafFrames");
const leftBookFrames = topDownBookLeafFrames("left");
const rightBookFrames = topDownBookLeafFrames("right");
assert.equal(leftBookFrames.length, 11, "the native double-page turn must use all eleven sampled angles");
assert.deepEqual(
  Array.from(leftBookFrames, (frame) => frame.angle),
  [0, 4, 16, 37, 66, 90, 119, 149, 169, 178, 180],
  "the coherent leaf must follow the measured PowerPoint angle trajectory",
);
assert.equal(leftBookFrames[0].transform, "rotateY(0.000deg)");
assert.equal(leftBookFrames.at(-1).transform, "rotateY(-180.000deg)");
assert.equal(rightBookFrames.at(-1).transform, "rotateY(180.000deg)", "reverse turns must mirror the same physical trajectory");
assert.ok(leftBookFrames.every((frame) => Number(frame.shadeOpacity) >= 0 && Number(frame.shadowOpacity) >= 0));

// The two faces must light up in turn rather than both glowing at once: the
// printed side while it still faces the room, the reverse only once the sheet
// has swung past vertical.
const flat = leftBookFrames[0];
const vertical = leftBookFrames[5];
const landed = leftBookFrames.at(-1);
assert.equal(Number(flat.frontShade), 0, "a page lying flat catches no grazing light");
assert.equal(Number(flat.backShade), 0);
assert.equal(Number(landed.frontShade), 0, "the turn ends with the sheet flat again");
assert.equal(Number(landed.backShade), 0);
assert.equal(Number(flat.frontOpacity), 1, "the printed side is fully visible while the leaf is still face-up");
assert.equal(Number(flat.backOpacity), 0, "the reverse must not show through a face-up leaf");
assert.ok(Number(vertical.frontOpacity) < .05, "both faces are culled at vertical instead of relying on backface-visibility");
assert.ok(Number(vertical.backOpacity) < .05);
assert.equal(Number(landed.frontOpacity), 0, "the printed side must be gone once the leaf has landed");
assert.equal(Number(landed.backOpacity), 1, "the incoming left page is the landed reverse");
assert.ok(
  Number(leftBookFrames[3].frontShade) > Number(leftBookFrames[3].backShade),
  "before vertical the room still sees the printed side",
);
assert.ok(
  Number(leftBookFrames[7].backShade) > Number(leftBookFrames[7].frontShade),
  "after vertical the reverse is the lit face",
);
// Edge-on, the view-dependent term drops out and each face sits at its own
// floor, so neither side is carrying a highlight it should not have.
assert.ok(
  Math.abs(Number(vertical.frontShade) - .34 * .4) < 1e-4,
  "at vertical the printed side falls back to its grazing floor",
);
assert.ok(
  Math.abs(Number(vertical.backShade) - .3 * .4) < 1e-4,
  "at vertical the reverse falls back to its grazing floor",
);
assert.ok(
  Number(leftBookFrames[4].frontShade) > Number(vertical.frontShade),
  "the front highlight peaks before the sheet reaches vertical",
);

// The highlight travels toward the spine as the sheet rises, and the two faces
// sweep in opposite directions because they point opposite ways.
assert.equal(Number(flat.frontSlide), 0);
assert.ok(Number(leftBookFrames[3].frontSlide) > 0, "a leftward turn sweeps its front highlight toward the spine");
assert.ok(Number(leftBookFrames[7].backSlide) < 0, "the reverse highlight sweeps the other way");
assert.ok(
  Number(rightBookFrames[3].frontSlide) === -Number(leftBookFrames[3].frontSlide),
  "mirrored turns mirror the highlight sweep",
);

// A sheet standing on its edge casts a narrow shadow and broadens again as it falls.
assert.ok(Number(vertical.shadowScale) < Number(flat.shadowScale), "the cast shadow narrows at vertical");
assert.ok(Number(landed.shadowScale) > Number(vertical.shadowScale), "and widens again once the page lands");
assert.ok(leftBookFrames.every((frame) => Number(frame.shadowScale) > 0));

class MockAnimation {
  constructor(frames, options) {
    this.frames = frames;
    this.options = options;
    this.playState = "running";
    this.startTime = null;
  }
  addEventListener() {}
  cancel() { this.playState = "idle"; }
}

class MockElement {
  constructor(tagName = "div") {
    this.tagName = tagName;
    this.className = "";
    this.dataset = {};
    this.style = {};
    this.children = [];
    this.parentElement = null;
    this.clientWidth = 0;
    this.clientHeight = 0;
    this.animations = [];
    this.classList = {
      add: (...names) => {
        const classes = new Set(this.className.split(/\s+/).filter(Boolean));
        names.forEach((name) => classes.add(name));
        this.className = [...classes].join(" ");
      },
      remove: (...names) => {
        const removed = new Set(names);
        this.className = this.className.split(/\s+/).filter((name) => name && !removed.has(name)).join(" ");
      },
    };
  }
  get nextSibling() {
    if (!this.parentElement) return null;
    const index = this.parentElement.children.indexOf(this);
    return this.parentElement.children[index + 1] || null;
  }
  append(...nodes) {
    nodes.forEach((node) => {
      node.parentElement = this;
      this.children.push(node);
    });
  }
  insertBefore(node, reference) {
    node.parentElement = this;
    const index = reference ? this.children.indexOf(reference) : -1;
    if (index < 0) this.children.push(node);
    else this.children.splice(index, 0, node);
  }
  remove() {
    if (!this.parentElement) return;
    const index = this.parentElement.children.indexOf(this);
    if (index >= 0) this.parentElement.children.splice(index, 1);
    this.parentElement = null;
  }
  removeAttribute() {}
  querySelectorAll() { return []; }
  cloneNode(deep = false) {
    const clone = new MockElement(this.tagName);
    clone.className = this.className;
    clone.style = { ...this.style };
    clone.clientWidth = this.clientWidth;
    clone.clientHeight = this.clientHeight;
    if (deep) this.children.forEach((child) => clone.append(child.cloneNode(true)));
    return clone;
  }
  animate(frames, options) {
    const animation = new MockAnimation(frames, options);
    this.animations.push(animation);
    return animation;
  }
}

function flattenElements(root, result = []) {
  for (const child of root.children) {
    result.push(child);
    flattenElements(child, result);
  }
  return result;
}

const presenter = new MockElement("main");
const underlay = new MockElement("section");
underlay.className = "present-stage slide-transition-underlay";
underlay.style = { width: "1280px", height: "720px", transform: "translate(-50%,-50%)" };
underlay.clientWidth = 1280;
underlay.clientHeight = 720;
const stage = new MockElement("section");
stage.className = "present-stage";
stage.style = { width: "1280px", height: "720px", transform: "translate(-50%,-50%)" };
stage.clientWidth = 1280;
stage.clientHeight = 720;
presenter.append(underlay, stage);

const runtimeContext = {
  clamp(value, minimum, maximum) {
    return Math.max(minimum, Math.min(maximum, value));
  },
  document: {
    timeline: { currentTime: 640 },
    createElement: (tagName) => new MockElement(tagName),
  },
  namespaceTransitionCloneIds(root) { return root; },
  presentationQualityTier() { return "high"; },
  runtimeResult: null,
};
runtimeContext.runSlideTransition = (_stage, create) => {
  runtimeContext.runtimeResult = create();
  return runtimeContext.runtimeResult.player;
};
for (const name of [
  "makeTopDownBookLeafFace", "topDownBookLeafFrames", "synchronizeTransitionAnimations",
  "animateTopDownBookTransition",
]) {
  vm.runInNewContext(topLevelFunctionSource(name), runtimeContext);
}
runtimeContext.animateTopDownBookTransition(stage, 1250, "left", underlay, "pagecurldouble");
const rendered = flattenElements(presenter);
const bookFrame = rendered.find((node) => node.className === "page-book-frame");
const turningLeaf = rendered.find((node) => node.className === "page-book-turning-leaf");
const leafSegments = rendered.filter((node) => node.className === "page-book-leaf-segment");
const leafFaces = rendered.filter((node) => /page-book-leaf-(?:front|back)(?:\s|$)/.test(node.className));
const freeEdgeShadow = rendered.find((node) => /(?:^|\s)page-book-free-edge-shadow(?:\s|$)/.test(node.className));
assert.ok(bookFrame, "the page-curl runtime must insert a top-down open book beside the presentation stage");
assert.equal(bookFrame.dataset.pageCurlTopology, "top-down-open-book-even-leaf");
assert.equal(bookFrame.dataset.pageCurlSpine, "fixed-50-percent");
assert.ok(parseFloat(bookFrame.style.perspective) >= 1600, "the book frame must preserve visible depth at every slide size");
assert.equal(underlay.style.clipPath, "inset(0 50% 0 0)", "a leftward turn must retain the outgoing left half statically");
assert.ok(turningLeaf, "the right/even page must be represented by one coherent physical leaf");
assert.equal(Number(turningLeaf.dataset.pageCurlStripCount), 1);
assert.equal(turningLeaf.dataset.pageCurlLeaf, "coherent-sheet");
assert.equal(turningLeaf.style.left, "50%");
assert.equal(turningLeaf.style.transformOrigin, "0 50%");
assert.equal(turningLeaf.children.length, 2, "the coherent leaf must have exactly one front and one reverse face");
assert.equal(leafSegments.length, 0, "the dedicated double-page runtime must not build segmented paper strips");
assert.equal(leafFaces.length, 2, "the single sheet must expose exactly two physical faces");
assert.deepEqual(
  leafFaces.map((node) => node.dataset.pageBookFace).sort(),
  ["incoming-left-page", "outgoing-even-page"],
);
assert.ok(leafFaces.every((node) => node.children.length === 2), "each face must carry one texture and one compositor lighting layer");
const frontFace = leafFaces.find((node) => node.dataset.pageBookFace === "outgoing-even-page");
const backFace = leafFaces.find((node) => node.dataset.pageBookFace === "incoming-left-page");
assert.equal(frontFace.animations[0].frames[0].opacity, "1.0000");
assert.equal(frontFace.animations[0].frames.at(-1).opacity, "0.0000");
assert.equal(backFace.animations[0].frames[0].opacity, "0.0000");
assert.equal(backFace.animations[0].frames.at(-1).opacity, "1.0000");
assert.ok(
  turningLeaf.animations[0].frames.every((frame) => !("filter" in frame)),
  "the coherent paper leaf must stay compositor-only and never allocate a page-sized filter surface",
);
const leafFrames = turningLeaf.animations[0].frames;
assert.equal(leafFrames.length, 11, "the runtime must retain all eleven native angle samples");
assert.ok(
  leafFrames.every((frame) => frame.opacity === 1 && /rotateY/.test(frame.transform)),
  "the half-page must remain fully opaque while rotating around its fixed spine",
);
assert.equal(leafFrames[0].transform, "rotateY(0.000deg)");
assert.equal(leafFrames[5].transform, "rotateY(-90.000deg)");
assert.equal(leafFrames.at(-1).transform, "rotateY(-180.000deg)");
assert.ok(freeEdgeShadow, "a coherent page turn still needs a moving free-edge cast shadow");
const shadowFrames = freeEdgeShadow.animations[0].frames;
assert.equal(shadowFrames.length, 11);
assert.notEqual(shadowFrames[0].transform, shadowFrames[5].transform);
assert.notEqual(shadowFrames[5].transform, shadowFrames.at(-1).transform);
assert.ok(Number(shadowFrames[5].opacity) > Number(shadowFrames[1].opacity), "the shadow must peak while the page stands near vertical");
assert.equal(runtimeContext.runtimeResult.animations.length, 7, "leaf, two faces, two lighting layers, spine and free-edge shadow must share one lifecycle");
assert.ok(
  runtimeContext.runtimeResult.animations.every((animation) => animation.startTime === 641),
  "every WAAPI player must receive the same document-timeline startTime",
);
runtimeContext.runtimeResult.cleanup();
assert.equal(flattenElements(presenter).filter((node) => /page-(?:curl|book)/.test(node.className)).length, 0, "transition cleanup must remove every temporary book layer");

console.log("page curl runtime tests passed");
