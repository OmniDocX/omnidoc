/**
 * Browser-side video capture for presentations.
 *
 * Records the live presenter through `getDisplayMedia` + `MediaRecorder`, so a
 * deck can be turned into a video with nothing installed on the server. It
 * complements, rather than replaces, the offline Chromium + FFmpeg pipeline:
 * this path captures in real time and depends on the viewer's screen, while the
 * server path re-renders every frame against a virtual clock and is exact.
 */
(function installUniPptScreenRecorder(root) {
  "use strict";

  /**
   * Prefer Chromium's mature WebM screen-capture path. Fragmented MP4 display
   * capture is still offered as a fallback, but must not be the default: some
   * Chromium builds advertise H.264 support and then emit decodable first
   * fragments followed by corrupt prediction frames when a timeslice is used.
   */
  const MIME_CANDIDATES = [
    { mimeType: "video/webm;codecs=vp9,opus", extension: "webm", label: "WebM · VP9 + Opus" },
    { mimeType: "video/webm;codecs=vp8,opus", extension: "webm", label: "WebM · VP8 + Opus" },
    { mimeType: "video/webm", extension: "webm", label: "WebM" },
    { mimeType: "video/mp4;codecs=avc1.42E01E,mp4a.40.2", extension: "mp4", label: "MP4 · H.264 + AAC（兼容模式）" },
    { mimeType: "video/mp4;codecs=avc1.42E01E", extension: "mp4", label: "MP4 · H.264（兼容模式）" },
    { mimeType: "video/mp4", extension: "mp4", label: "MP4（兼容模式）" },
  ];

  const SAFETY_LIMIT_MS = 2 * 60 * 60 * 1000;

  function now() {
    return root.performance?.now?.() ?? Date.now();
  }

  function supported() {
    return Boolean(
      root.MediaRecorder
      && root.navigator?.mediaDevices?.getDisplayMedia
      && root.MediaRecorder.isTypeSupported,
    );
  }

  function supportedFormats() {
    if (!supported()) return [];
    return MIME_CANDIDATES.filter((candidate) => {
      try {
        return root.MediaRecorder.isTypeSupported(candidate.mimeType);
      } catch (_) {
        return false;
      }
    });
  }

  function preferredFormat() {
    return supportedFormats()[0] || null;
  }

  /**
   * Screen content compresses far better than camera footage, so a modest
   * bits-per-pixel budget still looks clean. Clamped so a 4K deck does not
   * produce an unusable file and a 720p deck is not starved.
   */
  function bitrateFor(width, height, fps) {
    const pixels = Math.max(1, (Number(width) || 1920) * (Number(height) || 1080));
    const rate = Math.max(1, Number(fps) || 30);
    return Math.round(Math.min(40e6, Math.max(2.5e6, pixels * rate * 0.12)));
  }

  async function requestDisplayStream(options = {}) {
    const width = Number(options.width) || 1920;
    const height = Number(options.height) || 1080;
    const fps = Number(options.fps) || 30;
    return root.navigator.mediaDevices.getDisplayMedia({
      video: {
        // Do not silently capture a 60 FPS track for a 30 FPS export. Some
        // native players fall back to the declared frame cadence when WebM
        // duration metadata is sparse, which makes that mismatch look like a
        // 2× playback. Media timestamps and requested cadence now agree.
        frameRate: { ideal: fps, max: fps },
        width: { ideal: width },
        height: { ideal: height },
      },
      audio: options.audio === false ? false : { echoCancellation: false, noiseSuppression: false },
      // Chrome-only hints that steer the picker toward this tab. Other engines
      // ignore unknown dictionary members, so no feature test is needed.
      preferCurrentTab: true,
      selfBrowserSurface: "include",
      surfaceSwitching: "exclude",
      systemAudio: "exclude",
    });
  }

  function stopStream(stream) {
    for (const track of stream?.getTracks?.() || []) {
      try { track.stop(); } catch (_) { /* already ended */ }
    }
  }

  /**
   * Captures for the lifetime of `play`.
   *
   * `play` receives a controller and should resolve once the presentation has
   * finished. Recording keeps working when the viewer ends the share early: the
   * partial take is still returned rather than discarded.
   */
  async function record(options = {}) {
    if (!supported()) throw new Error("当前浏览器不支持屏幕录制");
    const format = options.format || preferredFormat();
    if (!format) throw new Error("当前浏览器没有可用的视频编码器");

    const stream = await requestDisplayStream(options);
    const chunks = [];
    let recorder = null;
    let endedByUser = false;
    try {
      const [videoTrack] = stream.getVideoTracks();
      const settings = videoTrack?.getSettings?.() || {};
      recorder = new root.MediaRecorder(stream, {
        mimeType: format.mimeType,
        videoBitsPerSecond: bitrateFor(
          settings.width || options.width,
          settings.height || options.height,
          settings.frameRate || options.fps,
        ),
      });
      recorder.ondataavailable = (event) => {
        if (event.data?.size) chunks.push(event.data);
      };
      const stopped = new Promise((resolve, reject) => {
        recorder.onstop = resolve;
        recorder.onerror = (event) => reject(event.error || new Error("录制失败"));
      });
      // Ending the share from the browser's own bar must finish the take
      // cleanly instead of leaving the presenter running against a dead track.
      if (videoTrack) {
        videoTrack.addEventListener("ended", () => {
          endedByUser = true;
          options.onShareEnded?.();
        }, { once: true });
      }

      const session = {
        get shareEnded() { return endedByUser; },
        get width() { return settings.width || null; },
        get height() { return settings.height || null; },
        get frameRate() { return settings.frameRate || null; },
      };

      // Prepare the presenter before the encoder starts. Starting first records
      // the editor chrome and picker teardown into the opening frames.
      await options.prepare?.(session);
      if (endedByUser) {
        const error = new Error("屏幕共享已停止");
        error.name = "AbortError";
        throw error;
      }

      const startedAt = now();
      // WebM supports incremental clusters reliably. Chromium's fragmented
      // MP4 screen recorder has produced corrupt H.264 continuation fragments
      // in the wild, so its compatibility fallback is finalized as one blob.
      if (format.extension === "webm") recorder.start(1000);
      else recorder.start();
      options.onStart?.({ format, settings });

      const safety = setTimeout(() => {
        if (recorder.state !== "inactive") recorder.stop();
      }, SAFETY_LIMIT_MS);
      try {
        await options.play?.(session);
      } finally {
        clearTimeout(safety);
        if (recorder.state !== "inactive") recorder.stop();
        await stopped;
      }

      const blob = new Blob(chunks, { type: format.mimeType.split(";")[0] });
      return {
        blob,
        format,
        endedByUser,
        durationMs: now() - startedAt,
        width: settings.width || null,
        height: settings.height || null,
        frameRate: settings.frameRate || null,
      };
    } finally {
      stopStream(stream);
    }
  }

  root.UniPptScreenRecorder = Object.freeze({
    supported,
    supportedFormats,
    preferredFormat,
    bitrateFor,
    record,
  });
})(globalThis);
