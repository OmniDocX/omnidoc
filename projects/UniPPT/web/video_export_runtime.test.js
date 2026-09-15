"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const app = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const controls = fs.readFileSync(path.join(__dirname, "lossless_player_controls.js"), "utf8");
const renderer = fs.readFileSync(
  path.join(__dirname, "..", "tools", "html-video-renderer", "render.mjs"),
  "utf8",
);

test("video export exposes profile, resolution, frame rate, and real progress", () => {
  for (const id of [
    "videoExportDialog",
    "videoExportProfile",
    "videoExportResolution",
    "videoExportFps",
    "videoExportMuted",
    "confirmVideoExport",
    "loadingProgressBar",
  ]) {
    assert.match(html, new RegExp(`id="${id}"`));
  }
  for (const header of [
    "X-UniPPT-Video-Width",
    "X-UniPPT-Video-Height",
    "X-UniPPT-Video-Fps",
    "X-UniPPT-Video-Profile",
    "X-UniPPT-Video-Muted",
    "X-UniPPT-Export-Job",
  ]) {
    assert.match(app, new RegExp(header));
  }
  assert.match(app, /\/api\/export-progress\//);
});

test("all video profiles use v6 deterministic JPEG frame transport", () => {
  assert.match(renderer, /frameFormat:\s*"jpeg"/);
  assert.match(renderer, /inputCodec:\s*"mjpeg"/);
  assert.match(renderer, /jpegQuality:\s*profile === "fast" \? 76 : profile === "balanced" \? 86 : 94/);
  assert.match(renderer, /optimizeForSpeed:\s*true/);
  assert.match(renderer, /"-vcodec", profile\.inputCodec/);
  assert.match(renderer, /encoderThreads:\s*1/);
  assert.match(renderer, /"-threads", String\(profile\.encoderThreads\)/);
  assert.match(renderer, /renderWorkerCount\(slideCount, process\.env\.UNIPPT_RENDER_WORKERS\)/);
  assert.doesNotMatch(renderer, /workers:\s*profile === "fast" \? 8 : 6/);
  assert.match(renderer, /document\.documentElement\.offsetHeight/);
  assert.doesNotMatch(renderer, /if \(changed\) await new Promise\(\(resolve\) => requestAnimationFrame/);
  assert.match(renderer, /segmentCacheKey/);
  assert.match(renderer, /unippt-video-segments-v6/);
  assert.match(renderer, /RENDER_ASSET_CACHE_SCHEMA = "unippt-render-assets-v6"/);
  assert.match(renderer, /Cache-Control": "public, max-age=31536000, immutable"/);
  assert.match(renderer, /supportsExternalAssets/);
  assert.match(renderer, /kept legacy inline assets/);
  assert.match(renderer, /async function persistentRendererContext\(/);
  assert.match(renderer, /async function encodeStillFrame\(/);
  assert.match(renderer, /staticSegment: !encoder/);
  assert.match(renderer, /UNIPPT_VIDEO_ENCODER/);
});

test("video export can preserve presentation audio or explicitly mute it", () => {
  assert.match(html, /id="videoExportMuted"/);
  assert.match(app, /"X-UniPPT-Video-Muted": options\.muted \? "1" : "0"/);
  assert.match(renderer, /function buildAudioPlan\(/);
  assert.match(renderer, /function muxPresentationAudio\(/);
  assert.match(renderer, /"-c:a", "aac"/);
  assert.match(renderer, /const silentOutput = muted \? output/);
});

test("PDF command uses browser-native print with a dedicated slide page tree", () => {
  assert.match(html, /id="exportPdf"[\s\S]*?打印 \/ PDF/);
  assert.match(app, /window\.addEventListener\("beforeprint", setupNativePrintPages\)/);
  assert.match(app, /window\.addEventListener\("afterprint", teardownNativePrintPages\)/);
  assert.match(app, /const NATIVE_PRINT_HOST_ID = "unipptNativePrintPages"/);
  assert.match(app, /@page \{ size: \$\{width\}px \$\{height\}px; margin: 0; \}/);
  assert.match(app, /renderReadOnlySlide\(slide, page,/);
  assert.match(app, /await settleNativePrintPages\(host\)/);
  assert.match(app, /window\.print\(\)/);
  assert.doesNotMatch(
    app.slice(app.indexOf("async function exportPdf()"), app.indexOf("async function exportSlideImages()")),
    /\/api\/export-pdf/,
  );
});

test("slide transitions are prepared before installing the offline capture clock", () => {
  assert.match(controls, /async prepareSlidePlayback\(index\)/);
  assert.match(controls, /async startPreparedSlidePlayback\(index, options = \{\}\)/);
  const prepare = renderer.indexOf("UniPptCapture.prepareSlidePlayback");
  const clock = renderer.indexOf("await installOfflineClock(page)", prepare);
  const start = renderer.indexOf("UniPptCapture.startPreparedSlidePlayback", clock);
  assert.ok(prepare >= 0 && clock > prepare && start > clock);
  assert.doesNotMatch(
    renderer.slice(clock, start),
    /UniPptCapture\.startSlidePlayback/,
    "the deterministic clock must not wrap asset/font preparation",
  );
});
