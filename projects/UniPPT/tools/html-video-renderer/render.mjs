#!/usr/bin/env node
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { AsyncLocalStorage } from "node:async_hooks";
import { createReadStream } from "node:fs";
import { access, mkdir, readFile, rename, stat, unlink, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";
import { chromium } from "playwright-core";
import { segmentCacheKey, publishCachedSegment } from "./segment_cache.mjs";
import { renderWorkerCount } from "./worker_policy.mjs";

const progressOutput = new AsyncLocalStorage();
const renderDocuments = new Map();
const RENDER_ASSET_CACHE_SCHEMA = "unippt-render-assets-v6";
let rendererBrowserPromise = null;
let rendererContextPromise = null;

function parseArgs(argv) {
  const result = {};
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith("--")) continue;
    result[key.slice(2)] = argv[index + 1];
    index += 1;
  }
  return result;
}

function emitProgress(phase, detail = {}) {
  const line = `UNIPPT_PROGRESS ${JSON.stringify({ phase, ...detail })}\n`;
  const output = progressOutput.getStore();
  if (output?.write) output.write(line);
  else process.stderr.write(line);
}

function emitDiagnostic(message) {
  const text = String(message || "");
  const output = progressOutput.getStore();
  if (output?.write) output.write(`UNIPPT_DIAGNOSTIC ${JSON.stringify({ message: text })}\n`);
  else process.stderr.write(`${text}\n`);
}

async function exists(candidate) {
  if (!candidate) return false;
  try { await access(candidate); return true; } catch { return false; }
}

async function findChromium() {
  const candidates = [
    process.env.UNIPPT_CHROMIUM_PATH,
    process.env.CHROME_PATH,
    process.platform === "win32" ? "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe" : null,
    process.platform === "win32" ? "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe" : null,
    process.platform === "darwin" ? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" : null,
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
  ];
  for (const candidate of candidates) if (await exists(candidate)) return candidate;
  throw new Error("Chromium was not found; set UNIPPT_CHROMIUM_PATH");
}

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"], windowsHide: true });
    let stderr = "";
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", reject);
    child.on("close", (code) => code === 0
      ? resolve()
      : reject(new Error(`${command} exited with ${code}: ${stderr.slice(-4000)}`)));
  });
}

function runOutput(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"], windowsHide: true });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", reject);
    child.on("close", (code) => code === 0
      ? resolve(stdout)
      : reject(new Error(`${command} exited with ${code}: ${stderr.slice(-4000)}`)));
  });
}

function writeStream(stream, chunk) {
  return new Promise((resolve, reject) => {
    const onError = (error) => { cleanup(); reject(error); };
    const cleanup = () => stream.off("error", onError);
    stream.on("error", onError);
    if (stream.write(chunk)) {
      cleanup();
      resolve();
    } else {
      stream.once("drain", () => { cleanup(); resolve(); });
    }
  });
}

function videoProfile(value) {
  const profile = String(value || "quality").toLowerCase();
  if (!["fast", "balanced", "quality"].includes(profile)) {
    throw new Error(`unsupported video profile: ${profile}`);
  }
  return {
    name: profile,
    scale: profile === "fast" ? 0.5 : profile === "balanced" ? 0.75 : 1,
    // PNG screenshots dominate wall time on animation-heavy slides.  A
    // visually lossless high-quality JPEG capture is substantially faster,
    // while the final H.264 encode still runs at full requested resolution.
    frameFormat: "jpeg",
    inputCodec: "mjpeg",
    jpegQuality: profile === "fast" ? 76 : profile === "balanced" ? 86 : 94,
    preset: profile === "fast" ? "ultrafast" : "veryfast",
    crf: profile === "fast" ? 23 : profile === "balanced" ? 20 : 18,
    // Multiple slides are rendered in parallel.  Letting every x264 child
    // auto-spawn one thread per CPU massively oversubscribes the machine.
    encoderThreads: 1,
  };
}

function even(value) {
  const rounded = Math.max(2, Math.round(value));
  return rounded - (rounded % 2);
}

function startFrameEncoder(output, width, height, fps, profile) {
  const child = spawn(ffmpegCommand(), [
    "-y", "-f", "image2pipe", "-framerate", String(fps),
    "-vcodec", profile.inputCodec, "-i", "pipe:0",
    "-vf", `scale=${width}:${height}:flags=lanczos`,
    ...videoEncoderArgs(profile),
    "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an", output,
  ], { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
  let stderr = "";
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  const completed = new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("close", (code) => code === 0
      ? resolve()
      : reject(new Error(`ffmpeg exited with ${code}: ${stderr.slice(-4000)}`)));
  });
  return { child, completed };
}

async function encodeStillFrame(output, width, height, fps, profile, frame, frames) {
  const still = `${output}.still-${process.pid}-${Math.random().toString(36).slice(2)}.jpg`;
  await writeFile(still, frame);
  try {
    await run(ffmpegCommand(), [
      "-y", "-loop", "1", "-framerate", String(fps), "-i", still,
      "-frames:v", String(Math.max(1, frames)),
      "-vf", `scale=${width}:${height}:flags=lanczos`,
      ...videoEncoderArgs(profile),
      "-pix_fmt", "yuv420p", "-movflags", "+faststart", "-an", output,
    ]);
  } finally {
    await unlink(still).catch(() => {});
  }
}

function ffmpegCommand() {
  return process.env.UNIPPT_FFMPEG_PATH || "ffmpeg";
}

function ffprobeCommand() {
  if (process.env.UNIPPT_FFPROBE_PATH) return process.env.UNIPPT_FFPROBE_PATH;
  const ffmpeg = ffmpegCommand();
  const basename = path.basename(ffmpeg).toLowerCase();
  if (basename === "ffmpeg" || basename === "ffmpeg.exe") {
    return path.join(path.dirname(ffmpeg), process.platform === "win32" ? "ffprobe.exe" : "ffprobe");
  }
  return "ffprobe";
}

function videoEncoderArgs(profile) {
  if (profile.encoder === "h264_nvenc") {
    const preset = profile.name === "fast" ? "p1" : profile.name === "balanced" ? "p3" : "p5";
    return ["-c:v", "h264_nvenc", "-preset", preset, "-rc", "vbr", "-cq", String(profile.crf), "-b:v", "0"];
  }
  if (profile.encoder === "h264_qsv") {
    return ["-c:v", "h264_qsv", "-preset", profile.preset, "-global_quality", String(profile.crf), "-look_ahead", "0"];
  }
  if (profile.encoder === "h264_amf") {
    const quality = profile.name === "fast" ? "speed" : profile.name === "balanced" ? "balanced" : "quality";
    return ["-c:v", "h264_amf", "-quality", quality, "-rc", "cqp", "-qp_i", String(profile.crf), "-qp_p", String(profile.crf)];
  }
  return [
    "-c:v", "libx264", "-preset", profile.preset, "-crf", String(profile.crf),
    "-threads", String(profile.encoderThreads),
  ];
}

async function videoEncoderWorks(encoder, profile) {
  try {
    await run(ffmpegCommand(), [
      "-hide_banner", "-loglevel", "error",
      "-f", "lavfi", "-i", "color=c=black:s=64x64:r=2:d=0.5",
      "-frames:v", "1", ...videoEncoderArgs({ ...profile, encoder }),
      "-pix_fmt", "yuv420p", "-f", "null", "-",
    ]);
    return true;
  } catch (_) {
    return false;
  }
}

async function resolveVideoProfile(value) {
  const profile = videoProfile(value);
  const requested = String(process.env.UNIPPT_VIDEO_ENCODER || "software").toLowerCase();
  if (["software", "libx264", "off", "0"].includes(requested)) {
    return { ...profile, encoder: "libx264", hardwareEncoder: false };
  }
  const candidates = requested === "auto"
    ? (process.platform === "win32"
      ? ["h264_qsv", "h264_nvenc", "h264_amf"]
      : ["h264_nvenc", "h264_qsv", "h264_amf"])
    : [requested];
  for (const encoder of candidates) {
    if (!["h264_qsv", "h264_nvenc", "h264_amf"].includes(encoder)) continue;
    if (await videoEncoderWorks(encoder, profile)) {
      emitDiagnostic(`video encoder selected: ${encoder}`);
      return { ...profile, encoder, hardwareEncoder: true };
    }
  }
  emitDiagnostic(`video encoder ${requested} is unavailable; falling back to libx264`);
  return { ...profile, encoder: "libx264", hardwareEncoder: false };
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

async function readRenderDocument(input) {
  const html = await readFile(input, "utf8");
  const open = '<script type="application/x-unippt+json" id="unippt-data">';
  const start = html.indexOf(open);
  const end = start < 0 ? -1 : html.indexOf("</script>", start + open.length);
  if (start < 0 || end < 0) throw new Error("UniPPT render payload marker is missing");
  const payload = JSON.parse(html.slice(start + open.length, end));
  return { html, open, start, end, payload };
}

function renderManifestFromDocument(document) {
  const { html, start, end, payload } = document;
  const deck = payload.deck;
  if (!deck || !Array.isArray(deck.slides)) throw new Error("UniPPT render deck is missing");
  const runtimeHash = sha256(`${html.slice(0, start)}${html.slice(end + 9)}`);
  const blobIndex = new Map((payload.blobs || []).map((blob) => [blob.path, blob]));
  const assets = new Map();
  const assetIndex = (payload.assets || []).map((asset) => {
    const blobPath = asset.browserBlob || asset.originalBlob;
    const encoded = blobPath
      ? blobIndex.get(blobPath)?.base64
      : (asset.browserBase64 || asset.originalBase64);
    assets.set(asset.id, {
      ...asset,
      mimeType: asset.browserMimeType || asset.mimeType || "application/octet-stream",
      encoded,
    });
    return {
    id: asset.id,
    mimeType: asset.mimeType,
    browserMimeType: asset.browserMimeType,
    originalBlob: asset.originalBlob,
    browserBlob: asset.browserBlob,
    };
  });
  return { deck, runtimeHash, assetIndex, assets };
}

async function readRenderManifest(input) {
  return renderManifestFromDocument(await readRenderDocument(input));
}

function assetExtension(mimeType) {
  const mime = String(mimeType || "").toLowerCase().split(";", 1)[0];
  return ({
    "image/png": ".png",
    "image/jpeg": ".jpg",
    "image/gif": ".gif",
    "image/webp": ".webp",
    "image/svg+xml": ".svg",
    "font/ttf": ".ttf",
    "font/otf": ".otf",
    "font/woff": ".woff",
    "font/woff2": ".woff2",
    "application/vnd.ms-fontobject": ".eot",
    "audio/mpeg": ".mp3",
    "audio/mp4": ".m4a",
    "audio/wav": ".wav",
    "audio/ogg": ".ogg",
    "video/mp4": ".mp4",
    "video/webm": ".webm",
  })[mime] || ".bin";
}

function contentTypeForAsset(filename) {
  const extension = path.extname(filename).toLowerCase();
  return ({
    ".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg",
    ".gif": "image/gif", ".webp": "image/webp", ".svg": "image/svg+xml",
    ".ttf": "font/ttf", ".otf": "font/otf", ".woff": "font/woff",
    ".woff2": "font/woff2", ".eot": "application/vnd.ms-fontobject",
    ".mp3": "audio/mpeg", ".m4a": "audio/mp4", ".wav": "audio/wav",
    ".ogg": "audio/ogg", ".mp4": "video/mp4", ".webm": "video/webm",
  })[extension] || "application/octet-stream";
}

async function persistRenderAsset(cacheDir, blob, mimeType) {
  const bytes = Buffer.from(String(blob.base64 || ""), "base64");
  const digest = /^[0-9a-f]{64}$/i.test(String(blob.sha256 || ""))
    ? String(blob.sha256).toLowerCase()
    : sha256(bytes);
  const filename = `${digest}${assetExtension(mimeType)}`;
  const destination = path.join(cacheDir, filename);
  if (!(await exists(destination))) {
    try {
      await writeFile(destination, bytes, { flag: "wx" });
    } catch (error) {
      if (error?.code !== "EEXIST") throw error;
    }
  }
  return filename;
}

async function prepareRenderDocument(input, assetCacheDir) {
  const document = await readRenderDocument(input);
  const supportsExternalAssets = document.html.includes(
    "src.url?new URL(src.url,document.baseURI).href",
  );
  if (!supportsExternalAssets) {
    const token = sha256(`${input}:${Date.now()}:${Math.random()}`).slice(0, 24);
    renderDocuments.set(token, document.html);
    return {
      token,
      manifest: renderManifestFromDocument(document),
      embeddedBytes: 0,
      htmlBytes: Buffer.byteLength(document.html),
      legacyInline: true,
    };
  }
  const mimeByBlob = new Map();
  for (const asset of document.payload.assets || []) {
    if (asset.originalBlob) mimeByBlob.set(asset.originalBlob, asset.mimeType);
    if (asset.browserBlob) mimeByBlob.set(asset.browserBlob, asset.browserMimeType || asset.mimeType);
  }
  await mkdir(assetCacheDir, { recursive: true });
  const blobs = [];
  let embeddedBytes = 0;
  for (const blob of document.payload.blobs || []) {
    embeddedBytes += Math.max(0, Number(blob.size) || 0);
    const filename = await persistRenderAsset(assetCacheDir, blob, mimeByBlob.get(blob.path));
    blobs.push({
      path: blob.path,
      size: blob.size,
      sha256: blob.sha256,
      url: `/__unippt_asset/${filename}`,
    });
  }
  const payload = { ...document.payload, blobs };
  const html = `${document.html.slice(0, document.start + document.open.length)}${JSON.stringify(payload)}${document.html.slice(document.end)}`;
  const token = sha256(`${input}:${Date.now()}:${Math.random()}`).slice(0, 24);
  renderDocuments.set(token, html);
  return {
    token,
    manifest: renderManifestFromDocument(document),
    embeddedBytes,
    htmlBytes: Buffer.byteLength(html),
    legacyInline: false,
  };
}

function playerUrl(port, token) {
  return `http://127.0.0.1:${port}/__unippt_render/${token}.html`;
}

async function openPlayer(context, input, viewport) {
  const page = await context.newPage();
  await page.setViewportSize(viewport);
  page.on("pageerror", (error) => emitDiagnostic(`renderer page error: ${error?.stack || error}`));
  page.on("requestfailed", (request) => emitDiagnostic(
    `renderer request failed: ${request.url()} (${request.failure()?.errorText || "unknown"})`,
  ));
  page.on("console", (message) => {
    if (["error", "warning"].includes(message.type())) {
      emitDiagnostic(`renderer console ${message.type()}: ${message.text()}`);
    }
  });
  await page.addInitScript(() => {
    globalThis.__UNIPPT_CAPTURE_MODE__ = true;
  });
  const url = /^https?:\/\//i.test(String(input)) ? String(input) : pathToFileURL(input).href;
  await page.goto(url, { waitUntil: "load", timeout: 120_000 });
  await page.waitForFunction(() => Boolean(globalThis.UniPptCapture), null, { timeout: 120_000 });
  const metadata = await page.evaluate(() => globalThis.UniPptCapture.ready());
  await page.addStyleTag({ content: "#playerControls,#progress{display:none!important}" });
  return { page, metadata };
}

async function renderSlides(context, input, output, width) {
  const initial = await openPlayer(context, input, { width, height: Math.round(width * 9 / 16) });
  const height = Math.max(1, Math.round(width * initial.metadata.height / initial.metadata.width));
  await initial.page.close();
  const { page, metadata } = await openPlayer(context, input, { width, height });
  await mkdir(output, { recursive: true });
  const digits = String(metadata.slideCount).length;
  for (let index = 0; index < metadata.slideCount; index += 1) {
    await page.evaluate((slide) => globalThis.UniPptCapture.showSlide(slide, { final: true }), index);
    const filename = `slide-${String(index + 1).padStart(digits, "0")}.png`;
    await page.locator("#stage").screenshot({ path: path.join(output, filename), animations: "disabled" });
  }
  await writeFile(path.join(output, "manifest.json"), JSON.stringify({
    format: "unippt-slide-images",
    width,
    height,
    slideCount: metadata.slideCount,
  }, null, 2));
  await page.close();
  return { width, height, slideCount: metadata.slideCount };
}

async function renderPdf(context, input, output, width, workspace) {
  const { page } = await openPlayer(context, input, { width, height: Math.round(width * 9 / 16) });
  const metadata = await page.evaluate(() => globalThis.UniPptCapture.renderPrintPages());
  await page.addStyleTag({ content: `
    @page{size:${metadata.width}px ${metadata.height}px;margin:0}
    html,body,#unipptPrintPages{margin:0!important;padding:0!important;background:#fff!important}
  ` });
  await page.pdf({
    path: output,
    width: `${metadata.width}px`,
    height: `${metadata.height}px`,
    printBackground: true,
    preferCSSPageSize: true,
    margin: { top: "0", right: "0", bottom: "0", left: "0" },
  });
  await page.close();
}

async function installOfflineClock(page) {
  await page.evaluate(() => {
    const nativeSetTimeout = globalThis.setTimeout.bind(globalThis);
    const nativeClearTimeout = globalThis.clearTimeout.bind(globalThis);
    const nativeAnimate = Element.prototype.animate;
    let clock = 0;
    let sequence = 1;
    const timers = new Map();
    const animations = [];

    globalThis.setTimeout = (callback, delay = 0, ...args) => {
      const id = sequence++;
      timers.set(id, {
        at: clock + Math.max(0, Number(delay) || 0),
        callback,
        args,
      });
      return id;
    };
    globalThis.clearTimeout = (id) => timers.delete(id);
    Element.prototype.animate = function captureAnimate(keyframes, options) {
      const animation = nativeAnimate.call(this, keyframes, options);
      animation.pause();
      animation.currentTime = 0;
      animations.push({ animation, createdAt: clock });
      return animation;
    };

    function seekAnimations(at) {
      let active = 0;
      let changed = false;
      for (const entry of animations) {
        const animation = entry.animation;
        if (animation.playState === "idle") continue;
        const end = Number(animation.effect?.getComputedTiming?.().endTime) || 0;
        const value = Math.max(0, at - entry.createdAt);
        const next = end > 0 ? Math.min(value, end) : value;
        if (end <= 0 || value < end) active += 1;
        if (entry.currentTime !== next) changed = true;
        entry.currentTime = next;
        try { animation.currentTime = next; } catch (_) {}
      }
      return { active, changed };
    }

    globalThis.__unipptAdvanceCaptureClock = async (delta) => {
      const target = clock + Math.max(0, Number(delta) || 0);
      let guard = 0;
      let fired = 0;
      let animationState = { active: 0, changed: false };
      while (guard++ < 100_000) {
        let nextId = null;
        let next = null;
        for (const [id, timer] of timers) {
          if (timer.at <= target && (!next || timer.at < next.at || (timer.at === next.at && id < nextId))) {
            nextId = id;
            next = timer;
          }
        }
        if (!next) break;
        timers.delete(nextId);
        clock = next.at;
        animationState = seekAnimations(clock);
        next.callback(...next.args);
        fired += 1;
        await Promise.resolve();
      }
      if (guard >= 100_000) throw new Error("UniPPT offline timer starvation");
      clock = target;
      animationState = seekAnimations(clock);
      await Promise.resolve();
      const playingMedia = [...document.querySelectorAll("video")]
        .some((node) => !node.paused && !node.ended && node.readyState >= 2);
      const changed = fired > 0 || animationState.changed || playingMedia;
      // `Page.captureScreenshot` performs its own compositor flush. Waiting a
      // real requestAnimationFrame here throttled an offline export toward
      // wall-clock playback speed. A synchronous layout read is sufficient to
      // publish the newly sought WAAPI state before the CDP capture.
      if (changed) {
        void document.documentElement.offsetHeight;
        await Promise.resolve();
      }
      return {
        clock,
        timers: timers.size,
        animations: animations.length,
        activeAnimations: animationState.active,
        changed,
      };
    };
    globalThis.__unipptRestoreCaptureClock = () => {
      globalThis.setTimeout = nativeSetTimeout;
      globalThis.clearTimeout = nativeClearTimeout;
      Element.prototype.animate = nativeAnimate;
    };
  });
}

async function captureFrame(cdp, profile) {
  const options = {
    format: profile.frameFormat,
    fromSurface: true,
    captureBeyondViewport: false,
    optimizeForSpeed: true,
  };
  if (profile.frameFormat === "jpeg") options.quality = profile.jpegQuality;
  const { data } = await cdp.send("Page.captureScreenshot", options);
  return Buffer.from(data, "base64");
}

async function renderVideoSegment(
  page,
  cdp,
  slideIndex,
  output,
  outputWidth,
  outputHeight,
  fps,
  profile,
  onProgress,
) {
  // Let the native browser clock finish images/fonts and build the outgoing
  // slide before we replace timers with the deterministic capture clock.
  // Transitions such as page curl and wind sample that outgoing surface; doing
  // this after clock installation can capture a blank or half-decoded page.
  await page.evaluate(
    (index) => globalThis.UniPptCapture.prepareSlidePlayback(index),
    slideIndex,
  );
  await installOfflineClock(page);
  await page.evaluate(
    (index) => globalThis.UniPptCapture.startPreparedSlidePlayback(index, { settle: false }),
    slideIndex,
  );
  const frameDuration = 1000 / fps;
  const maxFrames = fps * 60 * 10;
  let frames = 0;
  let captures = 0;
  let uniqueFrames = 0;
  let pendingFrames = 0;
  let encoder = null;
  try {
    let frame = await captureFrame(cdp, profile);
    captures += 1;
    uniqueFrames += 1;
    while (frames < maxFrames) {
      frames += 1;
      pendingFrames += 1;
      if (frames === 1 || frames % Math.max(1, Math.round(fps / 2)) === 0) {
        onProgress?.(frames);
      }
      const completed = await page.evaluate(
        () => Boolean(globalThis.UniPptCapture?.segmentCompleted),
      );
      if (completed) break;
      const state = await page.evaluate(
        (step) => globalThis.__unipptAdvanceCaptureClock(step),
        frameDuration,
      );
      if (state.changed) {
        const nextFrame = await captureFrame(cdp, profile);
        captures += 1;
        if (!nextFrame.equals(frame)) {
          uniqueFrames += 1;
          encoder ||= startFrameEncoder(output, outputWidth, outputHeight, fps, profile);
          for (let index = 0; index < pendingFrames; index += 1) {
            await writeStream(encoder.child.stdin, frame);
          }
          frame = nextFrame;
          pendingFrames = 0;
        }
      }
    }
    if (frames >= maxFrames) {
      throw new Error(`slide ${slideIndex + 1} exceeds the 10 minute safety limit`);
    }
    if (encoder) {
      for (let index = 0; index < pendingFrames; index += 1) {
        await writeStream(encoder.child.stdin, frame);
      }
      encoder.child.stdin.end();
      await encoder.completed;
    } else {
      await encodeStillFrame(output, outputWidth, outputHeight, fps, profile, frame, frames);
    }
    onProgress?.(frames, true);
  } catch (error) {
    encoder?.child.stdin.destroy();
    encoder?.child.kill();
    throw error;
  } finally {
    await page.evaluate(() => globalThis.__unipptRestoreCaptureClock?.());
  }
  return { slideIndex, frames, captures, uniqueFrames, staticSegment: !encoder };
}

function concatPath(value) {
  return path.resolve(value).replaceAll("\\", "/").replaceAll("'", "'\\''");
}

function animationActiveDuration(animation) {
  const duration = Math.max(1, Number(animation?.durationMs) || 500);
  const repeats = Number(animation?.repeatCount);
  const iterations = Number.isFinite(repeats) && repeats > 0 ? repeats : 1;
  return duration * iterations * (animation?.autoReverse ? 2 : 1);
}

function animationSchedule(animations, clickGap = 180) {
  const schedule = [];
  let cursor = 0;
  let batchOrigin = 0;
  while (cursor < animations.length) {
    const startIndex = cursor;
    let groupStart = 0;
    let groupEnd = 0;
    let batchEnd = 0;
    while (cursor < animations.length) {
      const animation = animations[cursor];
      const trigger = animation.trigger || "onClick";
      if (cursor > startIndex && trigger === "onClick") break;
      const delay = Math.max(0, Number(animation.delayMs) || 0);
      let start;
      if (cursor === startIndex) {
        start = delay;
        groupStart = start;
      } else if (trigger === "withPrevious") {
        start = groupStart + delay;
      } else {
        start = groupEnd + delay;
        groupStart = start;
        groupEnd = start;
      }
      const end = start + animationActiveDuration(animation);
      schedule.push({ animation, start: batchOrigin + start, end: batchOrigin + end });
      groupEnd = Math.max(groupEnd, end);
      batchEnd = Math.max(batchEnd, end);
      cursor += 1;
    }
    batchOrigin += batchEnd + (cursor < animations.length ? clickGap : 0);
  }
  return schedule;
}

function collectSlideObjects(slide) {
  const index = new Map();
  const visit = (object) => {
    if (!object) return;
    if (object.id) index.set(object.id, object);
    for (const child of object.children || []) visit(child);
  };
  for (const object of [
    ...(slide.masterObjects || []),
    ...(slide.layoutObjects || []),
    ...(slide.objects || []),
  ]) visit(object);
  return index;
}

function assetId(value) {
  const match = /^unippt-asset:([0-9a-f]{64})$/i.exec(String(value || ""));
  return match?.[1] || null;
}

function buildAudioPlan(manifest, segmentDurations) {
  const plans = [];
  const seenPersistent = new Set();
  let slideOffset = 0;
  const totalDuration = segmentDurations.reduce((sum, value) => sum + value, 0);
  for (let slideIndex = 0; slideIndex < manifest.deck.slides.length; slideIndex += 1) {
    const slide = manifest.deck.slides[slideIndex];
    const objects = collectSlideObjects(slide);
    const effects = [...(slide.inheritedAnimations || []), ...(slide.animations || [])]
      .sort((left, right) => (left.order || 0) - (right.order || 0));
    for (const entry of animationSchedule(effects)) {
      const effect = entry.animation;
      if (!(effect.effect === "media" || effect.class === "media")) continue;
      if ((effect.mediaAction || "play") !== "play") continue;
      const object = objects.get(effect.targetObjectId || "");
      const media = object?.media;
      if (!media) continue;
      const id = assetId(media.playbackAsset || media.asset);
      const asset = id ? manifest.assets.get(id) : null;
      if (!asset?.encoded) continue;
      const persistentKey = `${effect.targetObjectId || object.id}:${id}`;
      if (media.playAcrossSlides && seenPersistent.has(persistentKey)) continue;
      if (media.playAcrossSlides) seenPersistent.add(persistentKey);
      const start = slideOffset + (entry.start / 1000);
      const slideRemaining = Math.max(0.04, segmentDurations[slideIndex] - (entry.start / 1000));
      plans.push({
        id,
        asset,
        start,
        duration: media.playAcrossSlides ? Math.max(0.04, totalDuration - start) : slideRemaining,
        trimStart: Math.max(0, Number(media.trimStartMs) || 0) / 1000,
        trimEnd: Math.max(0, Number(media.trimEndMs) || 0) / 1000,
        volume: Math.max(0, Math.min(1, Number(media.volume ?? 1))),
        loop: Boolean(media.loopPlayback),
      });
    }
    slideOffset += segmentDurations[slideIndex];
  }
  return { plans, totalDuration };
}

function mediaExtension(mimeType) {
  const mime = String(mimeType || "").toLowerCase();
  if (mime.includes("mpeg")) return ".mp3";
  if (mime.includes("wav")) return ".wav";
  if (mime.includes("ogg")) return ".ogg";
  if (mime.includes("webm")) return ".webm";
  if (mime.includes("mp4") || mime.includes("m4a")) return ".m4a";
  if (mime.includes("avi")) return ".avi";
  return ".bin";
}

async function probeAudioMetadata(input) {
  // One probe supplies both stream presence and duration, once per source.
  // A broken probe must fail export, not silently drop the user's audio.
  const output = await runOutput(ffprobeCommand(), [
    "-v", "error", "-show_entries", "stream=codec_type:format=duration",
    "-of", "json", input,
  ]);
  const metadata = JSON.parse(output);
  return {
    hasAudio: (metadata.streams || []).some((stream) => stream.codec_type === "audio"),
    duration: Number(metadata.format?.duration),
  };
}

async function prepareAudioInputs(manifest, plan, workspace) {
  const audioDir = path.join(workspace, "audio-assets");
  await mkdir(audioDir, { recursive: true });
  const materialized = new Map();
  const metadataCache = new Map();
  const loopSources = new Map();
  const inputs = [];
  for (let index = 0; index < plan.plans.length; index += 1) {
    const item = plan.plans[index];
    let source = materialized.get(item.id);
    if (!source) {
      source = path.join(audioDir, `${item.id}${mediaExtension(item.asset.mimeType)}`);
      await writeFile(source, Buffer.from(item.asset.encoded, "base64"));
      materialized.set(item.id, source);
    }
    let metadata = metadataCache.get(item.id);
    if (!metadata) {
      metadata = await probeAudioMetadata(source);
      metadataCache.set(item.id, metadata);
    }
    if (!metadata.hasAudio) continue;
    if (!Number.isFinite(metadata.duration) || metadata.duration <= 0) {
      throw new Error(`invalid audio duration: ${source}`);
    }
    const trimStop = Math.max(0, metadata.duration - item.trimEnd);
    const available = Math.max(0, trimStop - item.trimStart);
    if (!available) continue;
    if (item.loop) {
      if (available >= item.duration) {
        inputs.push({ ...item, source, trimStop, prepared: false, loop: false });
        continue;
      }
      const loopKey = JSON.stringify([item.id, item.trimStart, trimStop]);
      let loopSource = loopSources.get(loopKey);
      if (!loopSource) {
        loopSource = path.join(audioDir, `loop-${index}.wav`);
        await run(ffmpegCommand(), [
          "-y", "-i", source, "-vn",
          "-af", `aresample=48000,atrim=start=${item.trimStart}:end=${trimStop},asetpts=PTS-STARTPTS`,
          "-c:a", "pcm_s16le", loopSource,
        ]);
        loopSources.set(loopKey, loopSource);
      }
      inputs.push({ ...item, source: loopSource, prepared: true });
    } else {
      inputs.push({ ...item, source, trimStop, prepared: false });
    }
  }
  return inputs;
}

async function muxPresentationAudio(manifest, silentVideo, output, segmentDurations, workspace) {
  const plan = buildAudioPlan(manifest, segmentDurations);
  if (!plan.plans.length) {
    await rename(silentVideo, output);
    return { tracks: 0 };
  }
  const inputs = await prepareAudioInputs(manifest, plan, workspace);
  if (!inputs.length) {
    await rename(silentVideo, output);
    return { tracks: 0 };
  }
  emitProgress("audio", { tracks: inputs.length });
  const args = ["-y", "-i", silentVideo];
  for (const input of inputs) {
    if (input.loop) args.push("-stream_loop", "-1");
    args.push("-i", input.source);
  }
  const filters = [];
  const labels = [];
  inputs.forEach((input, index) => {
    const label = `a${index}`;
    const chain = [`[${index + 1}:a]aresample=48000`];
    if (!input.prepared && (input.trimStart > 0 || input.trimEnd > 0)) {
      chain.push(`atrim=start=${input.trimStart}:end=${input.trimStop}`);
    }
    chain.push(
      "asetpts=PTS-STARTPTS",
      `atrim=duration=${input.duration}`,
      `volume=${input.volume}`,
      `adelay=${Math.max(0, Math.round(input.start * 1000))}|${Math.max(0, Math.round(input.start * 1000))}[${label}]`,
    );
    filters.push(chain.join(","));
    labels.push(`[${label}]`);
  });
  if (labels.length === 1) {
    filters.push(`${labels[0]}apad=whole_dur=${plan.totalDuration},atrim=duration=${plan.totalDuration}[aout]`);
  } else {
    filters.push(`${labels.join("")}amix=inputs=${labels.length}:duration=longest:dropout_transition=0:normalize=0,apad=whole_dur=${plan.totalDuration},atrim=duration=${plan.totalDuration}[aout]`);
  }
  args.push(
    "-filter_complex", filters.join(";"),
    "-map", "0:v:0", "-map", "[aout]",
    "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
    "-movflags", "+faststart", "-t", String(plan.totalDuration), output,
  );
  await run(ffmpegCommand(), args);
  return { tracks: inputs.length };
}

async function launchRendererBrowser() {
  const executablePath = await findChromium();
  return chromium.launch({
    headless: true,
    executablePath,
    args: [
      "--autoplay-policy=no-user-gesture-required",
      "--disable-background-timer-throttling",
      "--disable-backgrounding-occluded-windows",
      "--disable-renderer-backgrounding",
      "--hide-scrollbars",
      "--no-first-run",
      ...(process.platform === "linux" ? ["--no-sandbox", "--disable-dev-shm-usage"] : []),
    ],
  });
}

async function launchRendererContext() {
  const browser = await launchRendererBrowser();
  const context = await browser.newContext({
    viewport: { width: 1280, height: 720 },
    deviceScaleFactor: 1,
  });
  return { browser, context };
}

async function persistentRendererContext() {
  if (!rendererBrowserPromise) {
    rendererBrowserPromise = launchRendererBrowser().catch((error) => {
      rendererBrowserPromise = null;
      throw error;
    });
  }
  const browser = await rendererBrowserPromise;
  if (!browser.isConnected()) {
    rendererBrowserPromise = null;
    rendererContextPromise = null;
    return persistentRendererContext();
  }
  if (!rendererContextPromise) {
    rendererContextPromise = browser.newContext({
      viewport: { width: 1280, height: 720 },
      deviceScaleFactor: 1,
    }).catch((error) => {
      rendererContextPromise = null;
      throw error;
    });
  }
  return rendererContextPromise;
}

async function renderVideo(context, input, manifest, output, width, height, fps, workspace, profileName, muted) {
  const profile = await resolveVideoProfile(profileName);
  const renderWidth = even(width * profile.scale);
  const renderHeight = even(height * profile.scale);
  const slideCount = manifest.deck.slides.length;
  emitProgress("preparing", { totalSlides: slideCount });
  // Chromium capture and single-thread x264 segments scale up to all eight
  // logical processors on the reference 4C/8T machine.  Respect CPU affinity
  // through availableParallelism(), cap at the measured range, and retain an
  // explicit environment override for CI and constrained hosts.
  const workerCount = renderWorkerCount(slideCount, process.env.UNIPPT_RENDER_WORKERS);
  const segmentsDir = path.join(workspace, "video-segments");
  const cacheDir = path.resolve(
    process.env.UNIPPT_VIDEO_SEGMENT_CACHE
      || path.join(tmpdir(), "unippt-video-segments-v6"),
  );
  await Promise.all([mkdir(segmentsDir, { recursive: true }), mkdir(cacheDir, { recursive: true })]);
  const segmentKeys = Array.from(
    { length: slideCount },
    (_, index) => segmentCacheKey(manifest, index, profile, width, height, fps),
  );
  const segmentPaths = segmentKeys.map((key) => path.join(cacheDir, `${key}.mp4`));
  const segmentMetaPaths = segmentKeys.map((key) => path.join(cacheDir, `${key}.json`));
  const reports = new Array(slideCount);
  const missingSlides = [];
  for (let index = 0; index < slideCount; index += 1) {
    if (await exists(segmentPaths[index]) && await exists(segmentMetaPaths[index])) {
      try {
        const metadata = JSON.parse(await readFile(segmentMetaPaths[index], "utf8"));
        if (Number(metadata.frames) > 0) {
          reports[index] = { ...metadata, slideIndex: index, cached: true };
          continue;
        }
      } catch (_) {}
    }
    missingSlides.push(index);
  }
  const cachedSlides = slideCount - missingSlides.length;
  let completedSlides = cachedSlides;
  let renderedFrames = 0;
  emitProgress("cache", {
    completedSlides,
    totalSlides: slideCount,
    cachedSlides,
    frames: renderedFrames,
  });
  let nextMissing = 0;

  const worker = async () => {
    const { page } = await openPlayer(context, input, {
      width: renderWidth,
      height: renderHeight,
    });
    const cdp = await context.newCDPSession(page);
    try {
      while (true) {
        const slideIndex = missingSlides[nextMissing];
        nextMissing += 1;
        if (slideIndex == null) break;
        const temporary = path.join(
          segmentsDir,
          `${segmentKeys[slideIndex]}-${process.pid}-${Math.random().toString(36).slice(2)}.mp4`,
        );
        const report = await renderVideoSegment(
          page,
          cdp,
          slideIndex,
          temporary,
          width,
          height,
          fps,
          profile,
          (() => {
            let reportedFrames = 0;
            return (slideFrames) => {
              const delta = Math.max(0, slideFrames - reportedFrames);
              reportedFrames = slideFrames;
              renderedFrames += delta;
              emitProgress("rendering", {
                completedSlides,
                totalSlides: slideCount,
                cachedSlides,
                currentSlide: slideIndex + 1,
                frames: renderedFrames,
              });
            };
          })(),
        );
        try {
          await publishCachedSegment(temporary, segmentPaths[slideIndex]);
        } catch (error) {
          if (!(await exists(segmentPaths[slideIndex]))) throw error;
        }
        await writeFile(segmentMetaPaths[slideIndex], JSON.stringify({
          slideIndex,
          frames: report.frames,
          captures: report.captures,
          uniqueFrames: report.uniqueFrames,
          staticSegment: report.staticSegment,
          encoder: profile.encoder,
        }));
        reports[slideIndex] = { ...report, cached: false };
        completedSlides += 1;
        emitProgress("rendering", {
          completedSlides,
          totalSlides: slideCount,
          cachedSlides,
          currentSlide: slideIndex + 1,
          frames: renderedFrames,
        });
      }
    } finally {
      await page.close();
    }
  };
  if (missingSlides.length) {
    await Promise.all(
      Array.from(
        { length: Math.min(workerCount, missingSlides.length) },
        () => worker(),
      ),
    );
  }

  const concatFile = path.join(workspace, "video-segments.txt");
  await writeFile(
    concatFile,
    segmentPaths.map((entry) => `file '${concatPath(entry)}'`).join("\n"),
  );
  emitProgress("encoding", {
    completedSlides: slideCount,
    totalSlides: slideCount,
    cachedSlides,
    frames: renderedFrames,
  });
  const silentOutput = muted ? output : path.join(workspace, "silent-video.mp4");
  await run(ffmpegCommand(), [
    "-y", "-f", "concat", "-safe", "0", "-i", concatFile,
    "-c", "copy", "-movflags", "+faststart", silentOutput,
  ]);
  const segmentDurations = reports.map((report) => Math.max(1, Number(report.frames) || 1) / fps);
  const audio = muted
    ? { tracks: 0 }
    : await muxPresentationAudio(manifest, silentOutput, output, segmentDurations, workspace);
  const frames = reports.reduce((sum, report) => sum + report.frames, 0);
  const captures = reports.reduce((sum, report) => sum + report.captures, 0);
  const uniqueFrames = reports.reduce((sum, report) => sum + (Number(report.uniqueFrames) || Number(report.captures) || 0), 0);
  const staticSegments = reports.filter((report) => report.staticSegment).length;
  const cached = reports.filter((report) => report.cached).length;
  emitProgress("rendered", {
    completedSlides: slideCount,
    totalSlides: slideCount,
    cachedSlides: cached,
    frames,
  });
  emitDiagnostic(
    `offline video rendered ${frames} frame(s) at ${fps} FPS from ${captures} Chromium capture(s) and ${uniqueFrames} unique frame(s); staticSegments=${staticSegments}/${slideCount}, cached=${cached}/${slideCount}, workers=${Math.min(workerCount, missingSlides.length)}, profile=${profile.name}, encoder=${profile.encoder}, capture=${renderWidth}x${renderHeight}, output=${width}x${height}, audioTracks=${audio.tracks}, muted=${muted}`,
  );
}

function renderAssetCacheDir() {
  return path.resolve(
    process.env.UNIPPT_RENDER_ASSET_CACHE
      || path.join(tmpdir(), RENDER_ASSET_CACHE_SCHEMA),
  );
}

async function readJsonRequest(request, limit = 1024 * 1024) {
  const chunks = [];
  let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > limit) throw new Error("renderer request is too large");
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}

async function runRenderRequest(rawArgs, options = {}) {
  const format = rawArgs.format;
  const input = path.resolve(rawArgs.input || "");
  const output = path.resolve(rawArgs.output || "");
  const workspace = path.resolve(rawArgs.workspace || path.dirname(output));
  const width = Math.max(320, Number(rawArgs.width) || 1920);
  const height = Math.max(180, Number(rawArgs.height) || 1080);
  const fps = Math.max(1, Number(rawArgs.fps) || 30);
  const profile = rawArgs.profile || process.env.UNIPPT_VIDEO_PROFILE || "quality";
  const muted = String(rawArgs.muted || "false").toLowerCase() === "true";
  if (!(await exists(input))) throw new Error(`input HTML does not exist: ${input}`);
  await mkdir(workspace, { recursive: true });
  const bundle = await prepareRenderDocument(input, renderAssetCacheDir());
  const renderInput = playerUrl(options.serverPort, bundle.token);
  emitDiagnostic(bundle.legacyInline
    ? `v6 render transport kept legacy inline assets; player HTML=${bundle.htmlBytes} byte(s)`
    : `v6 render transport externalized ${bundle.embeddedBytes} byte(s); player HTML=${bundle.htmlBytes} byte(s)`);
  let localSession = null;
  const context = options.persistent
    ? await persistentRendererContext()
    : (localSession = await launchRendererContext()).context;
  try {
    if (format === "video") {
      await renderVideo(
        context, renderInput, bundle.manifest, output,
        width, height, fps, workspace, profile, muted,
      );
    } else if (format === "images") {
      await renderSlides(context, renderInput, output, width);
    } else if (format === "pdf") {
      await renderPdf(context, renderInput, output, width, workspace);
    } else {
      throw new Error(`unsupported format: ${format}`);
    }
  } finally {
    renderDocuments.delete(bundle.token);
    if (localSession) {
      await localSession.context.close().catch(() => {});
      await localSession.browser.close().catch(() => {});
    }
  }
  return { ok: true, format, output, renderer: "chromium-ffmpeg-v6" };
}

let renderQueue = Promise.resolve();

async function startRenderHttpServer({ daemon = false } = {}) {
  const assetCache = renderAssetCacheDir();
  await mkdir(assetCache, { recursive: true });
  let serverPort = 0;
  const server = createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url || "/", `http://127.0.0.1:${serverPort || 80}`);
      if (request.method === "GET" && requestUrl.pathname === "/favicon.ico") {
        response.writeHead(204, { "Cache-Control": "public, max-age=86400" });
        response.end();
        return;
      }
      if (request.method === "GET" && requestUrl.pathname.startsWith("/__unippt_render/")) {
        const match = /^\/__unippt_render\/([0-9a-f]{24})\.html$/i.exec(requestUrl.pathname);
        const html = match ? renderDocuments.get(match[1]) : null;
        if (!html) {
          response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
          response.end("render document not found");
          return;
        }
        response.writeHead(200, {
          "Content-Type": "text/html; charset=utf-8",
          "Cache-Control": "no-store",
          "Content-Length": String(Buffer.byteLength(html)),
        });
        response.end(html);
        return;
      }
      if (request.method === "GET" && requestUrl.pathname.startsWith("/__unippt_asset/")) {
        const match = /^\/__unippt_asset\/([0-9a-f]{64}\.[a-z0-9]+)$/i.exec(requestUrl.pathname);
        const filename = match?.[1];
        const source = filename ? path.join(assetCache, filename) : "";
        if (!filename || !(await exists(source))) {
          response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
          response.end("render asset not found");
          return;
        }
        const metadata = await stat(source);
        response.writeHead(200, {
          "Content-Type": contentTypeForAsset(filename),
          "Content-Length": String(metadata.size),
          "Cache-Control": "public, max-age=31536000, immutable",
          "Access-Control-Allow-Origin": "*",
        });
        createReadStream(source).pipe(response);
        return;
      }
      if (daemon && request.method === "POST" && requestUrl.pathname === "/render") {
        const args = await readJsonRequest(request);
        response.writeHead(200, {
          "Content-Type": "application/x-ndjson; charset=utf-8",
          "Cache-Control": "no-store",
          "X-Content-Type-Options": "nosniff",
        });
        response.write("UNIPPT_READY\n");
        const execute = () => progressOutput.run(response, () => runRenderRequest(args, {
          persistent: true,
          serverPort,
        }));
        const task = renderQueue.then(execute, execute);
        renderQueue = task.catch(() => {});
        try {
          const result = await task;
          response.write(`UNIPPT_DONE ${JSON.stringify(result)}\n`);
        } catch (error) {
          response.write(`UNIPPT_ERROR ${JSON.stringify({ message: String(error?.stack || error) })}\n`);
        }
        response.end();
        return;
      }
      response.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
      response.end("not found");
    } catch (error) {
      if (!response.headersSent) {
        response.writeHead(400, { "Content-Type": "text/plain; charset=utf-8" });
      }
      response.end(String(error?.stack || error));
    }
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  serverPort = Number(server.address()?.port) || 0;
  if (!serverPort) throw new Error("renderer HTTP server did not bind a port");
  return { server, port: serverPort };
}

function closeHttpServer(server) {
  return new Promise((resolve) => server.close(resolve));
}

function monitorParent(parentPid) {
  const pid = Number(parentPid);
  if (!Number.isInteger(pid) || pid <= 0) return;
  const timer = setInterval(() => {
    try {
      process.kill(pid, 0);
    } catch (_) {
      clearInterval(timer);
      process.exit(0);
    }
  }, 2000);
  timer.unref?.();
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const daemon = String(args.serve || "false").toLowerCase() === "true";
  const http = await startRenderHttpServer({ daemon });
  if (daemon) {
    monitorParent(args.parentPid);
    process.stdout.write(`UNIPPT_RENDER_DAEMON ${http.port}\n`);
    const shutdown = async () => {
      await rendererContextPromise?.then((context) => context.close()).catch(() => {});
      await rendererBrowserPromise?.then((browser) => browser.close()).catch(() => {});
      await closeHttpServer(http.server).catch(() => {});
      process.exit(0);
    };
    process.once("SIGTERM", shutdown);
    process.once("SIGINT", shutdown);
    return;
  }
  try {
    const result = await runRenderRequest(args, { persistent: false, serverPort: http.port });
    process.stdout.write(JSON.stringify(result));
  } finally {
    await closeHttpServer(http.server);
  }
}

main().catch((error) => {
  process.stderr.write(`${error?.stack || error}\n`);
  process.exitCode = 1;
});
