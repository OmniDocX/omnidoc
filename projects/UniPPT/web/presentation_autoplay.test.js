"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const htmlSource = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const styleSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");
const controlsSource = fs.readFileSync(path.join(__dirname, "lossless_player_controls.js"), "utf8");
const flowSource = fs.readFileSync(path.join(__dirname, "presentation_flow_runtime.js"), "utf8");
const rustSource = fs.readFileSync(
  path.join(__dirname, "..", "crates", "unippt-server", "src", "lossless_html.rs"),
  "utf8",
);

test("editor exposes global autoplay entry points and a pressed-state control", () => {
  for (const id of ["presentAutoRibbon", "slideShowAutoPlay", "presentAutoPlay"]) {
    assert.match(htmlSource, new RegExp(`id=["']${id}["']`), `${id} must exist`);
  }
  assert.match(appSource, /startPresentationWhenReady\("current", \{ autoPlay: true \}\)/);
  assert.match(appSource, /function togglePresentationAutoPlay\(/);
  assert.match(appSource, /presentationAutoPlayEnabled\(\)[\s\S]*?aria-pressed/);
  assert.match(styleSource, /\.present-auto-play\.active/);
});

test("editor autoplay waits for transitions and complete animation batches", () => {
  assert.match(
    appSource,
    /schedulePresentationAutoPlay\(presentationInitialAutoPlayDelay\([\s\S]*?animations\.length > 0/,
  );
  assert.match(
    appSource,
    /runPresentationAnimationBatch\(stage, animations\)[\s\S]*?Math\.max\(1, Number\(schedule\?\.end\) \|\| 0\) \+ PRESENTATION_MIN_POST_ANIMATION_MS/,
  );
  assert.match(
    appSource,
    /const autoPlay = state\.presenterMode !== "rehearse"[\s\S]*?const automaticCompletion = autoPlay && options\.userInitiated !== true[\s\S]*?state\.presenterMode === "rehearse" \|\| automaticCompletion \|\| options\.exitAtEnd[\s\S]*?closePresentation\(\)/,
    "autoplay must exit at the final slide even when loop is enabled",
  );
  assert.match(
    appSource,
    /!state\.slideShowSettings\.loop[\s\S]*?showPresentationBoundaryNotice\(\)[\s\S]*?return false/,
    "manual playback must stay on the final slide and show an end notice",
  );
});

test("standalone lossless HTML uses a bottom-only floating toolbar", () => {
  assert.match(rustSource, /id="playerControls" class="nav"/);
  assert.doesNotMatch(rustSource, /id="playerControls" class="nav visible"/);
  assert.match(rustSource, /\.nav\{position:fixed;left:50%;bottom:max\(18px/);
  assert.match(rustSource, /z-index:1000/);
  assert.match(rustSource, /id="playerAutoPlay" class="auto-play"/);
  assert.match(
    controlsSource,
    /const nearBottom = innerHeight - event\.clientY < 112;[\s\S]*?if \(nearBottom\) showLosslessPlayerControls\(true\);[\s\S]*?else hideLosslessPlayerControls\(\)/,
  );
  assert.match(controlsSource, /hideLosslessPlayerControls\(true\);\s*\n}/);
});

test("lossless autoplay consumes every timing group and exits after the last slide", () => {
  assert.match(controlsSource, /function scheduleLosslessAutoPlay\(/);
  assert.match(
    controlsSource,
    /const schedule = runAnimationBatch\(es\);[\s\S]*?scheduleLosslessAutoPlay\(Math\.max\(1, Number\(schedule\?\.end\) \|\| 0\) \+ 80\)/,
  );
  assert.match(
    controlsSource,
    /page >= d\.slides\.length - 1[\s\S]*?closeLosslessPlayer\(\)/,
  );
  assert.match(
    controlsSource,
    /window\.close\(\)[\s\S]*?window\.location\.replace\("about:blank"\)/,
    "a browser that refuses window.close must still leave the playback document",
  );
  assert.match(
    rustSource,
    /if\(playerAutoPlay\)scheduleLosslessAutoPlay\(lead\+\(effects\(\)\.length\?0:1000\)\)/,
  );

  const context = {};
  vm.runInNewContext(flowSource, context);
  const schedule = context.UniPptPresentationFlow.animationSchedule([
    { trigger: "onClick", durationMs: 1000, delayMs: 0 },
    { trigger: "withPrevious", durationMs: 200, delayMs: 100 },
    { trigger: "afterPrevious", durationMs: 300, delayMs: 0 },
    { trigger: "onClick", durationMs: 400, delayMs: 0 },
  ], 0);
  assert.deepEqual(
    JSON.parse(JSON.stringify(schedule.map(({ start, end }) => ({ start, end })))),
    [
      { start: 0, end: 1000 },
      { start: 100, end: 300 },
      { start: 1000, end: 1300 },
      { start: 1300, end: 1700 },
    ],
    "autoplay must preserve WithPrevious concurrency and AfterPrevious sequencing",
  );
});
