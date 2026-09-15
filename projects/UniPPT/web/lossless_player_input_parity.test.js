"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const controlsSource = fs.readFileSync(
  path.join(__dirname, "lossless_player_controls.js"),
  "utf8",
);
const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const losslessRustSource = fs.readFileSync(
  path.join(__dirname, "..", "crates", "unippt-server", "src", "lossless_html.rs"),
  "utf8",
);

function sourceBetween(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  const end = source.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, `${startMarker} must remain extractable`);
  return source.slice(start, end);
}

test("first automatic group waits for one user activation without fast-forwarding", () => {
  const activationSource = sourceBetween(
    controlsSource,
    "let playerActivated",
    "\nfunction playerMediaNodes",
  );
  const automaticSource = sourceBetween(
    losslessRustSource,
    "function startAutomaticBatch",
    "\nfunction stopPersistentMedia",
  );
  const context = {
    navigator: { userActivation: { hasBeenActive: false } },
    restoringPlayerState: false,
    cursor: 0,
    animations: [{ trigger: "afterPrevious", durationMs: 120 }],
    runs: [],
  };
  context.effects = () => context.animations;
  context.runAnimationBatch = (animations, offset) => {
    context.runs.push({ animations, offset });
  };
  vm.runInNewContext(
    `${activationSource}\n${automaticSource}\nthis.api={startAutomaticBatch,activateLosslessPlayer,runLosslessPlayerAction,snapshot:()=>({playerActivated,pendingPlayerAutomaticStartOffset})};`,
    context,
  );

  assert.equal(context.api.startAutomaticBatch(175), false);
  assert.deepEqual(context.runs, [], "load-time automatic media must not call play before activation");
  assert.deepEqual(
    JSON.parse(JSON.stringify(context.api.snapshot())),
    { playerActivated: false, pendingPlayerAutomaticStartOffset: 175 },
  );

  let navigationActions = 0;
  assert.equal(
    context.api.runLosslessPlayerAction(() => { navigationActions += 1; }),
    false,
    "the activation input is consumed by the queued automatic group",
  );
  assert.equal(navigationActions, 0, "the same first input must not fast-forward that group");
  assert.equal(context.runs.length, 1);
  assert.equal(context.runs[0].offset, 175);
  assert.equal(context.api.snapshot().pendingPlayerAutomaticStartOffset, null);

  assert.equal(
    context.api.runLosslessPlayerAction(() => { navigationActions += 1; }),
    true,
  );
  assert.equal(navigationActions, 1, "later input resumes normal PowerPoint navigation");
});

test("a first on-click build is not swallowed when no automatic group is queued", () => {
  const activationSource = sourceBetween(
    controlsSource,
    "let playerActivated",
    "\nfunction playerMediaNodes",
  );
  const automaticSource = sourceBetween(
    losslessRustSource,
    "function startAutomaticBatch",
    "\nfunction stopPersistentMedia",
  );
  const context = {
    navigator: { userActivation: { hasBeenActive: false } },
    restoringPlayerState: false,
    cursor: 0,
    animations: [{ trigger: "onClick", durationMs: 120 }],
    runAnimationBatch() { throw new Error("on-click group must not auto-run"); },
  };
  context.effects = () => context.animations;
  vm.runInNewContext(
    `${activationSource}\n${automaticSource}\nthis.api={startAutomaticBatch,runLosslessPlayerAction};`,
    context,
  );
  assert.equal(context.api.startAutomaticBatch(0), false);
  let actions = 0;
  assert.equal(context.api.runLosslessPlayerAction(() => { actions += 1; }), true);
  assert.equal(actions, 1);
});

test("focused player controls retain native arrow and Enter handling", () => {
  const keyboardSource = sourceBetween(
    controlsSource,
    'document.addEventListener("keydown"',
    "\n\n  new MutationObserver",
  );
  assert.match(keyboardSource, /focusedControl\s*=\s*event\.target\?\.closest\?\.\("#playerControls"\)/);
  assert.match(keyboardSource, /"ArrowLeft",\s*"ArrowRight",\s*"ArrowUp",\s*"ArrowDown"/);
  assert.match(
    keyboardSource,
    /focusedControl\s*&&\s*nativeControlKeys\.includes\(event\.key\)[\s\S]*?return;/,
    "range arrows and button activation must return before slide navigation",
  );
});

test("presentation shortcuts never summon the floating toolbar", () => {
  const keyboardSource = sourceBetween(
    controlsSource,
    'document.addEventListener("keydown"',
    "\n\n  new MutationObserver",
  );
  assert.match(keyboardSource, /event\.ctrlKey && event\.key === "Home"/);
  assert.match(keyboardSource, /event\.ctrlKey && event\.key === "End"/);
  assert.match(keyboardSource, /!event\.ctrlKey && event\.key === "Home"/);
  assert.match(keyboardSource, /!event\.ctrlKey && event\.key === "End"/);
  assert.match(
    keyboardSource,
    /hideLosslessPlayerControls\(true\);\s*}\);\s*$/,
    "handled presentation shortcuts must force-hide the toolbar",
  );
  assert.doesNotMatch(
    keyboardSource.replace(/if \(!event\.ctrlKey[\s\S]*?return;\s*}/, ""),
    /showLosslessPlayerControls\(\);/,
    "global shortcut handling must not reveal controls",
  );
});

test("Ctrl+Home/End settle the whole page before crossing boundaries", () => {
  assert.match(controlsSource, /function skipPlayerToPageEnd\(direction\)/);
  assert.match(controlsSource, /function playerPageComplete\(\)/);
  assert.match(controlsSource, /seekPlayerCursor\(effects\(\)\.length, true\)/);
  assert.match(controlsSource, /runLosslessPlayerAction\(\(\) => skipPlayerToPageEnd\(-1\)\)/);
  assert.match(controlsSource, /runLosslessPlayerAction\(\(\) => skipPlayerToPageEnd\(1\)\)/);
});

test("the activation click is captured before media objects can swallow it", () => {
  assert.match(
    controlsSource,
    /document\.addEventListener\("click",[\s\S]*?activateLosslessPlayer\(\)[\s\S]*?stopImmediatePropagation\(\)[\s\S]*?}, true\);/,
    "a first click on a media object must start the queued automatic group without also toggling or advancing it",
  );
});

test("Enter advances one PowerPoint node in both players", () => {
  assert.match(
    controlsSource,
    /\["ArrowRight",\s*"ArrowDown",\s*"PageDown",\s*" ",\s*"Enter"\]\.includes\(event\.key\)/,
  );
  assert.match(
    appSource,
    /\["ArrowRight",\s*"ArrowDown",\s*"PageDown",\s*" ",\s*"Enter"\]\.includes\(event\.key\)[\s\S]{0,120}stepPresentation\(1\)/,
  );
});

test("manual input after the final settled node stays open and shows the end notice", () => {
  const advanceSource = sourceBetween(
    losslessRustSource,
    "function advance()",
    "\nfunction rollback",
  );
  const context = {
    cursor: 0,
    page: 0,
    d: { slides: [{}] },
    closed: 0,
    notices: 0,
    effects() { return []; },
    finishAnimationBatch() { return false; },
    runAnimationBatch() { throw new Error("no animation remains"); },
    draw() { throw new Error("there is no next slide"); },
  };
  context.closeLosslessPlayer = () => { context.closed += 1; };
  context.hideLosslessEndNotice = () => {};
  context.showLosslessEndNotice = () => { context.notices += 1; };
  vm.runInNewContext(`${advanceSource}\nthis.advance=advance;`, context);
  context.advance();
  assert.equal(context.closed, 0);
  assert.equal(context.notices, 1);

  const closeSource = sourceBetween(
    controlsSource,
    "function closeLosslessPlayer",
    "\nfunction showLosslessPlayerControls",
  );
  assert.match(closeSource, /UniPptMedia\?\.stopAll\(s, true, false\)/);
  assert.match(closeSource, /stopPersistentMedia\(\)/);
  assert.match(
    appSource,
    /function advanceToNextPresentationSlide[\s\S]*?!state\.slideShowSettings\.loop[\s\S]*?showPresentationBoundaryNotice\(\)/,
    "the editor uses the same non-exiting final-node boundary",
  );
  assert.match(controlsSource, /function showLosslessEndNotice\(\)/);
  assert.match(controlsSource, /已经到底了/);
  assert.match(controlsSource, /最后一张 · 按 Esc 退出放映/);
});

test("standalone playback blocks native browser editing gestures", () => {
  const guards = sourceBetween(
    controlsSource,
    "function installLosslessBrowserGuards",
    "\nfunction activateLosslessPlayer",
  );
  for (const eventName of [
    "contextmenu", "auxclick", "dragstart", "selectstart", "pointerdown",
    "dblclick", "touchmove", "wheel",
  ]) {
    assert.match(guards, new RegExp(`document\\.addEventListener\\(\\"${eventName}\\"`));
  }
  assert.match(guards, /\["gesturestart", "gesturechange", "gestureend"\]/);
  assert.match(guards, /document\.addEventListener\(type,/);
  assert.match(controlsSource, /getSelection\?\.\(\)\?\.removeAllRanges/);
  assert.match(guards, /event\.ctrlKey \|\| event\.metaKey/);
  assert.match(controlsSource, /installLosslessBrowserGuards\(\);/);
  assert.match(losslessRustSource, /touch-action:none/);
  assert.match(losslessRustSource, /-webkit-user-select:none;user-select:none/);
  assert.match(losslessRustSource, /-webkit-user-drag:none/);
});

test("standalone playback owns zoom from 10 to 500 percent", () => {
  assert.match(controlsSource, /const PLAYER_MIN_ZOOM = 0\.1;/);
  assert.match(controlsSource, /const PLAYER_MAX_ZOOM = 5;/);
  assert.match(controlsSource, /function setPlayerZoom\(value\)/);
  assert.match(controlsSource, /browserZoomShortcut/);
  assert.match(controlsSource, /event\.key === "0"\) setPlayerZoom\(1\)/);
  for (const id of ["playerZoomOut", "playerZoomValue", "playerZoomIn"]) {
    assert.match(losslessRustSource, new RegExp(`id=\\"${id}\\"`));
  }
  assert.match(
    losslessRustSource,
    /Math\.min\(innerWidth\/d\.width,innerHeight\/d\.height\)\*\(typeof playerZoom===/,
    "custom zoom must multiply the fit-to-window scale instead of changing browser zoom",
  );
});
