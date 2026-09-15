"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");

function sourceBetween(startMarker, endMarker) {
  const start = appSource.indexOf(startMarker);
  const end = appSource.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, `${startMarker} must remain extractable`);
  return appSource.slice(start, end);
}

assert.match(
  appSource,
  /const PRESENTATION_MIN_FPS\s*=\s*30\s*;/,
  "the slideshow runtime must publish a 30 FPS minimum",
);

const summaryContext = {};
vm.runInNewContext(
  [
    "const PRESENTATION_MIN_FPS = 30;",
    "const PRESENTATION_FRAME_BUDGET_MS = 1000 / PRESENTATION_MIN_FPS;",
    "const PRESENTATION_DROPPED_FRAME_MS = 36;",
    "const PRESENTATION_MAX_FRAME_MS = 67;",
    sourceBetween("function summarizePresentationFrames", "\nfunction updatePresentationPerformance"),
    "this.summarize = summarizePresentationFrames;",
  ].join("\n"),
  summaryContext,
);

function summarize(milliseconds, frameCount = 120) {
  return summaryContext.summarize(Array.from({ length: frameCount }, () => milliseconds));
}

assert.equal(summarize(1000 / 60).passed, true, "60 Hz animation must pass the 30 FPS budget");
assert.equal(summarize(25).passed, true, "a stable 40 FPS animation must pass the 30 FPS budget");
assert.equal(summarize(33.3).passed, true, "a stable nominal 30 Hz stream must pass");
assert.equal(
  summarize(33.8).passed,
  true,
  "normal 30 Hz timestamp jitter up to 33.8 ms must not trigger low-quality fallback",
);
assert.equal(summarize(40).passed, false, "a stable 25 FPS animation must fail the 30 FPS budget");

const longFrame = Array.from({ length: 119 }, () => 1000 / 60);
longFrame.push(68);
const longFrameSummary = summaryContext.summarize(longFrame);
assert.equal(longFrameSummary.passed, false, "a frame longer than two 30 FPS budgets must fail the gate");
assert.ok(longFrameSummary.maxFrameMs >= 68, "the long frame must remain visible in diagnostics");

const transitionRuntime = sourceBetween("function runSlideTransition", "\nfunction settleSlideTransition");
assert.match(
  transitionRuntime,
  /beginPresentationFrameProbe\s*\(/,
  "every native slide transition must start a presentation frame probe",
);
assert.match(
  transitionRuntime,
  /\.finish\s*\(/,
  "the slide-transition probe must settle with the transition lifecycle",
);

const animationRuntime = sourceBetween("function runPresentationAnimationBatch", "\nfunction finishPresentationAnimationBatch");
assert.match(
  animationRuntime,
  /beginPresentationFrameProbe\s*\(/,
  "every native animation batch must start a presentation frame probe",
);
assert.match(
  appSource,
  /globalThis\.UniPptDiagnostics\s*=\s*Object\.freeze\s*\(/,
  "the measured FPS samples must remain inspectable during real 9.pptx playback",
);

const nativeQualityContext = {
  presentationQualityTier() { return "low"; },
  animationFrames() {
    return Array.from({ length: 12 }, (_, index) => ({
      offset: index / 11,
      maskImage: `mask-${index}`,
      clipPath: `inset(${index}%)`,
      opacity: index / 11,
    }));
  },
};
vm.runInNewContext(
  `${sourceBetween("function presentationAnimationFrames", "\nfunction animationFrames")}\nthis.adjustNativeFrames = presentationAnimationFrames;`,
  nativeQualityContext,
);
const lowTierFrames = nativeQualityContext.adjustNativeFrames({ class: "entrance" }, {});
assert.ok(lowTierFrames.length <= 4, "low-tier native animation must cap repaint keyframes");
assert.ok(
  lowTierFrames.some((frame) => frame.maskImage != null),
  "low-tier native animation must preserve the PowerPoint mask instead of degrading to a fade",
);

console.log("presentation performance tests passed");
