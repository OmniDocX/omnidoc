"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const indexSource = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const controlsSource = fs.readFileSync(path.join(__dirname, "lossless_player_controls.js"), "utf8");
const flowRuntimeSource = fs.readFileSync(
  path.join(__dirname, "presentation_flow_runtime.js"),
  "utf8",
);
const transitionRuntimeSource = fs.readFileSync(
  path.join(__dirname, "presentation_transition_runtime.js"),
  "utf8",
);
const losslessRustPath = path.join(
  __dirname,
  "..",
  "crates",
  "unippt-server",
  "src",
  "lossless_html.rs",
);
const losslessRustSource = fs.readFileSync(losslessRustPath, "utf8");

function sourceBetween(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  const end = source.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, `${startMarker} must remain extractable`);
  return source.slice(start, end);
}

function evaluateFlowRuntime() {
  const context = {};
  vm.runInNewContext(flowRuntimeSource, context);
  return context.UniPptPresentationFlow;
}

function compactBatch(batch) {
  return {
    nextIndex: batch.nextIndex,
    end: batch.end,
    entries: batch.entries.map(({ start, end }) => ({ start, end })),
  };
}

test("lossless HTML uses the editor's PowerPoint timing-group scheduler", () => {
  assert.match(
    losslessRustSource,
    /include_str!\("\.\.\/\.\.\/\.\.\/web\/presentation_flow_runtime\.js"\)/,
    "the standalone file must embed the same flow runtime used by the editor",
  );
  assert.match(indexSource, /\{\{asset:\/presentation_flow_runtime\.js\}\}/);
  assert.match(
    appSource,
    /UniPptPresentationFlow\?\.scheduleAnimationBatch[\s\S]{0,200}UniPptPresentationFlow\.scheduleAnimationBatch/,
    "the editor scheduler must delegate to the shared runtime",
  );
  assert.match(
    losslessRustSource,
    /function scheduleAnimationBatch\([^)]*\)\{return globalThis\.UniPptPresentationFlow\.scheduleAnimationBatch/,
    "the exported player scheduler must delegate to the shared runtime",
  );
  const flow = evaluateFlowRuntime();
  const animations = [
    { trigger: "onClick", delayMs: 0, durationMs: 1000 },
    { trigger: "withPrevious", delayMs: 200, durationMs: 100 },
    { trigger: "afterPrevious", delayMs: 0, durationMs: 250 },
    { trigger: "onClick", delayMs: 0, durationMs: 100 },
  ];

  assert.deepEqual(
    JSON.parse(JSON.stringify(compactBatch(flow.scheduleAnimationBatch(animations, 0)))),
    {
      nextIndex: 3,
      end: 1250,
      entries: [
        { start: 0, end: 1000 },
        { start: 200, end: 300 },
        { start: 1000, end: 1250 },
      ],
    },
    "AfterPrevious must begin after the whole WithPrevious group, not after only the immediately preceding effect",
  );
});

test("lossless HTML exposes the same reversible animation navigation positions", () => {
  const flow = evaluateFlowRuntime();
  const animations = [
    { trigger: "onClick", delayMs: 0, durationMs: 100 },
    { trigger: "withPrevious", delayMs: 0, durationMs: 80 },
    { trigger: "afterPrevious", delayMs: 0, durationMs: 60 },
    { trigger: "onClick", delayMs: 0, durationMs: 40 },
  ];
  assert.deepEqual(
    JSON.parse(JSON.stringify(flow.animationNavigationRanges(animations))),
    [
      { startIndex: 0, endIndex: 2 },
      { startIndex: 2, endIndex: 3 },
      { startIndex: 3, endIndex: 4 },
    ],
    "WithPrevious shares a navigation node, while AfterPrevious remains its own reversible node",
  );
});

test("lossless HTML dispatches native complex transitions instead of fading them", () => {
  assert.match(
    losslessRustSource,
    /include_str!\("\.\.\/\.\.\/\.\.\/web\/presentation_transition_runtime\.js"\)/,
    "the standalone file must embed the same transition dispatcher used by the editor",
  );
  assert.match(indexSource, /\{\{asset:\/presentation_transition_runtime\.js\}\}/);
  assert.match(appSource, /UniPptPresentationTransitions\?\.supports/);
  assert.match(losslessRustSource, /UniPptPresentationTransitions\?\.create/);
  const context = {};
  vm.runInNewContext(transitionRuntimeSource, context);
  for (const kind of ["pagecurldouble", "pagecurl", "peeloff", "origami", "fallover", "ripple", "wind", "curtains"]) {
    assert.equal(
      context.UniPptPresentationTransitions.supports(kind),
      true,
      `${kind} must be owned by the shared transition dispatcher`,
    );
  }
});

function evaluateLosslessFlowState(animations) {
  const runBatchSource = sourceBetween(
    losslessRustSource,
    "function runAnimationBatch",
    "\nfunction finishAnimationBatch",
  );
  const finishBatchSource = sourceBetween(
    losslessRustSource,
    "function finishAnimationBatch",
    "\nfunction cancelAnimationBatch",
  );
  const cancelBatchSource = sourceBetween(
    losslessRustSource,
    "function cancelAnimationBatch",
    "\nfunction startAutomaticBatch",
  );
  const advanceSource = sourceBetween(
    losslessRustSource,
    "function advance()",
    "\nfunction rollback",
  );
  const rollbackSource = sourceBetween(
    losslessRustSource,
    "function rollback()",
    "\nfunction draw",
  );
  const context = {
    UniPptPresentationFlow: evaluateFlowRuntime(),
    animationFixture: animations,
    clearTimeout() {},
  };
  vm.runInNewContext(
    `
      let cursor=0,page=0,epoch=1,timers=[],players=[],animationPlayers=[];
      let animationTargetCursor=null,animationBatchStartCursor=null;
      const applied=[],seeks=[];
      const effects=()=>animationFixture;
      const scheduleAnimationNavigation=(items,start)=>UniPptPresentationFlow.scheduleAnimationNavigation(items,start);
      const previousAnimationNavigationCursor=(items,value)=>UniPptPresentationFlow.previousAnimationNavigationCursor(items,value);
      function later(callback,delay){timers.push({callback,delay});return timers.length}
      function effect(){return 0}
      function status(){}
      function syncPlayerAudio(){}
      function applyPlayerAnimationFinalState(animation){applied.push(animation.id)}
      function seekPlayerCursor(value){seeks.push(value);cursor=value;animationTargetCursor=null;animationBatchStartCursor=null}
      function draw(){throw new Error('fixture must not cross a page boundary')}
      function stopPersistentMedia(){}
      function hideLosslessEndNotice(){}
      function showLosslessEndNotice(){}
      ${runBatchSource}
      ${finishBatchSource}
      ${cancelBatchSource}
      ${advanceSource}
      ${rollbackSource}
      this.api={
        advance,rollback,
        snapshot:()=>({cursor,page,animationTargetCursor,animationBatchStartCursor,timers:timers.length,applied:[...applied],seeks:[...seeks]})
      };
    `,
    context,
  );
  return context.api;
}

test("lossless forward input fast-forwards one in-flight navigation position", () => {
  const player = evaluateLosslessFlowState([
    { id: "a", trigger: "onClick", delayMs: 0, durationMs: 100 },
    { id: "b", trigger: "withPrevious", delayMs: 0, durationMs: 80 },
    { id: "c", trigger: "afterPrevious", delayMs: 0, durationMs: 60 },
    { id: "d", trigger: "onClick", delayMs: 0, durationMs: 40 },
  ]);

  player.advance();
  assert.deepEqual(
    JSON.parse(JSON.stringify(player.snapshot())),
    {
      cursor: 0,
      page: 0,
      animationTargetCursor: 2,
      animationBatchStartCursor: 0,
      timers: 2,
      applied: [],
      seeks: [],
    },
    "starting a position must not claim that it has already settled",
  );
  player.advance();
  assert.deepEqual(
    JSON.parse(JSON.stringify(player.snapshot())),
    {
      cursor: 2,
      page: 0,
      animationTargetCursor: null,
      animationBatchStartCursor: null,
      timers: 0,
      applied: ["a", "b"],
      seeks: [],
    },
    "the next input settles only that in-flight position and remains on the slide",
  );
});

test("lossless previous input walks backward through AfterPrevious positions", () => {
  const player = evaluateLosslessFlowState([
    { id: "a", trigger: "onClick", delayMs: 0, durationMs: 100 },
    { id: "b", trigger: "withPrevious", delayMs: 0, durationMs: 80 },
    { id: "c", trigger: "afterPrevious", delayMs: 0, durationMs: 60 },
    { id: "d", trigger: "onClick", delayMs: 0, durationMs: 40 },
  ]);

  player.advance();
  player.advance();
  player.advance();
  player.advance();
  assert.equal(player.snapshot().cursor, 3);
  player.rollback();
  assert.equal(player.snapshot().cursor, 2, "AfterPrevious must be a reversible navigation node");
  player.rollback();
  assert.equal(player.snapshot().cursor, 0, "WithPrevious companions roll back with their shared node");
});

test("lossless Home and End use the same one-node stepping as the editor", () => {
  const homeBranch = sourceBetween(
    controlsSource,
    'else if (!event.ctrlKey && event.key === "Home")',
    'else if (!event.ctrlKey && event.key === "End")',
  );
  const endBranch = sourceBetween(
    controlsSource,
    'else if (!event.ctrlKey && event.key === "End")',
    'else if (["ArrowRight"',
  );
  assert.match(homeBranch, /previousPlayerStep\(\)/, "Home must move back exactly one animation node");
  assert.doesNotMatch(homeBranch, /seekPlayerEdge/);
  assert.match(endBranch, /advance\(\)/, "End must move forward exactly one animation node");
  assert.doesNotMatch(endBranch, /seekPlayerEdge/);
});

test("lossless HTML media commands share editor toggle semantics", () => {
  const mediaRuntimeSource = fs.readFileSync(path.join(__dirname, "media_runtime.js"), "utf8");
  assert.match(
    losslessRustSource,
    /include_str!\("\.\.\/\.\.\/\.\.\/web\/media_runtime\.js"\)/,
    "the standalone player must embed the editor's media lifecycle runtime",
  );
  assert.match(indexSource, /\{\{asset:\/media_runtime\.js\}\}/);
  const editorContext = {};
  vm.runInNewContext(mediaRuntimeSource, editorContext);

  const exportedContext = {};
  vm.runInNewContext(
    [
      mediaRuntimeSource,
      sourceBetween(losslessRustSource, "function control(e,a)", "\nfunction media(e,o)"),
      "this.control = control;",
    ].join("\n"),
    exportedContext,
  );

  function playingMedia() {
    return {
      paused: false,
      ended: false,
      currentTime: 1,
      dataset: { trimStart: "0", playbackEnd: "" },
      pause() { this.paused = true; },
      play() { this.paused = false; return Promise.resolve(); },
    };
  }
  function owner(media) {
    return { querySelector: () => media };
  }

  const editorMedia = playingMedia();
  const exportedMedia = playingMedia();
  editorContext.UniPptMedia.controlNode(owner(editorMedia), "toggle");
  exportedContext.control(owner(exportedMedia), "toggle");
  assert.equal(
    exportedMedia.paused,
    editorMedia.paused,
    "a native toggle media effect must pause an already-playing decoder in either player",
  );
});
