(() => {
  "use strict";

  const clamp = (value, min, max) => Math.max(min, Math.min(max, Number(value)));
  const seconds = (milliseconds) => Math.max(0, Number(milliseconds) || 0) / 1000;
  const persistentHostId = "unippt-persistent-media";
  const activeAcrossByOwner = new Map();
  const activeAcrossBySource = new Map();
  const mediaMetadata = new WeakMap();
  const mediaOwners = new WeakMap();

  function persistentHost(create = true) {
    let host = document.getElementById(persistentHostId);
    if (!host && create) {
      host = document.createElement("div");
      host.id = persistentHostId;
      host.hidden = true;
      host.setAttribute("aria-hidden", "true");
      document.body.append(host);
    }
    return host;
  }

  function normalizeObject(object) {
    if (!object?.media) return;
    const media = object.media;
    media.kind ||= "audio";
    media.asset ??= null;
    media.mimeType ??= null;
    media.playbackAsset ??= null;
    media.playbackMimeType ??= null;
    media.sourcePartName ??= null;
    media.relationshipId ??= null;
    media.legacyRelationshipId ??= null;
    media.trimStartMs = media.trimStartMs == null ? null : Math.max(0, Number(media.trimStartMs) || 0);
    // OOXML p14:trim/@end is the amount removed from the end, not an
    // absolute playback timestamp. Keep the native value loss-aware here.
    media.trimEndMs = media.trimEndMs == null ? null : Math.max(0, Number(media.trimEndMs) || 0);
    media.volume = clamp(media.volume ?? 1, 0, 1);
    media.loopPlayback = Boolean(media.loopPlayback);
    media.playAcrossSlides = Boolean(media.playAcrossSlides);
    media.showWhenStopped = media.showWhenStopped !== false;
  }

  function identityPart(value) {
    return String(value ?? "").trim().replaceAll("|", "%7C");
  }

  function mediaIdentity(descriptor) {
    if (!descriptor) return "";
    if (descriptor.sourcePartName) return `part:${identityPart(descriptor.sourcePartName)}`;
    if (descriptor.asset) return `asset:${descriptor.asset}`;
    const relationship = descriptor.relationshipId || descriptor.legacyRelationshipId;
    return relationship ? `relationship:${identityPart(relationship)}` : "";
  }

  function mediaOwnerIdentity(descriptor, object, context = {}) {
    const scope = identityPart(context.scope || "default");
    const slide = identityPart(context.slideKey || "slide");
    const relationship = identityPart(
      descriptor.relationshipId
      || descriptor.legacyRelationshipId
      || descriptor.sourcePartName
      || descriptor.asset,
    );
    const shape = identityPart(object?.sourceShapeId ?? object?.id ?? "media");
    return `${scope}|${slide}|${relationship}|${shape}`;
  }

  function sourceScopeIdentity(descriptor, context = {}) {
    const source = mediaIdentity(descriptor);
    return source ? `${identityPart(context.scope || "default")}|${source}` : "";
  }

  function effectivePlaybackEnd(duration, trimEndOffsetMs, trimStartMs = 0) {
    const nativeDuration = Number(duration);
    if (!Number.isFinite(nativeDuration) || nativeDuration <= 0) return null;
    return Math.max(seconds(trimStartMs), nativeDuration - seconds(trimEndOffsetMs));
  }

  function refreshPlaybackEnd(media) {
    const end = effectivePlaybackEnd(
      media.duration,
      (Number(media.dataset.trimEndOffset) || 0) * 1000,
      (Number(media.dataset.trimStart) || 0) * 1000,
    );
    media.dataset.playbackEnd = end == null ? "" : String(end);
    return end;
  }

  function playbackEnd(media) {
    const value = Number(media.dataset.playbackEnd);
    return Number.isFinite(value) && value > 0 ? value : null;
  }

  function seekToStart(media) {
    const start = Number(media.dataset.trimStart) || 0;
    try { media.currentTime = start; } catch (_) { /* metadata may not be loaded yet */ }
  }

  function applyOutput(media) {
    if (!media) return;
    const base = clamp(media.dataset.baseVolume ?? media.volume ?? 1, 0, 1);
    const master = clamp(media.dataset.masterVolume ?? 1, 0, 1);
    media.volume = clamp(base * master, 0, 1);
    media.muted = media.dataset.masterMuted === "1";
  }

  function setOutput(root, volume = 1, muted = false, includePersistent = false) {
    const master = clamp(volume, 0, 1);
    const roots = [];
    if (root) roots.push(root);
    if (includePersistent) {
      const host = persistentHost(false);
      if (host && host !== root) roots.push(host);
    }
    const seen = new Set();
    for (const outputRoot of roots) {
      const mediaNodes = outputRoot.matches?.("audio.native-media,video.native-media")
        ? [outputRoot]
        : [...(outputRoot.querySelectorAll?.("audio.native-media,video.native-media") || [])];
      for (const media of mediaNodes) {
        if (seen.has(media)) continue;
        seen.add(media);
        media.dataset.masterVolume = String(master);
        media.dataset.masterMuted = muted ? "1" : "0";
        applyOutput(media);
      }
    }
  }

  function registerAcross(media, ownerKey, sourceKey) {
    let metadata = mediaMetadata.get(media);
    if (!metadata) {
      metadata = { ownerKeys: new Set(), sourceKey };
      mediaMetadata.set(media, metadata);
    }
    metadata.ownerKeys.add(ownerKey);
    metadata.sourceKey ||= sourceKey;
    activeAcrossByOwner.set(ownerKey, media);
    if (metadata.sourceKey) {
      let entries = activeAcrossBySource.get(metadata.sourceKey);
      if (!entries) activeAcrossBySource.set(metadata.sourceKey, entries = new Set());
      entries.add(media);
    }
  }

  function unregisterAcross(media) {
    const metadata = mediaMetadata.get(media);
    if (!metadata) return;
    for (const ownerKey of metadata.ownerKeys) {
      if (activeAcrossByOwner.get(ownerKey) === media) activeAcrossByOwner.delete(ownerKey);
    }
    const entries = activeAcrossBySource.get(metadata.sourceKey);
    if (entries) {
      entries.delete(media);
      if (!entries.size) activeAcrossBySource.delete(metadata.sourceKey);
    }
    mediaMetadata.delete(media);
    mediaOwners.delete(media);
  }

  function reusableAcross(ownerKey, sourceKey) {
    const exact = activeAcrossByOwner.get(ownerKey);
    if (exact) return exact;
    const entries = activeAcrossBySource.get(sourceKey);
    if (!entries || entries.size !== 1) return null;
    const candidate = entries.values().next().value;
    const parent = candidate?.parentElement || candidate?.parentNode;
    return parent?.id === persistentHostId ? candidate : null;
  }

  function updateOwnerState(media, mode) {
    const node = mediaOwners.get(media);
    if (!node?.classList) return;
    if (mode === "play") {
      node.classList.add("media-playing");
      node.classList.remove("media-ended", "media-play-blocked");
      delete node.dataset.mediaPrompt;
    } else if (mode === "ended") {
      node.classList.remove("media-playing");
      node.classList.add("media-ended");
    } else {
      node.classList.remove("media-playing");
    }
  }

  function configureMedia(media, descriptor, object, reused = false) {
    media.className = `native-media native-media-${descriptor.kind}`;
    const playbackSource = descriptor.playbackAsset || descriptor.asset;
    if (!reused) media.src = playbackSource;
    media.preload = "metadata";
    media.playsInline = true;
    media.controls = false;
    media.dataset.baseVolume = String(descriptor.volume);
    media.dataset.masterVolume ||= "1";
    media.dataset.masterMuted ||= "0";
    media.loop = false; // trim-aware looping is handled below.
    media.dataset.trimStart = String(seconds(descriptor.trimStartMs));
    media.dataset.trimEndOffset = String(seconds(descriptor.trimEndMs));
    media.dataset.loopPlayback = descriptor.loopPlayback ? "1" : "0";
    media.dataset.playAcrossSlides = descriptor.playAcrossSlides ? "1" : "0";
    if (descriptor.kind === "video" && object.asset) media.poster = object.asset;
    if (descriptor.kind === "audio") media.hidden = true;
    refreshPlaybackEnd(media);
    applyOutput(media);
  }

  function wireMediaEvents(media) {
    media.addEventListener("loadedmetadata", () => {
      applyOutput(media);
      refreshPlaybackEnd(media);
      if (media.dataset.initialSeekApplied !== "1") {
        media.dataset.initialSeekApplied = "1";
        if ((Number(media.dataset.trimStart) || 0) > 0) seekToStart(media);
      }
    });
    media.addEventListener("timeupdate", () => {
      const end = playbackEnd(media);
      if (end == null || media.currentTime < end) return;
      if (media.dataset.loopPlayback === "1") {
        seekToStart(media);
        void media.play().catch(() => {});
      } else {
        media.pause();
        updateOwnerState(media, "ended");
      }
    });
    media.addEventListener("play", () => updateOwnerState(media, "play"));
    media.addEventListener("pause", () => updateOwnerState(media, "pause"));
    media.addEventListener("ended", () => {
      if (media.dataset.loopPlayback === "1") {
        seekToStart(media);
        void media.play().catch(() => {});
      } else updateOwnerState(media, "ended");
    });
  }

  function attach(node, object, interactive, context = {}) {
    const descriptor = object?.media;
    if (!descriptor) return null;
    normalizeObject(object);
    if (!descriptor.playbackAsset && !descriptor.asset) return null;

    const ownerKey = mediaOwnerIdentity(descriptor, object, context);
    const sourceKey = sourceScopeIdentity(descriptor, context);
    const useRegistry = descriptor.playAcrossSlides && context.scope === "presentation";
    let media = useRegistry ? reusableAcross(ownerKey, sourceKey) : null;
    const reused = Boolean(media);
    if (!media) {
      media = document.createElement(descriptor.kind === "video" ? "video" : "audio");
      wireMediaEvents(media);
    }
    configureMedia(media, descriptor, object, reused);
    if (useRegistry) registerAcross(media, ownerKey, sourceKey);
    mediaOwners.set(media, node);
    node.classList.add("has-native-media", `native-media-${descriptor.kind}`);
    node.append(media);

    if (!interactive) {
      node.addEventListener("click", (event) => {
        if (event.defaultPrevented) return;
        event.stopPropagation();
        controlNode(node, media.paused ? "play" : "pause");
      });
    }
    return media;
  }

  function normalizeAction(action) {
    const value = String(action || "play").toLowerCase();
    if (value.includes("pause")) return "pause";
    if (value.includes("stop")) return "stop";
    if (value.includes("toggle")) return "toggle";
    return "play";
  }

  function controlNode(node, action) {
    const media = node?.querySelector?.("audio.native-media,video.native-media");
    if (!media) return false;
    const command = normalizeAction(action);
    if (command === "pause" || (command === "toggle" && !media.paused)) {
      media.pause();
      return true;
    }
    if (command === "stop") {
      media.pause();
      seekToStart(media);
      updateOwnerState(media, "pause");
      return true;
    }
    const start = Number(media.dataset.trimStart) || 0;
    const end = playbackEnd(media);
    if (media.ended || media.currentTime < start || (end != null && media.currentTime >= end)) seekToStart(media);
    // HTMLMediaElement.play() is idempotent while already playing; never seek
    // a live cross-slide session merely because its play effect is seen again.
    void media.play().catch(() => {
      node.classList.add("media-play-blocked");
      node.dataset.mediaPrompt = node.classList.contains("native-media-audio") ? "单击启用音频" : "单击播放视频";
    });
    return true;
  }

  function control(root, objectId, action) {
    if (!root || !objectId) return false;
    const escaped = globalThis.CSS?.escape ? CSS.escape(objectId) : String(objectId).replace(/["\\]/g, "\\$&");
    return controlNode(root.querySelector(`[data-id="${escaped}"]`), action);
  }

  function stopAll(root, reset = false, preserveAcrossSlides = false) {
    const mediaNodes = [...(root?.querySelectorAll?.("audio.native-media,video.native-media") || [])];
    for (const media of mediaNodes) {
      if (
        preserveAcrossSlides
        && media.matches?.("audio.native-media")
        && media.dataset.playAcrossSlides === "1"
      ) {
        // Moving the one live decoder preserves both playback position and a
        // deliberately paused state while the outgoing slide DOM is replaced.
        updateOwnerState(media, "pause");
        mediaOwners.delete(media);
        persistentHost().append(media);
        continue;
      }
      media.pause();
      if (reset) seekToStart(media);
      unregisterAcross(media);
    }
  }

  function stopPersistent(reset = true) {
    const host = persistentHost(false);
    if (!host) return;
    stopAll(host, reset, false);
    host.replaceChildren();
  }

  function debugRegistrySize() {
    return new Set(activeAcrossByOwner.values()).size;
  }

  globalThis.UniPptMedia = {
    attach,
    control,
    controlNode,
    debugRegistrySize,
    effectivePlaybackEnd,
    mediaIdentity,
    normalizeObject,
    setOutput,
    stopAll,
    stopPersistent,
  };
})();
