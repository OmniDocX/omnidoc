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

function createClock() {
  let nextId = 1;
  let now = 0;
  const pending = new Map();
  return {
    setTimeout(callback, delay = 0) {
      const id = nextId++;
      pending.set(id, { callback, at: now + Math.max(0, Number(delay) || 0) });
      return id;
    },
    clearTimeout(id) {
      pending.delete(id);
    },
    flushAll() {
      while (pending.size) {
        const [id, task] = [...pending.entries()].sort((a, b) => a[1].at - b[1].at)[0];
        pending.delete(id);
        now = task.at;
        task.callback();
      }
    },
    callbacks() {
      return [...pending.values()].map((task) => task.callback);
    },
    get size() {
      return pending.size;
    },
  };
}

const firstSlideAnimations = [
  { id: "a", order: 0, trigger: "onClick", durationMs: 100, delayMs: 0 },
  { id: "b", order: 1, trigger: "withPrevious", durationMs: 80, delayMs: 0 },
  { id: "c", order: 2, trigger: "onClick", durationMs: 60, delayMs: 0 },
];

function makeContext(firstAnimations = firstSlideAnimations) {
  const clock = createClock();
  const stage = {};
  const context = {
    state: {
      deck: {
        slides: [
          { inheritedAnimations: [], animations: firstAnimations },
          { inheritedAnimations: [], animations: [] },
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
    countUpdates: [],
    appliedAnimations: [],
    closeCalls: 0,
    boundaryNotices: 0,
    clock,
    setTimeout: clock.setTimeout,
    clearTimeout: clock.clearTimeout,
    recordRehearsalTiming() {},
    presentationAnimations() {
      const slide = context.state.deck.slides[context.state.presenterSlide];
      return [
        ...(slide.inheritedAnimations || []),
        ...(slide.animations || []),
      ].sort((a, b) => a.order - b.order);
    },
    animateSceneEffect(_stage, animation, offset = 0) {
      return Math.max(0, Number(offset) || 0) + Math.max(1, Number(animation.durationMs) || 500);
    },
    beginPresentationFrameProbe() {
      return { finish() { return null; } };
    },
    cancelPresentationFrameProbes() {},
    applyCompletedAnimationState(_stage, animation) {
      context.appliedAnimations.push(animation.id);
    },
    restorePresentationAnimationState() {},
    updatePresentationCount() {
      context.countUpdates.push({
        slide: context.state.presenterSlide,
        cursor: context.state.presenterAnimationCursor,
      });
    },
    renderPresentation(playTransition = false, options = {}) {
      context.state.presenterTimers.forEach(clock.clearTimeout);
      context.state.presenterTimers = [];
      context.state.presenterPlayers = [];
      context.state.presenterAnimationTargetCursor = null;
      context.state.presenterAnimationBatchStartCursor = null;
      for (const key of Object.keys(context.state)) {
        if (/presenter.*(?:in.?flight|pending|scheduled)/i.test(key)) {
          context.state[key] = null;
        }
      }
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
        reverseTransition: options.reverseTransition,
      });
    },
    closePresentation() {
      context.closeCalls += 1;
    },
    hidePresentationBoundaryNotice() {},
    showPresentationBoundaryNotice() {
      context.boundaryNotices += 1;
    },
    handleBrowserShortcut() {
      return false;
    },
    isTextEditingTarget() {
      return false;
    },
    togglePresentationMute() {},
    showPresentationControls() {},
    $: (selector) => {
      if (selector === "#presentStage") return stage;
      if (selector === "#exportMenuPopup") return { hidden: true };
      if (selector === "#slideContextMenu") return { hidden: true };
      if (selector === "#udocStructureDialog") return { open: false };
      if (selector === "#formulaDialog") return { open: false };
      if (selector === "#presenter") return { hidden: false };
      throw new Error(`unexpected selector ${selector}`);
    },
  };

  vm.runInNewContext(
    [
      sourceBetween("function scheduleAnimationBatch", "\nfunction animationEffectLabel"),
      sourceBetween("function stepPresentation", "\nfunction renderPresentation"),
      sourceBetween("function runPresentationAnimationBatch", "\nfunction handleKeyboard"),
      sourceBetween("function handleKeyboard", "\nfunction handleBrowserShortcut"),
      "this.advancePresentation = advancePresentation;",
      "this.stepPresentation = stepPresentation;",
      "this.jumpPresentationPage = jumpPresentationPage;",
      "this.reverseTransitionDirection = reverseTransitionDirection;",
      "this.handleKeyboard = handleKeyboard;",
    ].join("\n"),
    context,
  );
  return context;
}

function keyboardEvent(key, modifiers = {}) {
  return {
    key,
    code: key === " " ? "Space" : key,
    ctrlKey: Boolean(modifiers.ctrlKey),
    metaKey: Boolean(modifiers.metaKey),
    altKey: Boolean(modifiers.altKey),
    shiftKey: Boolean(modifiers.shiftKey),
    defaultPrevented: false,
    propagationStopped: false,
    preventDefault() {
      this.defaultPrevented = true;
    },
    stopPropagation() {
      this.propagationStopped = true;
    },
    target: {
      closest() {
        return null;
      },
    },
  };
}

{
  const context = makeContext();
  context.advancePresentation();
  assert.equal(context.state.presenterAnimationTargetCursor, 2, "fixture starts with an in-flight animation position");

  const ctrlEnd = keyboardEvent("End", { ctrlKey: true });
  context.handleKeyboard(ctrlEnd);
  assert.equal(ctrlEnd.defaultPrevented, true, "Ctrl+End must not reach native browser page navigation");
  assert.equal(context.state.presenterSlide, 0, "Ctrl+End settles the current page before changing slides");
  assert.equal(context.state.presenterAnimationCursor, 3, "Ctrl+End fast-forwards every animation node at once");
  assert.equal(context.state.presenterAnimationTargetCursor, null, "the in-flight batch is cancelled by the settle");
  assert.equal(context.clock.size, 0, "the abandoned animation timers are cleared");
  assert.deepEqual(context.appliedAnimations, ["a", "b", "c"], "the whole page is applied at its final state");
  const ctrlEndAgain = keyboardEvent("End", { ctrlKey: true });
  context.handleKeyboard(ctrlEndAgain);
  assert.equal(context.state.presenterSlide, 1, "a second Ctrl+End on a settled page advances to the next slide");
  assert.deepEqual(
    context.renderCalls.at(-1),
    {
      slide: 1,
      playTransition: true,
      restoreCursor: 0,
      runInitialAutomatic: undefined,
      reverseTransition: undefined,
    },
    "the advanced slide renders from its initial state before being settled",
  );
}

{
  const context = makeContext();
  context.advancePresentation();
  const ctrlHome = keyboardEvent("Home", { ctrlKey: true });
  context.handleKeyboard(ctrlHome);
  assert.equal(ctrlHome.defaultPrevented, true, "Ctrl+Home must not reach native browser page navigation");
  assert.equal(context.state.presenterSlide, 0, "Ctrl+Home never changes slides");
  assert.equal(context.state.presenterAnimationCursor, 3, "Ctrl+Home settles the in-flight page instantly");
  assert.deepEqual(context.appliedAnimations, ["a", "b", "c"], "Ctrl+Home applies every node final state");

  const ctrlHomeAgain = keyboardEvent("Home", { ctrlKey: true });
  context.handleKeyboard(ctrlHomeAgain);
  assert.equal(context.state.presenterSlide, 0, "Ctrl+Home on the first settled page stays put");
}

{
  const context = makeContext();
  context.state.presenterSlide = 1;
  const ctrlHome = keyboardEvent("Home", { ctrlKey: true });
  context.handleKeyboard(ctrlHome);
  assert.equal(ctrlHome.defaultPrevented, true, "Ctrl+Home must not reach native browser page navigation");
  assert.equal(context.state.presenterSlide, 0, "Ctrl+Home skips to the previous slide immediately");
  assert.equal(context.state.presenterAnimationCursor, firstSlideAnimations.length, "the previous slide is restored at its final build state");
  assert.deepEqual(
    context.renderCalls.at(-1),
    {
      slide: 0,
      playTransition: true,
      restoreCursor: firstSlideAnimations.length,
      runInitialAutomatic: false,
      reverseTransition: true,
    },
    "backward page skipping must request a reversed transition without replaying automatic builds",
  );
  assert.equal(context.reverseTransitionDirection("left"), "right");
  assert.equal(context.reverseTransitionDirection("right"), "left");
  assert.equal(context.reverseTransitionDirection("up"), "down");
  assert.equal(context.reverseTransitionDirection("down"), "up");
  assert.equal(context.reverseTransitionDirection("in"), "out");
  assert.equal(context.reverseTransitionDirection("out"), "in");
}

{
  const context = makeContext();

  context.advancePresentation();
  assert.equal(context.state.presenterSlide, 0, "scheduling the first batch must stay on its slide");
  assert.equal(
    context.state.presenterAnimationCursor,
    0,
    "the settled animation cursor must not jump to the batch end while that batch is in flight",
  );

  context.advancePresentation();
  assert.equal(context.state.presenterSlide, 0, "forward first completes an in-flight batch without changing slides");
  assert.equal(context.state.presenterAnimationCursor, 2, "forward settles exactly the current click batch");

  context.advancePresentation();
  assert.equal(context.state.presenterSlide, 0, "scheduling the final batch must remain on the current slide");
  assert.equal(context.state.presenterAnimationCursor, 2, "the final in-flight batch must retain its start cursor");

  context.advancePresentation();
  assert.equal(context.state.presenterSlide, 0, "completing the final batch still remains on the current slide");
  assert.equal(context.state.presenterAnimationCursor, 3, "the final batch settles at the final animation node");

  context.advancePresentation();
  assert.equal(context.state.presenterSlide, 1, "only input after the final settled node changes slides");
}

{
  const context = makeContext();
  context.state.presenterSlide = 1;
  context.advancePresentation({ userInitiated: true });
  assert.equal(context.state.presenterSlide, 1, "manual input at the final playback point stays on the last slide");
  assert.equal(context.closeCalls, 0, "manual input at the end must not exit the slide show");
  assert.equal(context.boundaryNotices, 1, "manual input at the end shows the boundary notice");

  const ctrlEnd = keyboardEvent("End", { ctrlKey: true });
  context.handleKeyboard(ctrlEnd);
  assert.equal(context.state.presenterSlide, 1, "Ctrl+End at the last page remains on the last page");
  assert.equal(context.closeCalls, 0, "Ctrl+End at the last page must not exit the slide show");
  assert.equal(context.boundaryNotices, 2, "Ctrl+End retriggers the boundary notice");
}

{
  const context = makeContext();
  context.state.presenterSlide = 1;
  context.state.slideShowSettings.autoPlay = true;
  context.advancePresentation();
  assert.equal(context.closeCalls, 1, "automatic playback still closes at the end for recorder completion");
  assert.equal(context.boundaryNotices, 0, "automatic completion must not burn the notice into a recording");
}

{
  const context = makeContext();
  context.advancePresentation();
  context.stepPresentation(-1);
  assert.equal(context.state.presenterSlide, 0, "backward during a batch must not jump to the previous slide");
  assert.equal(context.state.presenterAnimationCursor, 0, "backward restores the in-flight batch start");
  assert.equal(context.renderCalls.at(-1)?.restoreCursor, 0, "backward rebuilds the slide at the batch start state");
}

{
  const context = makeContext();
  context.state.presenterAnimationCursor = 2;
  const home = keyboardEvent("Home");
  context.handleKeyboard(home);
  assert.equal(context.state.presenterSlide, 0);
  assert.equal(context.state.presenterAnimationCursor, 0, "Home walks backward one animation batch");
  assert.equal(home.defaultPrevented, true, "Home must not reach the browser's native navigation");

  const end = keyboardEvent("End");
  context.handleKeyboard(end);
  assert.equal(context.state.presenterSlide, 0, "End starts the next animation batch before changing slides");
  assert.equal(context.state.presenterAnimationCursor, 0, "End uses the same in-flight cursor as other forward controls");
  assert.equal(end.defaultPrevented, true, "End must not reach the browser's native navigation");

  const endAgain = keyboardEvent("End");
  context.handleKeyboard(endAgain);
  assert.equal(context.state.presenterSlide, 0, "a second End completes the current batch on the same slide");
  assert.equal(context.state.presenterAnimationCursor, 2);
}

{
  // 9.pptx slide 4: 27 effects, 25 AfterPrevious and two
  // WithPrevious companions. The source remains one automatic click stream,
  // but it exposes 25 reversible navigation positions.
  const slide4Animations = Array.from({ length: 27 }, (_, index) => ({
    id: `s4-${index + 1}`,
    order: index,
    trigger: index === 2 || index === 26 ? "withPrevious" : "afterPrevious",
    durationMs: 10,
    delayMs: 0,
  }));
  const context = makeContext(slide4Animations);
  const ranges = context.animationNavigationRanges(slide4Animations);
  assert.equal(ranges.length, 25);
  assert.deepEqual(JSON.parse(JSON.stringify(ranges[1])), { startIndex: 1, endIndex: 3 });
  assert.deepEqual(JSON.parse(JSON.stringify(ranges.at(-1))), { startIndex: 25, endIndex: 27 });

  for (const range of ranges) {
    assert.equal(context.state.presenterAnimationCursor, range.startIndex);
    context.advancePresentation();
    assert.equal(context.state.presenterSlide, 0);
    assert.equal(context.state.presenterAnimationCursor, range.startIndex, "starting a position keeps its settled cursor");
    assert.equal(context.state.presenterAnimationTargetCursor, range.endIndex);
    const staleCallbacks = context.clock.callbacks();

    context.advancePresentation();
    assert.equal(context.state.presenterSlide, 0, "finishing a position never crosses the page boundary");
    assert.equal(context.state.presenterAnimationCursor, range.endIndex);
    assert.equal(context.state.presenterAnimationTargetCursor, null);
    assert.equal(context.clock.size, 0, "fast-forward clears every timer for later automatic positions");
    assert.equal(context.state.presenterPlayers.length, 0, "fast-forward clears DOM animation players");

    for (const staleCallback of staleCallbacks) staleCallback();
    assert.equal(
      context.state.presenterAnimationCursor,
      range.endIndex,
      "generation-guarded stale callbacks cannot advance a rapid-key navigation cursor",
    );
  }
  context.advancePresentation();
  assert.equal(context.state.presenterSlide, 1, "only input after slide 4's final position advances to slide 5");
}

{
  // 9.pptx slide 24: its final AfterPrevious entry has two WithPrevious
  // companions, so all three must be one final navigation position.
  const slide24Animations = Array.from({ length: 13 }, (_, index) => ({
    id: `s24-${index + 1}`,
    order: index,
    trigger: index === 2 || index === 11 || index === 12 ? "withPrevious" : "afterPrevious",
    durationMs: 10,
    delayMs: 0,
  }));
  const context = makeContext(slide24Animations);
  const ranges = context.animationNavigationRanges(slide24Animations);
  assert.equal(ranges.length, 10);
  assert.deepEqual(
    JSON.parse(JSON.stringify(ranges.at(-1))),
    { startIndex: 10, endIndex: 13 },
    "slide 24's final three simultaneous entries settle together",
  );

  context.advancePresentation();
  assert.equal(context.state.presenterAnimationTargetCursor, ranges[0].endIndex);
  context.clock.flushAll();
  assert.equal(context.state.presenterAnimationCursor, 13, "without intervention the original automatic stream continues");
  assert.equal(context.state.presenterAnimationTargetCursor, null);

  context.stepPresentation(-1);
  assert.equal(context.state.presenterSlide, 0);
  assert.equal(context.state.presenterAnimationCursor, 10, "backward restores only the final navigation position's start");
  context.stepPresentation(-1);
  assert.equal(context.state.presenterAnimationCursor, 9, "a second backward input restores exactly one earlier position");
}

console.log("presentation keyboard in-flight tests passed");
