"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const schedulerStart = appSource.indexOf("function scheduleAnimationBatch");
const schedulerEnd = appSource.indexOf("\nfunction animationEffectLabel", schedulerStart);
assert.ok(schedulerStart >= 0 && schedulerEnd > schedulerStart, "animation scheduler should be extractable");
const schedulerContext = {};
vm.runInNewContext(
  `${appSource.slice(schedulerStart, schedulerEnd)}\nthis.scheduler = { scheduleAnimationBatch, animationSchedule, animationBatchRanges, previousAnimationBatchCursor };`,
  schedulerContext,
);
const plain = (value) => JSON.parse(JSON.stringify(value));

const animations = [
  { id: "a", trigger: "onClick", delayMs: 100, durationMs: 1000 },
  { id: "b", trigger: "withPrevious", delayMs: 50, durationMs: 200 },
  { id: "c", trigger: "afterPrevious", delayMs: 25, durationMs: 300 },
  { id: "d", trigger: "withPrevious", delayMs: 10, durationMs: 100 },
  { id: "e", trigger: "onClick", delayMs: 20, durationMs: 400 },
];
const firstBatch = schedulerContext.scheduler.scheduleAnimationBatch(animations, 0);
assert.equal(firstBatch.nextIndex, 4);
assert.deepEqual(
  plain(firstBatch.entries.map(({ start, end }) => [start, end])),
  [[100, 1100], [150, 350], [1125, 1425], [1135, 1235]],
  "withPrevious must share its timing-group start and afterPrevious must follow the whole group",
);
assert.equal(firstBatch.end, 1425, "batch extent must track every timing group");

const fullSchedule = schedulerContext.scheduler.animationSchedule(animations);
assert.deepEqual(
  plain(fullSchedule.map(({ start, end }) => [start, end])),
  [[100, 1100], [150, 350], [1125, 1425], [1135, 1235], [1625, 2025]],
  "preview should compose the same batches with only a synthetic click gap",
);

assert.deepEqual(
  plain(schedulerContext.scheduler.animationBatchRanges(animations)),
  [{ startIndex: 0, endIndex: 4 }, { startIndex: 4, endIndex: 5 }],
  "onClick effects must split the PowerPoint step groups",
);
assert.equal(schedulerContext.scheduler.previousAnimationBatchCursor(animations, 5), 4);
assert.equal(schedulerContext.scheduler.previousAnimationBatchCursor(animations, 4), 0);
assert.equal(schedulerContext.scheduler.previousAnimationBatchCursor(animations, 0), null);

const leading = schedulerContext.scheduler.scheduleAnimationBatch([
  { trigger: "withPrevious", delayMs: 0, durationMs: 1 },
  { trigger: "afterPrevious", delayMs: 9, durationMs: 10 },
  { trigger: "onClick", delayMs: 0, durationMs: 10 },
]);
assert.equal(leading.nextIndex, 2, "leading automatic effects should stop before the first click batch");
assert.deepEqual(plain(leading.entries.map(({ start, end }) => [start, end])), [[0, 1], [10, 20]]);

function fakeHost() {
  return {
    children: [],
    hidden: false,
    append(node) { this.children.push(node); },
    replaceChildren() { this.children = []; },
    querySelectorAll() { return this.children.slice(); },
    setAttribute() {},
  };
}

const elements = new Map();
const document = {
  body: {
    append(node) { elements.set(node.id, node); },
  },
  createElement() { return fakeHost(); },
  getElementById(id) { return elements.get(id) || null; },
};
const mediaContext = { document };
vm.runInNewContext(
  fs.readFileSync(path.join(__dirname, "media_runtime.js"), "utf8"),
  mediaContext,
);

function fakeMedia({ across, audio = true, currentTime = 42 }) {
  return {
    dataset: { playAcrossSlides: across ? "1" : "0", trimStart: "3" },
    paused: false,
    currentTime,
    pauseCount: 0,
    matches(selector) { return audio && selector === "audio.native-media"; },
    pause() { this.paused = true; this.pauseCount++; },
  };
}

const across = fakeMedia({ across: true });
const ordinary = fakeMedia({ across: false });
const stage = { querySelectorAll() { return [across, ordinary]; } };
mediaContext.UniPptMedia.stopAll(stage, true, true);
assert.equal(across.pauseCount, 0, "playing cross-slide audio must not be stopped during slide render");
assert.equal(across.currentTime, 42);
assert.equal(ordinary.pauseCount, 1);
assert.equal(ordinary.currentTime, 3);
assert.deepEqual(elements.get("unippt-persistent-media").children, [across]);

mediaContext.UniPptMedia.stopPersistent(true);
assert.equal(across.pauseCount, 1, "closing the presentation should stop carried audio");
assert.equal(across.currentTime, 3);
assert.equal(elements.get("unippt-persistent-media").children.length, 0);

console.log("animation runtime tests passed");
