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

const context = {
  state: {
    deck: {
      slides: [
        {
          inheritedAnimations: [],
          animations: [
            { id: "a", order: 0, trigger: "onClick", durationMs: 100 },
            { id: "b", order: 1, trigger: "withPrevious", durationMs: 80 },
            { id: "c", order: 2, trigger: "afterPrevious", durationMs: 60 },
            { id: "d", order: 3, trigger: "onClick", durationMs: 40 },
          ],
        },
        {
          inheritedAnimations: [],
          animations: [
            { id: "e", order: 0, trigger: "onClick", durationMs: 50 },
          ],
        },
      ],
    },
    presenterSlide: 0,
    presenterAnimationCursor: 0,
    presenterRangeStart: 0,
    presenterRangeEnd: 1,
    presenterMode: "show",
    presenterTimers: [],
    presenterPlayers: [],
    presenterPlaybackToken: 0,
    presenterAnimationTargetCursor: null,
    presenterAnimationBatchStartCursor: null,
    slideShowSettings: { loop: false },
  },
  renderCalls: [],
  closeCalls: 0,
  boundaryNotices: 0,
  nextTimerId: 1,
  pendingTimers: new Map(),
  setTimeout(callback, delay = 0) {
    const id = context.nextTimerId++;
    context.pendingTimers.set(id, { callback, delay });
    return id;
  },
  clearTimeout(id) {
    context.pendingTimers.delete(id);
  },
  recordRehearsalTiming() {},
  animateSceneEffect() {},
  beginPresentationFrameProbe() {
    return { finish() { return null; } };
  },
  cancelPresentationFrameProbes() {},
  applyCompletedAnimationState() {},
  updatePresentationCount() {},
  $(selector) {
    assert.equal(selector, "#presentStage");
    return {};
  },
  renderPresentation(playTransition = false, options = {}) {
    context.state.presenterTimers.forEach(context.clearTimeout);
    context.state.presenterTimers = [];
    context.state.presenterPlayers = [];
    context.state.presenterAnimationTargetCursor = null;
    context.state.presenterAnimationBatchStartCursor = null;
    const animations = context.presentationAnimations();
    const restoreCursor = Math.max(0, Math.min(
      animations.length,
      Math.trunc(Number(options.restoreCursor) || 0),
    ));
    context.state.presenterAnimationCursor = restoreCursor;
    context.renderCalls.push({
      slide: context.state.presenterSlide,
      playTransition,
      restoreCursor,
      runInitialAutomatic: options.runInitialAutomatic,
    });
  },
  closePresentation() {
    context.closeCalls += 1;
  },
  hidePresentationBoundaryNotice() {},
  showPresentationBoundaryNotice() {
    context.boundaryNotices += 1;
  },
};

vm.runInNewContext(
  [
    sourceBetween("function scheduleAnimationBatch", "\nfunction animationEffectLabel"),
    sourceBetween("function stepPresentation", "\nfunction renderPresentation"),
    sourceBetween("function presentationAnimations", "\nfunction presentationTransitionLead"),
    sourceBetween("function runPresentationAnimationBatch", "\nfunction advanceToNextPresentationSlide"),
    sourceBetween("function advanceToNextPresentationSlide", "\nfunction advancePresentation"),
    sourceBetween("function advancePresentation", "\nfunction handleKeyboard"),
    "this.presentationAnimations = presentationAnimations;",
    "this.advancePresentation = advancePresentation;",
    "this.stepPresentation = stepPresentation;",
  ].join("\n"),
  context,
);

context.advancePresentation();
assert.equal(context.state.presenterSlide, 0);
assert.equal(context.state.presenterAnimationCursor, 0, "an in-flight click group does not advance the settled cursor");

context.advancePresentation();
assert.equal(context.state.presenterSlide, 0);
assert.equal(context.state.presenterAnimationCursor, 2, "a second right-arrow completes only the current navigation position");

context.advancePresentation();
assert.equal(context.state.presenterSlide, 0);
assert.equal(context.state.presenterAnimationCursor, 2, "the AfterPrevious position starts without leaving the slide");

context.advancePresentation();
assert.equal(context.state.presenterSlide, 0);
assert.equal(context.state.presenterAnimationCursor, 3, "completing AfterPrevious advances one navigation position");

context.advancePresentation();
assert.equal(context.state.presenterSlide, 0);
assert.equal(context.state.presenterAnimationCursor, 3, "the final OnClick position starts in flight on the current slide");

context.advancePresentation();
assert.equal(context.state.presenterSlide, 0);
assert.equal(context.state.presenterAnimationCursor, 4, "completing the final position still stays on the current slide");

context.advancePresentation();
assert.equal(context.state.presenterSlide, 1, "only the input after the final settled group changes slides");
assert.equal(context.state.presenterAnimationCursor, 0);

context.advancePresentation();
assert.equal(context.state.presenterAnimationCursor, 0, "the next slide's group starts in flight");
context.advancePresentation();
assert.equal(context.state.presenterAnimationCursor, 1);
context.stepPresentation(-1);
assert.equal(context.state.presenterSlide, 1);
assert.equal(context.state.presenterAnimationCursor, 0, "left-arrow rolls back the current slide's last group");

context.stepPresentation(-1);
assert.equal(context.state.presenterSlide, 0, "left-arrow at page start returns to the previous slide");
assert.equal(context.state.presenterAnimationCursor, 4, "the previous slide is restored at its final build state");

context.stepPresentation(-1);
assert.equal(context.state.presenterAnimationCursor, 3, "a second left-arrow rolls back exactly one previous click group");

context.state.presenterSlide = 1;
context.state.presenterAnimationCursor = 1;
context.advancePresentation();
assert.equal(context.closeCalls, 0, "manual input beyond the final slide keeps a non-looping show open");
assert.equal(context.boundaryNotices, 1, "manual input beyond the final slide shows the end notice");

// Source-fidelity guard for an externally supplied presentation fixture.
// Raw OOXML inspection found evt="onClick"=0 and nodeType="clickEffect"=0
// on all 27 slides. PowerPoint's own MainSequence reported only TriggerType
// 2/3 (WithPrevious/AfterPrevious); slide 4 is 2 WithPrevious + 25
// AfterPrevious. Those effects are one automatic onBegin stream and must not
// be invented as 27 keyboard click groups.
const ppt9Slide4AutomaticStream = [
  ...Array.from({ length: 25 }, () => ({ trigger: "afterPrevious", durationMs: 1 })),
  ...Array.from({ length: 2 }, () => ({ trigger: "withPrevious", durationMs: 1 })),
];
const automaticBatch = context.scheduleAnimationBatch(ppt9Slide4AutomaticStream, 0);
assert.equal(automaticBatch.nextIndex, 27);
assert.deepEqual(
  JSON.parse(JSON.stringify(context.animationBatchRanges(ppt9Slide4AutomaticStream))),
  [{ startIndex: 0, endIndex: 27 }],
  "AfterPrevious/WithPrevious must remain in one source automatic click stream",
);
assert.equal(
  context.animationNavigationRanges(ppt9Slide4AutomaticStream).length,
  25,
  "the same automatic stream exposes 25 entry-level navigation positions",
);

console.log("presentation flow state tests passed");
