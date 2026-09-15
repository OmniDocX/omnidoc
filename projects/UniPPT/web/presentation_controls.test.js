"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const style = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

for (const id of ["presentRibbon", "presentCurrentRibbon", "setupSlideShow", "rehearseTimings"]) {
  assert.match(html, new RegExp(`<button[^>]+id=["']${id}["']`), `${id} must be a real keyboard-accessible button`);
}
for (const id of ["slideShowSettingsDialog", "rehearsalResultDialog", "presentMute", "presentVolume"]) {
  assert.match(html, new RegExp(`id=["']${id}["']`), `${id} must exist`);
}

function presenterButtonTag(id) {
  const match = html.match(new RegExp(`<button\\b(?=[^>]*\\bid=["']${id}["'])[^>]*>`, "i"));
  assert.ok(match, `${id} must be a real presenter toolbar button`);
  return match[0];
}

assert.match(
  presenterButtonTag("prevSlidePresent"),
  /aria-keyshortcuts=["']Control\+Home["']/i,
  "previous-slide must expose Ctrl+Home to assistive technology",
);
assert.match(
  presenterButtonTag("nextSlidePresent"),
  /aria-keyshortcuts=["']Control\+End["']/i,
  "next-slide must expose Ctrl+End to assistive technology",
);
assert.match(
  appSource,
  /prevSlidePresent\.onclick\s*=\s*\(event\)\s*=>\s*\{[\s\S]*?jumpPresentationPage\(-1\)[\s\S]*?\};/,
  "previous-slide button must skip directly to the previous page",
);
assert.match(
  appSource,
  /nextSlidePresent\.onclick\s*=\s*\(event\)\s*=>\s*\{[\s\S]*?jumpPresentationPage\(1\)[\s\S]*?\};/,
  "next-slide button must skip directly to the next page",
);

assert.match(appSource, /presentRibbon"\)\.onclick\s*=\s*\(\)\s*=>\s*\{\s*void startPresentationWhenReady\("beginning"\)/, "from-beginning must use the beginning mode");
assert.match(appSource, /presentCurrentRibbon"\)\.onclick\s*=\s*\(\)\s*=>\s*\{\s*void startPresentationWhenReady\("current"\)/, "from-current must use the active slide");
assert.match(appSource, /startPresentationWhenReady\(event\.shiftKey\s*\?\s*"current"\s*:\s*"beginning"\)/, "F5 and Shift+F5 must keep PowerPoint semantics");
assert.match(appSource, /function renderPresentation[\s\S]*runInitialAutomatic !== false[\s\S]*animations\[0\]\.trigger[\s\S]*runPresentationAnimationBatch/, "entering a slide must only run its leading automatic group");
assert.match(appSource, /function advancePresentation[\s\S]*presenterAnimationCursor >= animations\.length[\s\S]*advanceToNextPresentationSlide\(options\)[\s\S]*runPresentationAnimationBatch/, "forward input must consume animation navigation positions before changing slides");
assert.match(appSource, /function showPresentationBoundaryNotice[\s\S]*已经到底了|presentationBoundaryNotice/, "manual input at the final point must keep the show open with a visible boundary notice");
assert.match(appSource, /function stepPresentation[\s\S]*previousAnimationNavigationCursor[\s\S]*restoreCursor: previousCursor[\s\S]*runInitialAutomatic: false/, "backward input must roll back one animation navigation position before changing slides");
assert.doesNotMatch(appSource, /playPresentationTimeline|schedulePresenterCallback/, "slide show must not auto-chain every click group or auto-advance the page");
assert.match(appSource, /presenterPlaybackToken/, "stale playback timers must be generation-guarded");
assert.match(appSource, /setOutput\([\s\S]*state\.presenterVolume,[\s\S]*state\.presenterMuted,[\s\S]*true/, "presenter output must include persistent cross-slide media");
assert.match(appSource, /scope:\s*"presentation"[\s\S]*slideKey:/, "presenter media must keep a stable reuse identity across slide rebuilds");
assert.match(appSource, /function presentationAnimations[\s\S]*inheritedAnimations[\s\S]*slideAnimations[\s\S]*return \[\.\.\.inheritedAnimations, \.\.\.slideAnimations\]/, "master/layout animations must precede the current slide's animation steps");
assert.match(appSource, /pagecurl[\s\S]*animatePageCurlTransition/, "native page-curl transitions must not fade-fallback");
assert.match(appSource, /normalizedKind\s*===\s*["']pagecurldouble["'][\s\S]*animateTopDownBookTransition/, "pageCurlDouble must use the dedicated top-down open-book renderer");
assert.match(appSource, /ripple[\s\S]*animateRippleTransition/, "native ripple transitions must not fade-fallback");
assert.match(appSource, /wind[\s\S]*animateWindTransition/, "native wind transitions must not fade-fallback");
assert.match(style, /#presentVolume/, "presenter volume slider must have dedicated styling");

const schedulerStart = appSource.indexOf("function scheduleAnimationBatch");
const schedulerEnd = appSource.indexOf("\nfunction animationEffectLabel", schedulerStart);
assert.ok(schedulerStart >= 0 && schedulerEnd > schedulerStart, "animation click scheduler must be extractable");
const schedulerContext = {};
vm.runInNewContext(
  `${appSource.slice(schedulerStart, schedulerEnd)}\nthis.batch = scheduleAnimationBatch; this.previous = previousAnimationBatchCursor; this.navigation = animationNavigationRanges; this.previousNavigation = previousAnimationNavigationCursor;`,
  schedulerContext,
);
const clickFlow = [
  { trigger: "afterPrevious", durationMs: 120, delayMs: 0 },
  { trigger: "withPrevious", durationMs: 80, delayMs: 10 },
  { trigger: "onClick", durationMs: 200, delayMs: 0 },
  { trigger: "afterPrevious", durationMs: 100, delayMs: 20 },
  { trigger: "onClick", durationMs: 90, delayMs: 0 },
];
assert.equal(schedulerContext.batch(clickFlow, 0).nextIndex, 2, "slide entry consumes only the leading automatic group");
assert.equal(schedulerContext.batch(clickFlow, 2).nextIndex, 4, "one right-arrow consumes exactly one PowerPoint click group");
assert.equal(schedulerContext.batch(clickFlow, 4).nextIndex, 5, "the final click group remains on the page until the next input");
assert.equal(schedulerContext.previous(clickFlow, 5), 4, "left-arrow returns from the final node to the previous click position");
assert.equal(schedulerContext.previous(clickFlow, 4), 2, "left-arrow rolls back one click group at a time");
assert.equal(schedulerContext.previous(clickFlow, 2), 0, "left-arrow can restore the initial page state");
assert.equal(schedulerContext.previous(clickFlow, 0), null, "only the page-start boundary may move to the previous slide");
assert.deepEqual(
  JSON.parse(JSON.stringify(schedulerContext.navigation(clickFlow))),
  [
    { startIndex: 0, endIndex: 2 },
    { startIndex: 2, endIndex: 3 },
    { startIndex: 3, endIndex: 4 },
    { startIndex: 4, endIndex: 5 },
  ],
  "AfterPrevious entries become navigation positions while WithPrevious stays merged",
);
assert.equal(schedulerContext.previousNavigation(clickFlow, 5), 4);
assert.equal(schedulerContext.previousNavigation(clickFlow, 4), 3);
assert.equal(schedulerContext.previousNavigation(clickFlow, 3), 2);
assert.equal(schedulerContext.previousNavigation(clickFlow, 2), 0);

const boundsStart = appSource.indexOf("function slideShowBounds");
const boundsEnd = appSource.indexOf("\nfunction openSlideShowSettings", boundsStart);
assert.ok(boundsStart >= 0 && boundsEnd > boundsStart, "slide-show bounds helper must be extractable");
const context = {
  state: {
    deck: { slides: Array.from({ length: 27 }, () => ({})) },
    slideShowSettings: { range: "all", from: 1, to: null },
  },
};
vm.runInNewContext(
  `function clamp(value,min,max){return Math.max(min,Math.min(max,value));}\n${appSource.slice(boundsStart, boundsEnd)}\nthis.bounds = slideShowBounds;`,
  context,
);
assert.deepEqual(JSON.parse(JSON.stringify(context.bounds())), { start: 0, end: 26 });
context.state.slideShowSettings = { range: "range", from: 4, to: 9 };
assert.deepEqual(JSON.parse(JSON.stringify(context.bounds())), { start: 3, end: 8 });
context.state.slideShowSettings = { range: "range", from: 50, to: 2 };
assert.deepEqual(JSON.parse(JSON.stringify(context.bounds())), { start: 26, end: 26 }, "invalid ranges must clamp safely");

console.log("presentation controls tests passed");
