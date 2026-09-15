"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "presentation_transition_runtime.js"), "utf8");
const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const context = {};
vm.runInNewContext(source, context);

const runtime = context.UniPptPresentationTransitions;
assert.equal(runtime.supports("random"), true, "the shared editor/export runtime must own Random");
assert.match(
  appSource,
  /slideIndex:\s*stage\.id\s*===\s*"slideStage"\s*\?\s*state\.activeSlide\s*:\s*state\.presenterSlide/,
  "editor preview and presenter/export playback must seed Random from their actual slide",
);

const resolve = (slideIndex, transition = { kind: "random", durationMs: 900 }) => (
  runtime.resolveRandomTransition(transition, { slideIndex })
);

const first = resolve(17);
const again = resolve(17);
assert.equal(first.kind, again.kind, "the same slide must resolve Random to the same transition kind");
assert.equal(first.direction, again.direction, "the same slide must resolve Random to the same direction");
assert.equal(first.durationMs, 900, "resolution must preserve transition timing");
assert.notEqual(first.kind, "random", "Random must resolve before dispatch instead of degrading to fade");
assert.equal(runtime.supports(first.kind), true, "Random may only resolve to a shared transition kind");

const resolutions = Array.from({ length: 64 }, (_, slideIndex) => resolve(slideIndex));
assert.ok(new Set(resolutions.map((item) => item.kind)).size >= 8, "page seeds should produce a useful range of transitions");
assert.ok(new Set(resolutions.map((item) => item.direction)).size >= 4, "page seeds should vary directional transitions");
assert.ok(resolutions.every((item) => runtime.supports(item.kind)), "every seeded result must be playable in editor, HTML, and video export");

const explicit = resolve(5, { kind: "RANDOM", direction: "right", advanceOnClick: false });
assert.equal(explicit.direction, "right", "an explicit imported direction must win over the seeded direction");
assert.equal(explicit.advanceOnClick, false, "resolution must preserve non-visual transition metadata");

const fade = { kind: "fade", durationMs: 250 };
assert.equal(runtime.resolveRandomTransition(fade, { slideIndex: 1 }), fade, "non-Random transitions must pass through unchanged");

console.log("deterministic random transition tests passed");
