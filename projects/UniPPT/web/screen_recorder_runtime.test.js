"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const recorderSource = fs.readFileSync(
  path.join(__dirname, "screen_recorder_runtime.js"),
  "utf8",
);
const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const style = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

/**
 * Minimal stand-ins for the capture APIs. `supportedTypes` controls which
 * container the fake browser claims to encode so codec negotiation is testable.
 */
function browser({ supportedTypes = ["video/webm;codecs=vp9,opus", "video/mp4;codecs=avc1.42E01E,mp4a.40.2"], trackSettings = {} } = {}) {
  const calls = { getDisplayMedia: [], constructed: [], starts: [], stoppedTracks: 0 };
  let endTrack = null;

  class FakeRecorder {
    static isTypeSupported(type) { return supportedTypes.includes(type); }
    constructor(stream, options) {
      calls.constructed.push({ stream, options });
      this.state = "inactive";
      this.stream = stream;
    }
    start(...args) {
      calls.starts.push(args);
      this.state = "recording";
      setTimeout(() => this.ondataavailable?.({ data: { size: 1024 } }), 0);
    }
    stop() {
      this.state = "inactive";
      this.ondataavailable?.({ data: { size: 2048 } });
      this.onstop?.();
    }
  }

  const track = {
    kind: "video",
    listeners: {},
    getSettings: () => ({ width: 1920, height: 1080, frameRate: 30, ...trackSettings }),
    addEventListener(name, handler) {
      this.listeners[name] = handler;
      if (name === "ended") endTrack = handler;
    },
    stop() { calls.stoppedTracks += 1; },
  };
  const stream = { getVideoTracks: () => [track], getTracks: () => [track] };

  const context = {
    MediaRecorder: FakeRecorder,
    Blob: globalThis.Blob,
    setTimeout,
    clearTimeout,
    navigator: {
      mediaDevices: {
        getDisplayMedia(constraints) {
          calls.getDisplayMedia.push(constraints);
          return Promise.resolve(stream);
        },
      },
    },
  };
  context.globalThis = context;
  vm.runInNewContext(recorderSource, context);
  return { runtime: context.UniPptScreenRecorder, calls, endShare: () => endTrack?.() };
}

test("recording is offered only when the browser can capture and encode", () => {
  const bare = { globalThis: null };
  bare.globalThis = bare;
  vm.runInNewContext(recorderSource, bare);
  assert.equal(bare.UniPptScreenRecorder.supported(), false);
  assert.equal(bare.UniPptScreenRecorder.supportedFormats().length, 0);
  assert.equal(bare.UniPptScreenRecorder.preferredFormat(), null);

  const { runtime } = browser();
  assert.equal(runtime.supported(), true);
});

test("codec negotiation prefers stable WebM and keeps MP4 as a fallback", () => {
  const webmPreferred = browser({ supportedTypes: ["video/mp4;codecs=avc1.42E01E,mp4a.40.2", "video/webm;codecs=vp9,opus"] });
  assert.equal(webmPreferred.runtime.preferredFormat().extension, "webm");
  assert.match(webmPreferred.runtime.preferredFormat().label, /VP9/);

  const webm = browser({ supportedTypes: ["video/webm;codecs=vp9,opus", "video/webm"] });
  assert.equal(webm.runtime.preferredFormat().extension, "webm");
  assert.equal(webm.runtime.supportedFormats().length, 2);

  const none = browser({ supportedTypes: [] });
  assert.equal(none.runtime.preferredFormat(), null);

  const mp4Only = browser({ supportedTypes: ["video/mp4;codecs=avc1.42E01E"] });
  assert.equal(mp4Only.runtime.preferredFormat().extension, "mp4");
  assert.match(mp4Only.runtime.preferredFormat().label, /兼容模式/);
});

test("bitrate scales with pixels but stays inside a usable band", () => {
  const { runtime } = browser();
  const hd = runtime.bitrateFor(1920, 1080, 30);
  const sd = runtime.bitrateFor(1280, 720, 30);
  assert.ok(hd > sd, "a larger frame gets more bits");
  assert.ok(sd >= 2.5e6, "small decks are not starved");
  assert.equal(runtime.bitrateFor(7680, 4320, 60), 40e6, "8K is clamped to a sane ceiling");
});

test("a take captures for the lifetime of playback and returns a downloadable blob", async () => {
  const { runtime, calls } = browser();
  const order = [];
  const result = await runtime.record({
    width: 1920,
    height: 1080,
    fps: 30,
    prepare: () => order.push("prepare"),
    onStart: () => order.push("start"),
    async play() {
      order.push("play");
      await new Promise((resolve) => setTimeout(resolve, 5));
    },
  });
  assert.deepEqual(order, ["prepare", "start", "play"], "the painted show is ready before capture begins");
  assert.equal(result.format.extension, "webm");
  assert.equal(result.width, 1920);
  assert.equal(result.height, 1080);
  assert.equal(result.frameRate, 30);
  assert.equal(result.endedByUser, false);
  assert.ok(result.blob.size > 0, "chunks are assembled into a real file");
  assert.ok(result.durationMs >= 0);
  assert.equal(calls.stoppedTracks, 1, "the shared surface is always released");

  // The picker is steered at the current tab and asked for the chosen geometry.
  const [constraints] = calls.getDisplayMedia;
  assert.equal(constraints.preferCurrentTab, true);
  assert.equal(constraints.video.width.ideal, 1920);
  assert.equal(constraints.video.frameRate.ideal, 30);
  assert.equal(constraints.video.frameRate.max, 30, "capture cadence must match the requested export FPS");
  assert.ok(constraints.audio, "audio is captured unless explicitly muted");
  assert.deepEqual(calls.starts[0], [1000], "WebM keeps recoverable incremental clusters");
});

test("preparing the first slide does not leak editor chrome into the recording", async () => {
  const { runtime, calls } = browser();
  let startsSeenDuringPrepare = null;
  await runtime.record({
    prepare: async (session) => {
      startsSeenDuringPrepare = calls.starts.length;
      assert.equal(session.width, 1920);
      assert.equal(session.height, 1080);
      assert.equal(session.frameRate, 30);
    },
    play: async () => {},
  });
  assert.equal(startsSeenDuringPrepare, 0, "MediaRecorder stays idle while the presenter paints");
  assert.equal(calls.starts.length, 1);
});

test("ending the share while the first slide prepares never starts an empty take", async () => {
  const { runtime, calls, endShare } = browser();
  await assert.rejects(
    runtime.record({ prepare: async () => endShare(), play: async () => {} }),
    (error) => error?.name === "AbortError",
  );
  assert.equal(calls.starts.length, 0);
  assert.equal(calls.stoppedTracks, 1);
});

test("MP4 fallback avoids fragmented timeslices that corrupt H.264 screen captures", async () => {
  const { runtime, calls } = browser({ supportedTypes: ["video/mp4;codecs=avc1.42E01E"] });
  const result = await runtime.record({ play: async () => {} });
  assert.equal(result.format.extension, "mp4");
  assert.deepEqual(calls.starts[0], [], "MP4 is finalized as one continuous recording");
});

test("muting the export drops the audio track request", async () => {
  const { runtime, calls } = browser();
  await runtime.record({ audio: false, play: async () => {} });
  assert.equal(calls.getDisplayMedia[0].audio, false);
});

test("ending the share early still yields the partial recording", async () => {
  const { runtime, endShare } = browser();
  let notified = false;
  const result = await runtime.record({
    onShareEnded: () => { notified = true; },
    async play(session) {
      endShare();
      assert.equal(session.shareEnded, true, "playback can observe the interruption");
    },
  });
  assert.equal(notified, true);
  assert.equal(result.endedByUser, true);
  assert.ok(result.blob.size > 0, "an interrupted take is saved rather than discarded");
});

test("the stream is released even when playback throws", async () => {
  const { runtime, calls } = browser();
  await assert.rejects(
    runtime.record({ play: async () => { throw new Error("boom"); } }),
    /boom/,
  );
  assert.equal(calls.stoppedTracks, 1);
});

test("recording is reachable from the export UI and cannot leak chrome into the frame", () => {
  assert.match(html, /id="exportVideoRecording"/);
  assert.match(html, /id="videoExportEngine"[\s\S]*?value="browser"/);
  assert.match(html, /export:\s*\{[\s\S]*?\{\{asset:\/screen_recorder_runtime\.js\}\}/);
  assert.match(appSource, /ensureFeature\("export"\)/, "the recorder must load outside the first-paint core");
  assert.match(appSource, /\$\("#exportVideoRecording"\)\.onclick/);
  assert.match(appSource, /UniPptScreenRecorder/);
  // The show must open idle and settle before MediaRecorder starts, so the
  // first captured frame is a finished slide rather than editor chrome.
  assert.match(appSource, /async prepare\(\)[\s\S]*?startPresentation\("beginning",\s*\{[\s\S]*?runInitialAutomatic:\s*false/);
  assert.match(recorderSource, /await options\.prepare\?\.\(session\)[\s\S]*?recorder\.start/);
  assert.match(appSource, /renderPresentation\(false,\s*\{\s*runInitialAutomatic:\s*options\.runInitialAutomatic\s*\}\)/);
  // Playback end is signalled one way, so the recorder never polls the presenter.
  assert.match(appSource, /unippt:presentation-closed/);
  assert.match(style, /\.presenter\.recording \.present-controls\s*\{\s*display:none!important/);
  assert.match(style, /\.presenter\.recording[^{]*\{\s*cursor:none!important/);
  // The grid sets `display:grid` on every label, which outranks the user-agent
  // `[hidden]` rule, so the server-only quality field needs an explicit opt-out.
  assert.match(style, /\.video-export-grid label\[hidden\]\s*\{\s*display:none/);
});

test("recording uses a real-time slide clock instead of the one-second autoplay preview", () => {
  assert.match(appSource, /PRESENTATION_RECORDING_DEFAULT_SLIDE_MS\s*=\s*5_000/);
  assert.match(appSource, /function presentationSlideTargetMs\([\s\S]*?presentationIsRecording\(\)[\s\S]*?PRESENTATION_RECORDING_DEFAULT_SLIDE_MS/);
  assert.match(appSource, /presentationCompletionDelay\(slide\)/,
    "the final animation batch must wait out the recording slide budget");
  assert.doesNotMatch(recorderSource, /frameRate:\s*\{\s*ideal:\s*fps,\s*max:\s*Math\.max\(fps,\s*60\)/,
    "a 30 FPS take must not silently negotiate a 60 FPS track");
});
