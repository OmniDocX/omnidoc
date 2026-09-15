(function installUniPptPresentationFlow(root) {
  "use strict";

  function clampCursor(animations, cursor) {
    return Math.max(0, Math.min(
      animations.length,
      Math.trunc(Number(cursor) || 0),
    ));
  }

  function nativeRepeatCount(animation) {
    const token = String(animation?.repeatCount ?? "").trim().toLowerCase();
    if (!token) return 1;
    if (token === "indefinite") return Infinity;
    const fixedPoint = Number(token);
    if (!Number.isFinite(fixedPoint) || fixedPoint <= 0) return 1;
    // OOXML stores repeatCount in thousandths: 2000 means two cycles.
    return fixedPoint / 1000;
  }

  function animationPlaybackTiming(animation) {
    const nativeSpeed = Number(animation?.speed);
    const speedRatio = Number.isFinite(nativeSpeed) && nativeSpeed !== 0
      ? Math.abs(nativeSpeed) / 100000
      : 1;
    const duration = Math.max(1, (Number(animation?.durationMs) || 500) / speedRatio);
    const autoReverse = Boolean(animation?.autoReverse);
    const repeatCount = nativeRepeatCount(animation);
    const repeatDuration = Number(animation?.repeatDurationMs);
    let iterations = repeatCount * (autoReverse ? 2 : 1);
    if (Number.isFinite(repeatDuration) && repeatDuration > 0) {
      iterations = Math.min(iterations, repeatDuration / duration);
    }
    const acceleration = Math.max(0, Math.min(100000, Number(animation?.acceleration) || 0));
    const deceleration = Math.max(0, Math.min(100000, Number(animation?.deceleration) || 0));
    const filteredEasing = nativeTimeFilterEasing(animation?.timeFilter);
    let easing = filteredEasing || "linear";
    if (!filteredEasing) {
      if (acceleration > 0 && deceleration > 0) easing = "cubic-bezier(.42,0,.58,1)";
      else if (acceleration > 0) easing = "cubic-bezier(.42,0,1,1)";
      else if (deceleration > 0) easing = "cubic-bezier(0,0,.2,1)";
    }
    const reversed = Number.isFinite(nativeSpeed) && nativeSpeed < 0;
    return {
      duration,
      iterations: Math.max(Number.MIN_VALUE, iterations),
      // Reversing a forward-then-back auto-reverse cycle yields the same
      // triangular timeline, so only a one-way effect needs `reverse`.
      direction: autoReverse ? "alternate" : (reversed ? "reverse" : "normal"),
      easing,
      fill: "forwards",
    };
  }

  function nativeTimeFilterEasing(value) {
    const text = String(value ?? "").trim();
    if (!text) return null;
    const points = text.split(";").map((token) => {
      const pair = token.trim().split(/\s*,\s*/).map(Number);
      if (pair.length !== 2 || !pair.every(Number.isFinite)) return null;
      return { input: pair[0], output: pair[1] };
    });
    if (points.length < 2 || points.some((point) => !point)) return null;
    const candidate = `linear(${points.map((point) => `${point.output} ${point.input * 100}%`).join(",")})`;
    if (root.CSS?.supports && !root.CSS.supports("animation-timing-function", candidate)) return null;
    return candidate;
  }

  function animationActiveDuration(animation) {
    const timing = animationPlaybackTiming(animation);
    // An indefinite emphasis remains alive until the page changes, but must
    // not permanently block PowerPoint's next navigation position.
    const iterations = Number.isFinite(timing.iterations)
      ? timing.iterations
      : (animation?.autoReverse ? 2 : 1);
    return Math.max(1, timing.duration * iterations);
  }

  function scheduleAnimationBatch(animations, startIndex = 0) {
    const entries = [];
    let cursor = clampCursor(animations, startIndex);
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
      entries.push({ animation, index: cursor, start, end });
      groupEnd = Math.max(groupEnd, end);
      batchEnd = Math.max(batchEnd, end);
      cursor += 1;
    }
    return { entries, nextIndex: cursor, end: batchEnd };
  }

  function navigationPositionsForEntries(entries) {
    const positions = [];
    for (const entry of entries) {
      const previous = positions[positions.length - 1];
      const trigger = entry.animation.trigger || "onClick";
      const sharesStart = previous && Math.abs(entry.start - previous.start) < 0.5;
      if (previous && (trigger === "withPrevious" || sharesStart)) {
        previous.entries.push(entry);
        previous.endIndex = entry.index + 1;
        previous.end = Math.max(previous.end, entry.end);
        continue;
      }
      positions.push({
        startIndex: entry.index,
        endIndex: entry.index + 1,
        start: entry.start,
        end: entry.end,
        entries: [entry],
      });
    }
    return positions;
  }

  function scheduleAnimationNavigation(animations, startIndex = 0) {
    const batch = scheduleAnimationBatch(animations, startIndex);
    return { ...batch, positions: navigationPositionsForEntries(batch.entries) };
  }

  function animationNavigationRanges(animations) {
    const ranges = [];
    let cursor = 0;
    while (cursor < animations.length) {
      const schedule = scheduleAnimationNavigation(animations, cursor);
      if (schedule.nextIndex <= cursor) break;
      for (const position of schedule.positions) {
        ranges.push({ startIndex: position.startIndex, endIndex: position.endIndex });
      }
      cursor = schedule.nextIndex;
    }
    return ranges;
  }

  function previousAnimationNavigationCursor(animations, cursor) {
    const completed = clampCursor(animations, cursor);
    if (completed === 0) return null;
    for (const range of animationNavigationRanges(animations)) {
      if (completed <= range.endIndex) return range.startIndex;
    }
    return 0;
  }

  function animationSchedule(animations, clickGap = 180) {
    const schedule = [];
    let cursor = 0;
    let batchOrigin = 0;
    while (cursor < animations.length) {
      const batch = scheduleAnimationBatch(animations, cursor);
      for (const entry of batch.entries) {
        schedule.push({
          ...entry,
          start: batchOrigin + entry.start,
          end: batchOrigin + entry.end,
        });
      }
      cursor = batch.nextIndex;
      batchOrigin += batch.end + (cursor < animations.length ? clickGap : 0);
    }
    return schedule;
  }

  function animationBatchRanges(animations) {
    const ranges = [];
    let cursor = 0;
    while (cursor < animations.length) {
      const batch = scheduleAnimationBatch(animations, cursor);
      if (batch.nextIndex <= cursor) break;
      ranges.push({ startIndex: cursor, endIndex: batch.nextIndex });
      cursor = batch.nextIndex;
    }
    return ranges;
  }

  function previousAnimationBatchCursor(animations, cursor) {
    const completed = clampCursor(animations, cursor);
    if (completed === 0) return null;
    for (const range of animationBatchRanges(animations)) {
      if (completed <= range.endIndex) return range.startIndex;
    }
    return 0;
  }

  root.UniPptPresentationFlow = Object.freeze({
    nativeRepeatCount,
    nativeTimeFilterEasing,
    animationPlaybackTiming,
    animationActiveDuration,
    scheduleAnimationBatch,
    navigationPositionsForEntries,
    scheduleAnimationNavigation,
    animationNavigationRanges,
    previousAnimationNavigationCursor,
    animationSchedule,
    animationBatchRanges,
    previousAnimationBatchCursor,
  });
})(globalThis);
