let restoringPlayerState = false;
let playerVolume = 1;
let playerMuted = false;
let playerControlsTimer = 0;
let playerAutoPlayTimer = 0;
let playerBoundaryTimer = 0;
let playerAutoPlay = false;
let playerZoom = 1;
let playerActivated = Boolean(globalThis.navigator?.userActivation?.hasBeenActive);
let pendingPlayerAutomaticStartOffset = null;
let captureStopAtPageEnd = false;
let captureSegmentComplete = false;

const PLAYER_MIN_ZOOM = 0.1;
const PLAYER_MAX_ZOOM = 5;
const PLAYER_ZOOM_STEP = 0.1;

function clampPlayerZoom(value) {
  return Math.max(PLAYER_MIN_ZOOM, Math.min(PLAYER_MAX_ZOOM, Number(value) || 1));
}

function syncPlayerZoom() {
  const value = document.getElementById("playerZoomValue");
  const out = document.getElementById("playerZoomOut");
  const input = document.getElementById("playerZoomIn");
  const percentage = Math.round(playerZoom * 100);
  if (value) {
    value.textContent = `${percentage}%`;
    value.setAttribute("aria-label", `放映缩放 ${percentage}%`);
  }
  if (out) out.disabled = playerZoom <= PLAYER_MIN_ZOOM + 0.0001;
  if (input) input.disabled = playerZoom >= PLAYER_MAX_ZOOM - 0.0001;
}

function setPlayerZoom(value) {
  playerZoom = clampPlayerZoom(value);
  scale();
  syncPlayerZoom();
}

function adjustPlayerZoom(delta) {
  setPlayerZoom(Math.round((playerZoom + delta) * 10) / 10);
}

function clearNativePlayerSelection() {
  try { globalThis.getSelection?.()?.removeAllRanges?.(); } catch (_) {}
}

function isPlayerInteractiveTarget(target) {
  return Boolean(target?.closest?.("#playerControls,a[href],button,input,select,textarea,[contenteditable=true]"));
}

function installLosslessBrowserGuards() {
  document.addEventListener("contextmenu", (event) => event.preventDefault(), true);
  document.addEventListener("auxclick", (event) => event.preventDefault(), true);
  document.addEventListener("dragstart", (event) => event.preventDefault(), true);
  document.addEventListener("selectstart", (event) => {
    if (!isPlayerInteractiveTarget(event.target)) event.preventDefault();
  }, true);
  document.addEventListener("pointerdown", (event) => {
    if (event.detail > 1 && !isPlayerInteractiveTarget(event.target)) {
      event.preventDefault();
      clearNativePlayerSelection();
    }
  }, true);
  document.addEventListener("dblclick", (event) => {
    if (!isPlayerInteractiveTarget(event.target)) event.preventDefault();
    clearNativePlayerSelection();
  }, true);
  for (const type of ["gesturestart", "gesturechange", "gestureend"]) {
    document.addEventListener(type, (event) => event.preventDefault(), {
      capture: true,
      passive: false,
    });
  }
  document.addEventListener("touchmove", (event) => {
    if (event.touches?.length > 1) event.preventDefault();
  }, { capture: true, passive: false });
  document.addEventListener("wheel", (event) => {
    if (!(event.ctrlKey || event.metaKey)) return;
    event.preventDefault();
    if (event.deltaY !== 0) adjustPlayerZoom(event.deltaY < 0 ? PLAYER_ZOOM_STEP : -PLAYER_ZOOM_STEP);
    hideLosslessPlayerControls(true);
  }, { capture: true, passive: false });
}

function activateLosslessPlayer() {
  if (playerActivated) return false;
  playerActivated = true;
  const startOffset = pendingPlayerAutomaticStartOffset;
  pendingPlayerAutomaticStartOffset = null;
  if (startOffset == null) return false;
  if (typeof playerAutoPlay !== "undefined" && playerAutoPlay) scheduleLosslessAutoPlay(startOffset);
  else startAutomaticBatch(startOffset);
  return true;
}

function runLosslessPlayerAction(action) {
  if (activateLosslessPlayer()) return false;
  action();
  if (typeof playerAutoPlay !== "undefined" && playerAutoPlay) scheduleLosslessAutoPlay(80);
  return true;
}

function playerMediaNodes() {
  return [...document.querySelectorAll(
    "audio.native-media,video.native-media,audio.media,video.media",
  )];
}

function applyPlayerMediaVolume(node) {
  if (!node.dataset.unipptBaseVolume) {
    node.dataset.unipptBaseVolume = String(Math.max(0, Math.min(1, Number(node.volume) || 0)));
  }
  const base = Math.max(0, Math.min(1, Number(node.dataset.unipptBaseVolume) || 0));
  node.volume = Math.max(0, Math.min(1, base * playerVolume));
  node.muted = playerMuted || playerVolume === 0;
}

function syncPlayerAudio() {
  playerMediaNodes().forEach(applyPlayerMediaVolume);
  const button = document.getElementById("playerMute");
  const slider = document.getElementById("playerVolume");
  const glyph = document.getElementById("playerVolumeGlyph");
  const silent = playerMuted || playerVolume === 0;
  if (button) {
    button.setAttribute("aria-pressed", String(playerMuted));
    button.title = silent ? "取消静音（M）" : "静音（M）";
  }
  if (glyph) glyph.textContent = silent ? "🔇" : playerVolume < 0.5 ? "🔉" : "🔊";
  if (slider) {
    slider.value = String(Math.round(playerVolume * 100));
    slider.setAttribute(
      "aria-valuetext",
      `${Math.round(playerVolume * 100)}%${playerMuted ? "，已静音" : ""}`,
    );
  }
}

function setPlayerVolume(value) {
  playerVolume = Math.max(0, Math.min(1, Number(value) || 0));
  if (playerVolume > 0) playerMuted = false;
  syncPlayerAudio();
}

function togglePlayerMute() {
  playerMuted = !playerMuted;
  syncPlayerAudio();
}

function playerStepBoundaries(es = effects()) {
  return [0, ...animationNavigationRanges(es).map((range) => range.endIndex)];
}

function applyPlayerAnimationFinalState(animation, runMedia = false) {
  if (animation.effect === "media" || animation.class === "media") {
    if (runMedia) {
      const owner = s.querySelector(
        '[data-id="' + CSS.escape(animation.targetObjectId || "") + '"]',
      );
      if (owner) control(owner, animation.mediaAction || "play");
    }
    return;
  }
  const element = s.querySelector(
    '[data-id="' + CSS.escape(animation.targetObjectId || "") + '"]',
  );
  if (!element) return;
  const keyframes = losslessAnimationFrames(animation, element) || [];
  const returnsToStart = Boolean(animation.autoReverse) || Number(animation.speed) < 0;
  const finalFrame = keyframes[returnsToStart ? 0 : keyframes.length - 1] || {};
  for (const [name, value] of Object.entries(finalFrame)) {
    if (name !== "offset" && name !== "easing" && name !== "composite") {
      element.style[name] = value;
    }
  }
  if (animation.class === "exit" && !returnsToStart) {
    element.style.visibility = "hidden";
    element.style.opacity = "0";
  } else {
    element.style.visibility = "visible";
    element.style.opacity = "1";
  }
}

function seekPlayerCursor(target, runMedia = false) {
  const es = effects();
  const limit = Math.max(0, Math.min(es.length, Number(target) || 0));
  restoringPlayerState = true;
  draw(false);
  restoringPlayerState = false;
  cursor = 0;
  for (const animation of es.slice(0, limit)) applyPlayerAnimationFinalState(animation, runMedia);
  cursor = limit;
  status();
  syncPlayerAudio();
}

function previousPlayerStep() {
  hideLosslessEndNotice();
  if (cancelAnimationBatch()) return;
  const previous = previousAnimationNavigationCursor(effects(), cursor);
  if (previous != null) {
    seekPlayerCursor(previous);
    return;
  }
  if (page > 0) {
    page -= 1;
    restoringPlayerState = true;
    draw(true, { reverse: true });
    restoringPlayerState = false;
    seekPlayerCursor(effects().length);
  }
}

function jumpPlayerPage(delta) {
  hideLosslessEndNotice();
  const nextPage = Math.max(0, Math.min(d.slides.length - 1, page + delta));
  if (nextPage === page) {
    if (delta > 0) showLosslessEndNotice();
    return;
  }
  page = nextPage;
  draw(true, { reverse: delta < 0 });
  if (delta < 0) seekPlayerCursor(effects().length);
}

function playerPageComplete() {
  return animationTargetCursor == null && cursor >= effects().length;
}

function skipPlayerToPageEnd(direction) {
  hideLosslessEndNotice();
  if (!playerPageComplete()) {
    seekPlayerCursor(effects().length, true);
    return;
  }
  const nextPage = Math.max(0, Math.min(d.slides.length - 1, page + direction));
  if (nextPage === page) {
    if (direction > 0) showLosslessEndNotice();
    return;
  }
  page = nextPage;
  draw(true, { reverse: direction < 0 });
  seekPlayerCursor(effects().length, true);
}

function signalLosslessCaptureComplete() {
  globalThis.__unipptCaptureComplete = true;
  globalThis.dispatchEvent?.(new CustomEvent("unippt:playback-complete", {
    detail: { page, slideCount: d.slides.length },
  }));
}

function ensureLosslessEndNotice() {
  let notice = document.getElementById("playerBoundaryNotice");
  if (notice) return notice;
  if (!document.getElementById("playerBoundaryNoticeStyle")) {
    const style = document.createElement("style");
    style.id = "playerBoundaryNoticeStyle";
    style.textContent = `
      .boundary-notice{position:fixed;left:50%;bottom:clamp(76px,10vh,118px);z-index:1001;display:flex;align-items:center;gap:11px;min-width:230px;padding:10px 17px 10px 11px;transform:translate3d(-50%,18px,0) scale(.94);border:1px solid #ffffff38;border-radius:999px;background:linear-gradient(135deg,#202329e8,#101216e8);color:#fff;box-shadow:0 16px 42px #0008,inset 0 1px #ffffff20;backdrop-filter:blur(18px) saturate(1.2);opacity:0;pointer-events:none;will-change:transform,opacity}
      .boundary-notice[hidden]{display:none}.boundary-notice.visible{animation:boundary-arrive 1.85s cubic-bezier(.2,.8,.2,1) both}
      .boundary-icon{display:grid;place-items:center;flex:0 0 34px;width:34px;height:34px;border-radius:50%;background:linear-gradient(145deg,#e95b35,#b92e12);box-shadow:0 5px 14px #a82e1c66,inset 0 1px #ffffff55;color:#fff;font:700 18px/1 "Segoe UI Symbol","Segoe UI",sans-serif}
      .boundary-copy{display:grid;gap:3px;min-width:0;white-space:nowrap}.boundary-copy strong{font-size:13px;line-height:1.1;font-weight:650;letter-spacing:.02em}.boundary-copy small{color:#d7d9de;font-size:10px;line-height:1.1}
      @keyframes boundary-arrive{0%{transform:translate3d(-50%,18px,0) scale(.94);opacity:0}12%,72%{transform:translate3d(-50%,0,0) scale(1);opacity:1}100%{transform:translate3d(-50%,-5px,0) scale(.98);opacity:0}}
    `;
    document.head.append(style);
  }
  notice = document.createElement("div");
  notice.id = "playerBoundaryNotice";
  notice.className = "boundary-notice";
  notice.setAttribute("role", "status");
  notice.setAttribute("aria-live", "polite");
  notice.setAttribute("aria-atomic", "true");
  notice.hidden = true;
  const icon = document.createElement("span");
  icon.className = "boundary-icon";
  icon.setAttribute("aria-hidden", "true");
  icon.textContent = "✓";
  const copy = document.createElement("span");
  copy.className = "boundary-copy";
  const title = document.createElement("strong");
  title.textContent = "已经到底了";
  const hint = document.createElement("small");
  hint.textContent = "最后一张 · 按 Esc 退出放映";
  copy.append(title, hint);
  notice.append(icon, copy);
  document.body.append(notice);
  return notice;
}

function hideLosslessEndNotice() {
  clearTimeout(playerBoundaryTimer);
  playerBoundaryTimer = 0;
  const notice = document.getElementById("playerBoundaryNotice");
  if (!notice) return;
  notice.classList.remove("visible");
  notice.hidden = true;
}

function showLosslessEndNotice() {
  if (globalThis.__UNIPPT_CAPTURE_MODE__) return;
  const notice = ensureLosslessEndNotice();
  clearTimeout(playerBoundaryTimer);
  notice.hidden = false;
  notice.classList.remove("visible");
  void notice.offsetWidth;
  notice.classList.add("visible");
  playerBoundaryTimer = setTimeout(() => {
    notice.classList.remove("visible");
    notice.hidden = true;
    playerBoundaryTimer = 0;
  }, 1900);
}

function closeLosslessPlayer(reason = "user") {
  hideLosslessEndNotice();
  clearTimeout(playerAutoPlayTimer);
  playerAutoPlayTimer = 0;
  playerAutoPlay = false;
  globalThis.UniPptMedia?.stopAll(s, true, false);
  stopPersistentMedia();
  if (reason === "complete") signalLosslessCaptureComplete();
  if (globalThis.__UNIPPT_CAPTURE_MODE__) return;
  if (document.fullscreenElement) {
    document.exitFullscreen?.();
    return;
  }
  window.close();
  setTimeout(() => {
    if (!window.closed) window.location.replace("about:blank");
  }, 0);
}

function updateLosslessAutoPlayControl() {
  const button = document.getElementById("playerAutoPlay");
  if (!button) return;
  button.classList.toggle("active", playerAutoPlay);
  button.setAttribute("aria-pressed", String(playerAutoPlay));
  button.title = playerAutoPlay
    ? "停止自动放映（当前动画仍会完成，A）"
    : "自动放映：本页全部动画完成后自动换页（A）";
}

function scheduleLosslessAutoPlay(delay = 0) {
  clearTimeout(playerAutoPlayTimer);
  playerAutoPlayTimer = 0;
  if (!playerAutoPlay) return;
  if (!playerActivated) {
    pendingPlayerAutomaticStartOffset = Math.max(0, Number(delay) || 0);
    return;
  }
  const token = epoch;
  playerAutoPlayTimer = setTimeout(() => {
    playerAutoPlayTimer = 0;
    if (!playerAutoPlay || token !== epoch) return;
    if (animationTargetCursor != null) {
      scheduleLosslessAutoPlay(40);
      return;
    }
    const es = effects();
    if (cursor >= es.length) {
      if (captureStopAtPageEnd) {
        playerAutoPlay = false;
        captureSegmentComplete = true;
        globalThis.dispatchEvent?.(new CustomEvent("unippt:slide-playback-complete", {
          detail: { page, slideCount: d.slides.length },
        }));
        return;
      }
      if (page >= d.slides.length - 1) {
        closeLosslessPlayer("complete");
        return;
      }
      page += 1;
      draw(true);
      return;
    }
    const schedule = runAnimationBatch(es);
    scheduleLosslessAutoPlay(Math.max(1, Number(schedule?.end) || 0) + 80);
  }, Math.max(0, Number(delay) || 0));
}

function setLosslessAutoPlay(enabled) {
  playerAutoPlay = Boolean(enabled);
  updateLosslessAutoPlayControl();
  if (!playerAutoPlay) {
    clearTimeout(playerAutoPlayTimer);
    playerAutoPlayTimer = 0;
    return;
  }
  if (!playerActivated) activateLosslessPlayer();
  if (!playerAutoPlayTimer) scheduleLosslessAutoPlay(40);
}

function toggleLosslessAutoPlay() {
  setLosslessAutoPlay(!playerAutoPlay);
}

async function settleLosslessCaptureFrame() {
  try { await document.fonts?.ready; } catch (_) {}
  const media = [...s.querySelectorAll("img,video")];
  await Promise.all(media.map(async (node) => {
    try {
      if (node.tagName === "IMG") await node.decode?.();
      else if (node.readyState < 2) await new Promise((resolve) => {
        const done = () => resolve();
        node.addEventListener("loadeddata", done, { once: true });
        node.addEventListener("error", done, { once: true });
        setTimeout(done, 2000);
      });
    } catch (_) {}
  }));
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
}

function installLosslessCaptureBridge() {
  globalThis.UniPptCapture = Object.freeze({
    version: 1,
    get slideCount() { return d.slides.length; },
    get width() { return d.width; },
    get height() { return d.height; },
    get completed() { return Boolean(globalThis.__unipptCaptureComplete); },
    get segmentCompleted() { return captureSegmentComplete; },
    async ready() {
      await settleLosslessCaptureFrame();
      return { slideCount: d.slides.length, width: d.width, height: d.height };
    },
    async showSlide(index, options = {}) {
      captureStopAtPageEnd = false;
      captureSegmentComplete = false;
      setLosslessAutoPlay(false);
      restoringPlayerState = true;
      page = Math.max(0, Math.min(d.slides.length - 1, Number(index) || 0));
      draw(false);
      restoringPlayerState = false;
      if (options.final !== false) seekPlayerCursor(effects().length);
      hideLosslessPlayerControls(true);
      await settleLosslessCaptureFrame();
      return { page, animations: effects().length };
    },
    async startPlayback(options = {}) {
      captureStopAtPageEnd = false;
      captureSegmentComplete = false;
      globalThis.__unipptCaptureComplete = false;
      playerActivated = true;
      pendingPlayerAutomaticStartOffset = null;
      page = Math.max(0, Math.min(d.slides.length - 1, Number(options.page) || 0));
      cursor = 0;
      playerAutoPlay = true;
      updateLosslessAutoPlayControl();
      draw(false);
      hideLosslessPlayerControls(true);
      if (options.settle === false) return { page, slideCount: d.slides.length };
      await settleLosslessCaptureFrame();
      return { page, slideCount: d.slides.length };
    },
    async startSlidePlayback(index, options = {}) {
      await this.prepareSlidePlayback(index);
      return this.startPreparedSlidePlayback(index, options);
    },
    async prepareSlidePlayback(index) {
      globalThis.__unipptCaptureComplete = false;
      captureStopAtPageEnd = true;
      captureSegmentComplete = false;
      playerActivated = true;
      pendingPlayerAutomaticStartOffset = null;
      playerAutoPlay = false;
      const target = Math.max(0, Math.min(d.slides.length - 1, Number(index) || 0));
      if (target > 0) {
        restoringPlayerState = true;
        page = target - 1;
        draw(false);
        restoringPlayerState = false;
        seekPlayerCursor(effects().length);
      } else {
        restoringPlayerState = true;
        page = target;
        draw(false);
        restoringPlayerState = false;
      }
      hideLosslessPlayerControls(true);
      await settleLosslessCaptureFrame();
      return { page, target, slideCount: d.slides.length };
    },
    async startPreparedSlidePlayback(index, options = {}) {
      captureStopAtPageEnd = true;
      captureSegmentComplete = false;
      playerActivated = true;
      pendingPlayerAutomaticStartOffset = null;
      const target = Math.max(0, Math.min(d.slides.length - 1, Number(index) || 0));
      page = target;
      cursor = 0;
      playerAutoPlay = true;
      updateLosslessAutoPlayControl();
      draw(target > 0);
      hideLosslessPlayerControls(true);
      if (options.settle === false) return { page, slideCount: d.slides.length };
      await settleLosslessCaptureFrame();
      return { page, slideCount: d.slides.length };
    },
    async renderPrintPages() {
      setLosslessAutoPlay(false);
      clear();
      const host = document.createElement("main");
      host.id = "unipptPrintPages";
      for (let index = 0; index < d.slides.length; index += 1) {
        const slide = d.slides[index];
        const stage = document.createElement("section");
        stage.className = "unippt-print-slide";
        Object.assign(stage.style, {
          position: "relative",
          width: `${d.width}px`,
          height: `${d.height}px`,
          overflow: "hidden",
          boxSizing: "border-box",
          background: slide.background || "#fff",
          backgroundImage: slide.backgroundAsset ? `url(${JSON.stringify(slide.backgroundAsset)})` : "none",
          backgroundSize: "100% 100%",
          backgroundPosition: "center",
          backgroundRepeat: "no-repeat",
          breakAfter: index === d.slides.length - 1 ? "auto" : "page",
          pageBreakAfter: index === d.slides.length - 1 ? "auto" : "always",
        });
        globalThis.UniPptPresentationScene.renderSlide(slide, stage, {
          clear: false,
          applyBackground: false,
          deckWidth: d.width,
          deckHeight: d.height,
          dynamicObjects: d.extensions?.["org.unippt.dynamic"]?.objects || {},
          mediaContext: {
            scope: "presentation-print",
            slideKey: slide.sourcePartName || slide.id || String(index),
          },
          onHyperlink: () => {},
        });
        host.append(stage);
      }
      document.body.replaceChildren(host);
      Object.assign(document.documentElement.style, { margin: "0", padding: "0", background: "#fff" });
      Object.assign(document.body.style, { margin: "0", padding: "0", background: "#fff", overflow: "visible" });
      try { await document.fonts?.ready; } catch (_) {}
      await Promise.all([...host.querySelectorAll("img")].map(async (image) => {
        try { await image.decode?.(); } catch (_) {}
      }));
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      return { slideCount: d.slides.length, width: d.width, height: d.height };
    },
    stop() {
      closeLosslessPlayer("capture-stop");
    },
  });
}

function showLosslessPlayerControls(pinned = false) {
  const nav = document.getElementById("playerControls");
  if (!nav) return;
  nav.classList.add("visible");
  clearTimeout(playerControlsTimer);
  if (!pinned) {
    playerControlsTimer = setTimeout(() => {
      if (!nav.matches(":hover,:focus-within")) nav.classList.remove("visible");
    }, 2200);
  }
}

function hideLosslessPlayerControls(force = false) {
  const nav = document.getElementById("playerControls");
  if (!nav) return;
  clearTimeout(playerControlsTimer);
  playerControlsTimer = 0;
  if (!force && nav.matches(":hover,:focus-within")) return;
  nav.classList.remove("visible");
}

function scheduleLosslessPlayerControlsDismiss(delay = 900) {
  const nav = document.getElementById("playerControls");
  if (!nav) return;
  clearTimeout(playerControlsTimer);
  playerControlsTimer = setTimeout(() => {
    nav.classList.remove("visible");
    playerControlsTimer = 0;
  }, Math.max(0, Number(delay) || 0));
}

function settleLosslessPlayerControls(event, pinned = false) {
  const pointerActivated = Number(event?.detail) > 0;
  if (pointerActivated) {
    event?.currentTarget?.blur?.();
    showLosslessPlayerControls(true);
    // A clicked navigation button remains under the pointer. PowerPoint still
    // dismisses its floating controls after the action, so this timer must not
    // be held open by :hover or :focus-within.
    scheduleLosslessPlayerControlsDismiss();
    return;
  }
  showLosslessPlayerControls(pinned);
}

function installLosslessPlayerControls() {
  const controls = document.getElementById("playerControls");
  const prevPage = document.getElementById("prevSlide");
  const nextPage = document.getElementById("nextSlide");
  const autoPlay = document.getElementById("playerAutoPlay");
  const mute = document.getElementById("playerMute");
  const volume = document.getElementById("playerVolume");
  const zoomOut = document.getElementById("playerZoomOut");
  const zoomValue = document.getElementById("playerZoomValue");
  const zoomIn = document.getElementById("playerZoomIn");
  const close = document.getElementById("playerClose");

  installLosslessBrowserGuards();

  prev.onclick = (event) => {
    event.stopPropagation();
    runLosslessPlayerAction(previousPlayerStep);
    settleLosslessPlayerControls(event);
  };
  next.onclick = (event) => {
    event.stopPropagation();
    runLosslessPlayerAction(advance);
    settleLosslessPlayerControls(event);
  };
  prevPage.onclick = (event) => {
    event.stopPropagation();
    runLosslessPlayerAction(() => jumpPlayerPage(-1));
    settleLosslessPlayerControls(event);
  };
  nextPage.onclick = (event) => {
    event.stopPropagation();
    runLosslessPlayerAction(() => jumpPlayerPage(1));
    settleLosslessPlayerControls(event);
  };
  autoPlay.onclick = (event) => {
    event.stopPropagation();
    toggleLosslessAutoPlay();
    settleLosslessPlayerControls(event, true);
  };
  mute.onclick = (event) => {
    event.stopPropagation();
    togglePlayerMute();
    activateLosslessPlayer();
    settleLosslessPlayerControls(event, true);
  };
  volume.oninput = (event) => {
    event.stopPropagation();
    setPlayerVolume(Number(event.target.value) / 100);
    activateLosslessPlayer();
    settleLosslessPlayerControls(event, true);
  };
  volume.addEventListener("pointerup", () => {
    volume.blur();
    scheduleLosslessPlayerControlsDismiss();
  });
  zoomOut.onclick = (event) => {
    event.stopPropagation();
    adjustPlayerZoom(-PLAYER_ZOOM_STEP);
    settleLosslessPlayerControls(event);
  };
  zoomValue.onclick = (event) => {
    event.stopPropagation();
    setPlayerZoom(1);
    settleLosslessPlayerControls(event);
  };
  zoomIn.onclick = (event) => {
    event.stopPropagation();
    adjustPlayerZoom(PLAYER_ZOOM_STEP);
    settleLosslessPlayerControls(event);
  };
  close.onclick = (event) => {
    event.stopPropagation();
    closeLosslessPlayer();
  };
  controls.addEventListener("pointerdown", (event) => event.stopPropagation());
  controls.addEventListener("click", (event) => event.stopPropagation());
  controls.addEventListener("pointerenter", () => showLosslessPlayerControls(true));
  controls.addEventListener("pointerleave", () => hideLosslessPlayerControls());

  document.addEventListener("click", (event) => {
    if (event.target?.closest?.("#playerControls")) return;
    if (!activateLosslessPlayer()) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  }, true);
  s.onclick = () => runLosslessPlayerAction(advance);
  document.addEventListener("pointermove", (event) => {
    const nearBottom = innerHeight - event.clientY < 112;
    if (nearBottom) showLosslessPlayerControls(true);
    else hideLosslessPlayerControls();
  });
  document.addEventListener("pointerleave", () => hideLosslessPlayerControls(true));
  document.addEventListener("keydown", (event) => {
    const focusedControl = event.target?.closest?.("#playerControls");
    const nativeControlKeys = [
      "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown",
      "PageUp", "PageDown", "Home", "End", " ", "Enter",
    ];
    if (!event.ctrlKey && !event.metaKey && focusedControl && nativeControlKeys.includes(event.key)) {
      showLosslessPlayerControls(true);
      return;
    }
    const browserZoomShortcut = (event.ctrlKey || event.metaKey)
      && ["+", "=", "-", "_", "0"].includes(event.key);
    if (browserZoomShortcut) {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "0") setPlayerZoom(1);
      else adjustPlayerZoom(["+", "="].includes(event.key) ? PLAYER_ZOOM_STEP : -PLAYER_ZOOM_STEP);
    } else if ((event.ctrlKey || event.metaKey)
      && ["a", "f", "g", "h", "j", "l", "n", "o", "p", "r", "s", "t", "u", "w"].includes(event.key.toLowerCase())) {
      event.preventDefault();
      event.stopPropagation();
    } else if (event.altKey && ["ArrowLeft", "ArrowRight", "Home"].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
    } else if (["F1", "F3", "F5", "F7", "F12"].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
    } else if (event.ctrlKey && event.key === "Home") {
      event.preventDefault();
      runLosslessPlayerAction(() => skipPlayerToPageEnd(-1));
    } else if (event.ctrlKey && event.key === "End") {
      event.preventDefault();
      runLosslessPlayerAction(() => skipPlayerToPageEnd(1));
    } else if (!event.ctrlKey && event.key === "Home") {
      event.preventDefault();
      runLosslessPlayerAction(() => previousPlayerStep());
    } else if (!event.ctrlKey && event.key === "End") {
      event.preventDefault();
      runLosslessPlayerAction(() => advance());
    } else if (["ArrowRight", "ArrowDown", "PageDown", " ", "Enter"].includes(event.key)) {
      event.preventDefault();
      runLosslessPlayerAction(advance);
    } else if (["ArrowLeft", "ArrowUp", "PageUp"].includes(event.key)) {
      event.preventDefault();
      runLosslessPlayerAction(previousPlayerStep);
    } else if (!event.ctrlKey && !event.metaKey && !event.altKey && event.key.toLowerCase() === "m") {
      event.preventDefault();
      togglePlayerMute();
      activateLosslessPlayer();
    } else if (!event.ctrlKey && !event.metaKey && !event.altKey && event.key.toLowerCase() === "a") {
      event.preventDefault();
      toggleLosslessAutoPlay();
    } else if (event.key === "Escape") {
      event.preventDefault();
      closeLosslessPlayer();
    } else {
      return;
    }
    // Keyboard navigation must never cover the slide with the floating UI.
    // Keep the toolbar visible only when the keyboard event originated from
    // an actual control (handled by the focusedControl branch above).
    hideLosslessPlayerControls(true);
  });

  new MutationObserver((records) => {
    for (const record of records) {
      for (const node of record.addedNodes) {
        if (!(node instanceof Element)) continue;
        if (node.matches?.("audio.native-media,video.native-media,audio.media,video.media")) {
          applyPlayerMediaVolume(node);
        }
        node.querySelectorAll?.(
          "audio.native-media,video.native-media,audio.media,video.media",
        ).forEach(applyPlayerMediaVolume);
      }
    }
  }).observe(document.body, { childList: true, subtree: true });

  syncPlayerAudio();
  syncPlayerZoom();
  updateLosslessAutoPlayControl();
  installLosslessCaptureBridge();
  hideLosslessPlayerControls(true);
}
