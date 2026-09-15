"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const flowSource = fs.readFileSync(
  path.join(__dirname, "presentation_flow_runtime.js"),
  "utf8",
);
const controlsSource = fs.readFileSync(
  path.join(__dirname, "lossless_player_controls.js"),
  "utf8",
);
const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const htmlSource = fs.readFileSync(
  path.join(__dirname, "..", "crates", "unippt-server", "src", "lossless_html.rs"),
  "utf8",
);

function runtime() {
  const context = {};
  vm.runInNewContext(flowSource, context);
  return context.UniPptPresentationFlow;
}

test("native repeat count and auto reverse extend the real PowerPoint time slot", () => {
  const flow = runtime();
  assert.equal(flow.nativeRepeatCount({ repeatCount: "2000" }), 2);
  assert.equal(flow.nativeRepeatCount({ repeatCount: "indefinite" }), Infinity);

  const repeated = { durationMs: 500, repeatCount: "2000" };
  assert.deepEqual(
    JSON.parse(JSON.stringify(flow.animationPlaybackTiming(repeated))),
    { duration: 500, iterations: 2, direction: "normal", easing: "linear", fill: "forwards" },
  );
  assert.equal(flow.animationActiveDuration(repeated), 1000);
  assert.equal(flow.scheduleAnimationBatch([repeated]).end, 1000);

  const reversed = { durationMs: 500, repeatCount: "2000", autoReverse: true };
  assert.equal(flow.animationPlaybackTiming(reversed).iterations, 4);
  assert.equal(flow.animationPlaybackTiming(reversed).direction, "alternate");
  assert.equal(flow.animationActiveDuration(reversed), 2000);

  const capped = { ...reversed, repeatDurationMs: 1250 };
  assert.equal(flow.animationPlaybackTiming(capped).iterations, 2.5);
  assert.equal(flow.animationActiveDuration(capped), 1250);

  const perpetual = { durationMs: 600, repeatCount: "indefinite" };
  assert.equal(flow.animationPlaybackTiming(perpetual).iterations, Infinity);
  assert.equal(
    flow.animationActiveDuration(perpetual),
    600,
    "a looping emphasis remains alive but does not permanently block navigation",
  );
});

test("native acceleration and deceleration select perceptible easing instead of generic ease", () => {
  const flow = runtime();
  assert.equal(flow.animationPlaybackTiming({ durationMs: 500 }).easing, "linear");
  assert.equal(
    flow.animationPlaybackTiming({ durationMs: 500, deceleration: 100000 }).easing,
    "cubic-bezier(0,0,.2,1)",
  );
  assert.equal(
    flow.animationPlaybackTiming({ durationMs: 500, acceleration: 100000 }).easing,
    "cubic-bezier(.42,0,1,1)",
  );
  assert.equal(
    flow.animationPlaybackTiming({ durationMs: 500, acceleration: 40000, deceleration: 40000 }).easing,
    "cubic-bezier(.42,0,.58,1)",
  );
});

test("native signed speed and time filter preserve reverse and shaped PowerPoint motion", () => {
  const flow = runtime();
  const reverse = flow.animationPlaybackTiming({ durationMs: 800, speed: -50000 });
  assert.equal(reverse.duration, 1600);
  assert.equal(reverse.direction, "reverse");
  assert.equal(
    flow.animationPlaybackTiming({ durationMs: 800, speed: -100000, autoReverse: true }).direction,
    "alternate",
  );
  const filtered = flow.animationPlaybackTiming({
    durationMs: 500,
    acceleration: 100000,
    timeFilter: "0,0; .2,.5; .8,.5; 1,0",
  });
  assert.equal(filtered.easing, "linear(0 0%,0.5 20%,0.5 80%,0 100%)");
});

test("editor and lossless HTML consume the same native playback timing", () => {
  assert.match(appSource, /animationPlaybackTiming\?\.\(animation\)/);
  assert.match(appSource, /node\.animate\(frames, playbackTiming\)/);
  assert.match(htmlSource, /animationPlaybackTiming\(a\)/);
  assert.match(htmlSource, /e\.animate\(keyframes,timing\)/);
  assert.match(htmlSource, /animationActiveDuration\(a\)/);
  assert.match(appSource, /UniPptPresetAnimation\?\.frames\(animation/);
  assert.match(htmlSource, /function losslessAnimationFrames[\s\S]*?UniPptPresetAnimation\?\.frames\(a/);
  assert.match(htmlSource, /const keyframes=losslessAnimationFrames\(a,e\)/);
});

test("pointer-clicking player navigation does not pin the floating toolbar forever", () => {
  assert.match(
    controlsSource,
    /function settleLosslessPlayerControls[\s\S]*?event\?\.currentTarget\?\.blur\?\.\(\)/,
  );
  assert.match(controlsSource, /prev\.onclick[\s\S]*?settleLosslessPlayerControls\(event\)/);
  assert.match(controlsSource, /next\.onclick[\s\S]*?settleLosslessPlayerControls\(event\)/);
  assert.match(
    controlsSource,
    /volume\.addEventListener\("pointerup"[\s\S]*?volume\.blur\(\)[\s\S]*?scheduleLosslessPlayerControlsDismiss\(\)/,
  );
});

test("pointer navigation dismisses controls even while the button remains hovered", () => {
  const begin = controlsSource.indexOf("function showLosslessPlayerControls");
  const end = controlsSource.indexOf("function installLosslessPlayerControls");
  assert.ok(begin >= 0 && end > begin);
  const callbacks = [];
  const nav = {
    visible: false,
    classList: {
      add(name) { if (name === "visible") nav.visible = true; },
      remove(name) { if (name === "visible") nav.visible = false; },
    },
    matches() { return true; },
  };
  const context = {
    document: { getElementById: () => nav },
    setTimeout(callback, delay) { callbacks.push({ callback, delay }); return callbacks.length; },
    clearTimeout() {},
  };
  vm.runInNewContext(
    `let playerControlsTimer=0;${controlsSource.slice(begin, end)};globalThis.api={settleLosslessPlayerControls};`,
    context,
  );
  let blurred = 0;
  context.api.settleLosslessPlayerControls({ detail: 1, currentTarget: { blur() { blurred += 1; } } });
  assert.equal(blurred, 1);
  assert.equal(nav.visible, true);
  assert.equal(callbacks.at(-1).delay, 900);
  callbacks.at(-1).callback();
  assert.equal(nav.visible, false, "hover must not keep controls visible after navigation");
});
