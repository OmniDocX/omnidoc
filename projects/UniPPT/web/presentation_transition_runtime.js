(function installUniPptPresentationTransitions(root) {
  "use strict";

  let cloneSequence = 0;
  const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
  const SIMPLE_WIPES = new Set([
    "wipe", "push", "pull", "cover", "strips", "split", "blinds",
    "checker", "comb", "gallery", "conveyor", "switch", "pan", "ferris",
  ]);
  const RADIAL = new Set(["circle", "diamond", "plus", "wedge", "wheel"]);
  const RANDOM_KINDS = Object.freeze([
    "fade", "wipe", "push", "cover", "pull", "strips",
    "split", "blinds", "checker", "circle", "diamond", "wheel", "zoom",
  ]);
  const RANDOM_DIRECTIONS = Object.freeze(["left", "right", "up", "down"]);
  const SHARED_KINDS = new Set([
    ...SIMPLE_WIPES,
    ...RADIAL,
    "fade", "zoom", "morph", "cut", "none",
    "pagecurldouble", "pagecurl", "peeloff", "origami", "fallover",
    "ripple", "honeycomb", "glitter", "prism",
    "wind", "warp", "vortex",
    "curtains", "drape", "doors",
    "random",
  ]);

  function slideSeed(options = {}) {
    const rawIndex = Number(options.slideIndex);
    let value = Number.isFinite(rawIndex) ? Math.trunc(rawIndex) : 0;
    value = Math.imul((value + 1) ^ 0x9e3779b9, 0x85ebca6b);
    value ^= value >>> 13;
    value = Math.imul(value, 0xc2b2ae35);
    return (value ^ (value >>> 16)) >>> 0;
  }

  function resolveRandomTransition(transition, options = {}) {
    if (String(transition?.kind || "fade").toLowerCase() !== "random") return transition;
    const seed = slideSeed(options);
    return {
      ...transition,
      kind: RANDOM_KINDS[seed % RANDOM_KINDS.length],
      direction: transition?.direction
        || RANDOM_DIRECTIONS[Math.floor(seed / RANDOM_KINDS.length) % RANDOM_DIRECTIONS.length],
    };
  }

  function ensureStyle() {
    if (typeof document === "undefined" || document.getElementById("unippt-shared-transition-style")) return;
    const style = document.createElement("style");
    style.id = "unippt-shared-transition-style";
    style.textContent = `
.slide-transition-underlay{z-index:0;pointer-events:none}
.page-book-frame{position:absolute;z-index:4;overflow:visible;pointer-events:none;transform-origin:50% 50%;transform-style:preserve-3d;perspective-origin:50% 50%}
.page-book-turning-leaf{position:absolute;z-index:2;overflow:visible;pointer-events:none;transform-style:preserve-3d;will-change:transform}
.page-book-leaf-face{position:absolute;inset:0;overflow:visible;pointer-events:none;transform-style:preserve-3d;background:#f7f3f1;clip-path:inset(0)}
.page-book-leaf-front{z-index:2;transform:translateZ(.15px);backface-visibility:hidden}
.page-book-leaf-back{z-index:1;transform:rotateY(180deg) translateZ(.15px);background:#fbf6f3;backface-visibility:visible}
.page-book-leaf-texture{pointer-events:none}
.page-book-leaf-lighting{position:absolute;top:0;bottom:0;left:-45%;right:-45%;z-index:3;pointer-events:none;opacity:0;will-change:transform,opacity}
.page-book-leaf-front-lighting{background:linear-gradient(90deg,transparent 6%,rgba(0,0,0,.05) 24%,rgba(0,0,0,.2) 58%,rgba(255,255,255,.3) 78%,rgba(255,255,255,.06) 94%)}
.page-book-leaf-back-lighting{background:linear-gradient(270deg,transparent 6%,rgba(0,0,0,.05) 24%,rgba(0,0,0,.17) 58%,rgba(255,252,248,.26) 78%,rgba(255,255,255,.06) 94%)}
.page-book-spine{position:absolute;z-index:3;width:14px;pointer-events:none;background:linear-gradient(90deg,transparent,rgba(0,0,0,.18),rgba(255,255,255,.32),rgba(0,0,0,.12),transparent);opacity:0;will-change:opacity}
.page-book-shadow-frame,.wind-transition-shadow-frame{position:absolute;z-index:2;overflow:visible;pointer-events:none;transform-origin:50% 50%}
.page-book-free-edge-shadow{position:absolute;left:0;top:0;width:96px;height:100%;pointer-events:none;background:linear-gradient(90deg,transparent,rgba(0,0,0,.035) 20%,rgba(0,0,0,.22) 50%,rgba(0,0,0,.035) 80%,transparent);will-change:transform,opacity}
.page-book-free-edge-shadow.turns-right{background:linear-gradient(270deg,transparent,rgba(0,0,0,.035) 20%,rgba(0,0,0,.22) 50%,rgba(0,0,0,.035) 80%,transparent)}
.ripple-refraction-layer{--ripple-radius:-9%;--ripple-band-offset:0%;--ripple-band-width:7%;position:absolute;left:50%;top:50%;pointer-events:none;overflow:hidden;backface-visibility:hidden;mask-image:radial-gradient(circle farthest-corner at 50% 50%,transparent 0 calc(var(--ripple-radius) + var(--ripple-band-offset) - var(--ripple-band-width)),#000 calc(var(--ripple-radius) + var(--ripple-band-offset) - var(--ripple-band-width) + .7%),#000 calc(var(--ripple-radius) + var(--ripple-band-offset) + var(--ripple-band-width) - .7%),transparent calc(var(--ripple-radius) + var(--ripple-band-offset) + var(--ripple-band-width)) 100%);-webkit-mask-image:radial-gradient(circle farthest-corner at 50% 50%,transparent 0 calc(var(--ripple-radius) + var(--ripple-band-offset) - var(--ripple-band-width)),#000 calc(var(--ripple-radius) + var(--ripple-band-offset) - var(--ripple-band-width) + .7%),#000 calc(var(--ripple-radius) + var(--ripple-band-offset) + var(--ripple-band-width) - .7%),transparent calc(var(--ripple-radius) + var(--ripple-band-offset) + var(--ripple-band-width)) 100%);mask-repeat:no-repeat;-webkit-mask-repeat:no-repeat;will-change:transform,opacity}
.ripple-refraction-caustic{--ripple-radius:-8%;position:absolute;left:50%;top:50%;z-index:5;pointer-events:none;background:radial-gradient(circle farthest-corner at 50% 50%,transparent calc(var(--ripple-radius) - 4.5%),#ffffff14 calc(var(--ripple-radius) - 2.8%),#ffffffb8 calc(var(--ripple-radius) - .45%),#9bc9e34a calc(var(--ripple-radius) + 1.4%),transparent calc(var(--ripple-radius) + 4.8%));will-change:transform,opacity}
.wind-transition-mesh{position:absolute;z-index:3;overflow:visible;pointer-events:none;transform-origin:50% 50%;transform-style:preserve-3d;perspective:1400px;perspective-origin:50% 42%}
.wind-transition-strip{position:absolute;pointer-events:none;overflow:hidden;transform-style:preserve-3d;backface-visibility:hidden;will-change:transform;contain:paint}
.wind-transition-lighting{position:absolute;inset:0;z-index:2;pointer-events:none;background:linear-gradient(90deg,transparent 0%,rgba(255,255,255,.26) 32%,rgba(0,0,0,.2) 70%,transparent 100%);opacity:0;will-change:opacity}
.wind-transition-sheet-shadow{position:absolute;left:4%;top:0;width:92%;height:24%;border-radius:50%;pointer-events:none;background:radial-gradient(ellipse at 50% 0%,rgba(0,0,0,.2),rgba(0,0,0,.07) 38%,transparent 72%);opacity:0;will-change:transform,opacity}
.page-curl-mesh{position:absolute;z-index:5;overflow:visible;pointer-events:none;transform-origin:50% 50%;transform-style:preserve-3d;perspective-origin:50% 50%}
.page-curl-mesh-strip{position:absolute;overflow:visible;pointer-events:none;transform-origin:50% 50%;transform-style:preserve-3d;will-change:transform}
.page-curl-strip-face{position:absolute;inset:0;overflow:hidden;pointer-events:none;backface-visibility:hidden;transform-style:preserve-3d}
.page-curl-strip-front{z-index:2}
.page-curl-strip-back{z-index:1;transform:rotateY(180deg);background:#f7f3f1}
.page-curl-strip-light{position:absolute;inset:0;z-index:3;pointer-events:none;background:linear-gradient(90deg,transparent,rgba(0,0,0,.2) 46%,rgba(255,255,255,.16) 78%,transparent);opacity:0}
.page-curl-old-flat{pointer-events:none;backface-visibility:hidden}
.page-curl-fold-frame{z-index:6}
.page-curl-cast-frame{z-index:2}
.page-curl-moving-shadow{position:absolute;left:0;top:0;height:100%;pointer-events:none;transform-origin:0 50%;will-change:transform,opacity}
.page-curl-fold-shadow{width:56px;background:linear-gradient(90deg,transparent 0%,rgba(0,0,0,.3) 42%,rgba(0,0,0,.08) 62%,rgba(255,255,255,.2) 82%,transparent 100%)}
.page-curl-cast-shadow{width:92px;background:linear-gradient(90deg,rgba(0,0,0,.17) 0%,rgba(0,0,0,.08) 28%,rgba(0,0,0,.018) 66%,transparent 100%)}
.page-curl-fold-shadow.turns-right{background:linear-gradient(270deg,transparent 0%,rgba(0,0,0,.32) 42%,rgba(0,0,0,.09) 62%,rgba(255,255,255,.22) 82%,transparent 100%)}
.page-curl-cast-shadow.turns-right{background:linear-gradient(270deg,rgba(0,0,0,.2) 0%,rgba(0,0,0,.1) 20%,rgba(0,0,0,.025) 54%,transparent 100%)}
.page-turn-peel-frame,.page-turn-origami-mesh,.page-turn-fall-frame{position:absolute;z-index:4;overflow:visible;pointer-events:none;transform-origin:50% 50%;transform-style:preserve-3d;perspective-origin:50% 50%}
.page-turn-peel-sheet,.page-turn-origami-flap,.page-turn-fall-sheet{position:absolute;inset:0;overflow:hidden;pointer-events:none;transform-style:preserve-3d;will-change:transform,opacity,clip-path}
.page-turn-peel-light,.page-turn-origami-light{position:absolute;inset:0;z-index:3;pointer-events:none;opacity:0}
.page-turn-peel-light{background:linear-gradient(135deg,transparent 20%,rgba(0,0,0,.18) 48%,rgba(255,255,255,.22) 62%,transparent 88%)}
.page-turn-origami-light{background:linear-gradient(90deg,transparent,rgba(255,255,255,.16) 40%,rgba(0,0,0,.2) 70%,transparent)}
.page-turn-fall-shadow{position:absolute;left:8%;right:8%;height:18%;border-radius:50%;pointer-events:none;background:radial-gradient(ellipse at 50% 50%,rgba(0,0,0,.28),rgba(0,0,0,.08) 42%,transparent 74%);opacity:0;will-change:transform,opacity}
`;
    document.head.append(style);
  }

  function namespaceCloneIds(node, label = "transition") {
    const prefix = `unippt-${label}-${++cloneSequence}`;
    const replacements = new Map();
    for (const candidate of [node, ...node.querySelectorAll("[id]")]) {
      const original = candidate.getAttribute?.("id");
      if (!original) continue;
      const next = `${prefix}-${original.replace(/[^A-Za-z0-9_-]/g, "-")}`;
      replacements.set(original, next);
      candidate.setAttribute("id", next);
    }
    if (!replacements.size) return node;
    for (const candidate of [node, ...node.querySelectorAll("*")]) {
      for (const attribute of [...(candidate.attributes || [])]) {
        let value = attribute.value;
        for (const [original, next] of replacements) {
          value = value
            .replaceAll(`url(#${original})`, `url(#${next})`)
            .replaceAll(`url("#${original}")`, `url("#${next}")`)
            .replaceAll(`url('#${original}')`, `url('#${next}')`);
          if (value === `#${original}`) value = `#${next}`;
        }
        if (["aria-labelledby", "aria-describedby", "for"].includes(attribute.name)) {
          value = value.split(/\s+/).map((token) => replacements.get(token) || token).join(" ");
        }
        if (value !== attribute.value) candidate.setAttribute(attribute.name, value);
      }
    }
    return node;
  }

  function removeCloneMedia(node) {
    node.querySelectorAll("audio").forEach((media) => media.remove());
    node.querySelectorAll("video").forEach((media) => {
      media.autoplay = false;
      media.muted = true;
      media.removeAttribute("controls");
    });
  }

  function captureUnderlay(stage) {
    ensureStyle();
    if (!stage?.parentElement || !stage.childNodes.length) return null;
    stage.parentElement.querySelectorAll(":scope > .slide-transition-underlay").forEach((node) => node.remove());
    const underlay = stage.cloneNode(true);
    namespaceCloneIds(underlay, "underlay");
    underlay.classList.add("slide-transition-underlay");
    underlay.setAttribute("aria-hidden", "true");
    removeCloneMedia(underlay);
    stage.parentElement.insertBefore(underlay, stage);
    return underlay;
  }

  function stageMetrics(stage, underlay) {
    return {
      width: Math.max(1, Number(stage.clientWidth) || parseFloat(stage.style.width || underlay?.style.width) || 1280),
      height: Math.max(1, Number(stage.clientHeight) || parseFloat(stage.style.height || underlay?.style.height) || 720),
      baseTransform: underlay?.style.transform || stage.style.transform || "",
    };
  }

  function synchronize(animations) {
    const timelineNow = Number(document?.timeline?.currentTime);
    if (!Number.isFinite(timelineNow)) return null;
    const sharedStartTime = timelineNow + 1;
    for (const animation of animations || []) {
      try { animation.startTime = sharedStartTime; } catch (_error) {}
    }
    return sharedStartTime;
  }

  /**
   * Sampled from PowerPoint's 1.25s pageCurlDouble recording. Must stay in step
   * with `topDownBookLeafFrames` in the editor so a deck turns its pages the
   * same way in the editor, the exported player, and the rendered video.
   */
  function leafSamples(direction) {
    const sign = ["r", "right"].includes(String(direction).toLowerCase()) ? 1 : -1;
    return [
      [0, 0], [.08, 4], [.18, 16], [.3, 37], [.42, 66], [.5, 90],
      [.6, 119], [.72, 149], [.84, 169], [.94, 178], [1, 180],
    ].map(([offset, angle]) => {
      const radians = angle * Math.PI / 180;
      const grazing = Math.sin(radians);
      const facing = Math.max(0, Math.cos(radians));
      const away = Math.max(0, -Math.cos(radians));
      return {
        offset,
        angle,
        transform: `rotateY(${(sign * angle).toFixed(3)}deg)`,
        shadeOpacity: (.26 * grazing).toFixed(4),
        shadowOpacity: (.22 * grazing).toFixed(4),
        frontShade: (.34 * grazing * (.4 + .6 * facing)).toFixed(4),
        frontSlide: (sign * -32 * grazing * facing).toFixed(3),
        backShade: (.3 * grazing * (.4 + .6 * away)).toFixed(4),
        backSlide: (sign * 32 * grazing * away).toFixed(3),
        // Chromium drops backface-visibility on overflow/contain faces, so the
        // printed side would stay visible after vertical. Drive both faces
        // with the same facing term the lighting already uses.
        frontOpacity: clamp((facing - .02) / .12, 0, 1).toFixed(4),
        backOpacity: clamp((away - .02) / .12, 0, 1).toFixed(4),
        shadowScale: (.34 + .66 * Math.abs(Math.cos(radians))).toFixed(4),
      };
    });
  }

  function makeLeafFace(source, width, height, face, sourceLeft) {
    const back = face === "back";
    const surface = document.createElement("div");
    surface.className = `page-book-leaf-face page-book-leaf-${back ? "back" : "front"}`;
    const texture = source.cloneNode(true);
    namespaceCloneIds(texture, `book-${face}`);
    texture.classList.remove("slide-transition-underlay");
    texture.classList.add("page-book-leaf-texture");
    removeCloneMedia(texture);
    Object.assign(texture.style, {
      position: "absolute", inset: "auto", left: `${(-sourceLeft).toFixed(3)}px`, top: "0",
      width: `${width}px`, height: `${height}px`, transform: "none", transformOrigin: "0 0",
      clipPath: "none", visibility: "visible", zIndex: "1", willChange: "auto",
    });
    surface.append(texture);
    return surface;
  }

  function createBook(stage, duration, direction, underlay) {
    if (!underlay || !stage.parentElement) return null;
    const presenter = stage.parentElement;
    const turnsRight = ["r", "right"].includes(String(direction).toLowerCase());
    const { width, height, baseTransform } = stageMetrics(stage, underlay);
    const samples = leafSamples(direction);
    stage.style.zIndex = "1";
    underlay.style.zIndex = "2";
    underlay.style.clipPath = turnsRight ? "inset(0 0 0 50%)" : "inset(0 50% 0 0)";

    const frame = document.createElement("div");
    frame.className = "page-book-frame";
    frame.dataset.pageCurlTopology = "top-down-open-book-even-leaf";
    Object.assign(frame.style, {
      left: "50%", top: "50%", width: `${width}px`, height: `${height}px`,
      transform: baseTransform, perspective: `${Math.max(1600, width * 1.65).toFixed(0)}px`,
    });
    const leaf = document.createElement("div");
    leaf.className = "page-book-turning-leaf";
    Object.assign(leaf.style, {
      left: turnsRight ? "0" : "50%", top: "0", width: "50%", height: "100%",
      transformOrigin: turnsRight ? "100% 50%" : "0 50%",
    });
    const front = makeLeafFace(underlay, width, height, "front", turnsRight ? 0 : width * .5);
    const back = makeLeafFace(stage, width, height, "back", turnsRight ? width * .5 : 0);
    const frontLight = document.createElement("div");
    frontLight.className = "page-book-leaf-lighting page-book-leaf-front-lighting";
    const backLight = document.createElement("div");
    backLight.className = "page-book-leaf-lighting page-book-leaf-back-lighting";
    front.append(frontLight);
    back.append(backLight);
    leaf.append(front, back);
    const spine = document.createElement("div");
    spine.className = "page-book-spine";
    Object.assign(spine.style, { left: "calc(50% - 7px)", top: "0", height: "100%" });
    frame.append(spine, leaf);
    presenter.insertBefore(frame, stage.nextSibling);

    const leafAnimation = leaf.animate(samples.map((sample) => ({
      offset: sample.offset, opacity: 1, transform: sample.transform,
    })), { duration, easing: "linear", fill: "both" });
    const animations = [
      leafAnimation,
      front.animate(samples.map((sample) => ({
        offset: sample.offset, opacity: sample.frontOpacity,
      })), { duration, easing: "linear", fill: "both" }),
      back.animate(samples.map((sample) => ({
        offset: sample.offset, opacity: sample.backOpacity,
      })), { duration, easing: "linear", fill: "both" }),
      frontLight.animate(samples.map((sample) => ({
        offset: sample.offset,
        opacity: sample.frontShade,
        transform: `translate3d(${sample.frontSlide}%,0,0)`,
      })), { duration, easing: "linear", fill: "both" }),
      backLight.animate(samples.map((sample) => ({
        offset: sample.offset,
        opacity: sample.backShade,
        transform: `translate3d(${sample.backSlide}%,0,0)`,
      })), { duration, easing: "linear", fill: "both" }),
      spine.animate(samples.map((sample) => ({ offset: sample.offset, opacity: Math.min(.58, Number(sample.shadowOpacity) * 2.25).toFixed(4) })), { duration, easing: "linear", fill: "both" }),
    ];

    const shadowFrame = document.createElement("div");
    shadowFrame.className = "page-book-shadow-frame";
    Object.assign(shadowFrame.style, {
      left: "50%", top: "50%", width: `${width}px`, height: `${height}px`, transform: baseTransform,
    });
    const shadow = document.createElement("div");
    shadow.className = `page-book-free-edge-shadow${turnsRight ? " turns-right" : ""}`;
    shadowFrame.append(shadow);
    presenter.insertBefore(shadowFrame, stage.nextSibling);
    animations.push(shadow.animate(samples.map((sample) => {
      const edgeX = width * .5 + (turnsRight ? -1 : 1) * width * .5 * Math.cos(sample.angle * Math.PI / 180);
      return {
        offset: sample.offset,
        opacity: sample.shadowOpacity,
        transform: `translate3d(${(edgeX - 48).toFixed(3)}px,0,0) scaleX(${sample.shadowScale})`,
      };
    }), { duration, easing: "linear", fill: "both" }));
    synchronize(animations);
    return {
      player: leafAnimation,
      animations,
      metrics: { quality: "shared", layers: 1, nodeCount: underlay.querySelectorAll("*").length, coherentLeaf: true },
      cleanup() { frame.remove(); shadowFrame.remove(); stage.style.zIndex = "1"; },
    };
  }

  function windState(progress, count, width, height, direction = "right") {
    const p = clamp(Number(progress) || 0, 0, 1);
    const movesLeft = ["l", "left"].includes(String(direction).toLowerCase());
    const sign = movesLeft ? -1 : 1;
    const q = clamp((p - .15) / .7, 0, 1);
    const travelX = sign * width * 1.22 * Math.pow(q, 2.1);
    const travelY = -height * .55 * Math.pow(q, 1.55);
    const envelope = q > 0 && q < 1 ? 34 * Math.pow(Math.sin(Math.PI * q), .55) : 0;
    const surfaceEnvelope = envelope / 34;
    const boundaries = Array.from({ length: count + 1 }, (_, index) => {
      const u = index / count;
      const phase = 2 * Math.PI * (1.1 * u - .18 * q);
      const topWave = Math.sin(phase - .35);
      const bottomWave = Math.sin(phase + .72);
      return {
        top: { x: travelX + u * width + sign * width * .014 * surfaceEnvelope * topWave, y: travelY + height * .025 * surfaceEnvelope * topWave },
        bottom: { x: travelX + u * width + sign * width * .052 * surfaceEnvelope * bottomWave, y: travelY + height + height * .09 * surfaceEnvelope * bottomWave },
      };
    });
    const poses = Array.from({ length: count }, (_, index) => {
      const phase = 2 * Math.PI * (1.1 * (index + .5) / count - .18 * q);
      const angle = sign * envelope * Math.sin(phase);
      return {
        angle,
        light: Math.min(.1, Math.abs(Math.sin(angle * Math.PI / 180)) * .11),
        quad: [boundaries[index].top, boundaries[index + 1].top, boundaries[index + 1].bottom, boundaries[index].bottom],
      };
    });
    return { q, travelX, travelY, poses };
  }

  function projectiveTransform(pose, sourceWidth, sourceHeight, originalLeft) {
    const width = Math.max(.001, Number(sourceWidth) || .001);
    const height = Math.max(.001, Number(sourceHeight) || .001);
    const [topLeft, topRight, bottomRight, bottomLeft] = pose.quad.map((point) => ({ x: point.x - originalLeft, y: point.y }));
    const dx1 = topRight.x - bottomRight.x;
    const dx2 = bottomLeft.x - bottomRight.x;
    const dx3 = topLeft.x - topRight.x + bottomRight.x - bottomLeft.x;
    const dy1 = topRight.y - bottomRight.y;
    const dy2 = bottomLeft.y - bottomRight.y;
    const dy3 = topLeft.y - topRight.y + bottomRight.y - bottomLeft.y;
    let projectX = 0;
    let projectY = 0;
    const denominator = dx1 * dy2 - dx2 * dy1;
    if ((Math.abs(dx3) > .000001 || Math.abs(dy3) > .000001) && Math.abs(denominator) > .000001) {
      projectX = (dx3 * dy2 - dx2 * dy3) / denominator;
      projectY = (dx1 * dy3 - dx3 * dy1) / denominator;
    }
    const values = [
      (topRight.x - topLeft.x + projectX * topRight.x) / width,
      (topRight.y - topLeft.y + projectX * topRight.y) / width,
      0, projectX / width,
      (bottomLeft.x - topLeft.x + projectY * bottomLeft.x) / height,
      (bottomLeft.y - topLeft.y + projectY * bottomLeft.y) / height,
      0, projectY / height,
      0, 0, 1, 0,
      topLeft.x, topLeft.y, 0, 1,
    ];
    return `matrix3d(${values.map((value) => (Math.abs(value) < 1e-9 ? 0 : value).toFixed(8)).join(",")})`;
  }

  function createWind(stage, duration, direction, underlay, quality = "high") {
    if (!underlay || !stage.parentElement) return null;
    const presenter = stage.parentElement;
    const { width, height, baseTransform } = stageMetrics(stage, underlay);
    const count = quality === "low"
      ? 4
      : quality === "medium"
        ? 6
        : underlay.querySelectorAll("*").length > 150 ? 8 : 10;
    const mesh = document.createElement("div");
    mesh.className = "wind-transition-mesh";
    Object.assign(mesh.style, { left: "50%", top: "50%", width: `${width}px`, height: `${height}px`, transform: baseTransform });
    presenter.insertBefore(mesh, stage.nextSibling);
    underlay.style.visibility = "hidden";
    stage.style.zIndex = "1";
    stage.style.opacity = "1";
    const animations = [];
    const strips = [];
    const offsets = [0, .08, .15, .2, .26, .34, .42, .5, .58, .66, .74, .82, .86, .92, 1];
    for (let index = 0; index < count; index += 1) {
      const left = index / count;
      const stripWidth = 1 / count;
      const strip = document.createElement("div");
      strip.className = "wind-transition-strip";
      Object.assign(strip.style, {
        left: `${(left * 100).toFixed(5)}%`, top: "0",
        width: `calc(${(stripWidth * 100).toFixed(5)}% + 1.4px)`, height: "100%", transformOrigin: "0 0",
      });
      const texture = underlay.cloneNode(true);
      namespaceCloneIds(texture, "wind");
      texture.classList.remove("slide-transition-underlay");
      removeCloneMedia(texture);
      Object.assign(texture.style, {
        position: "absolute", left: `${(-left * width).toFixed(3)}px`, top: "0",
        width: `${width}px`, height: `${height}px`, transform: "none", visibility: "visible",
      });
      strip.append(texture);
      const lighting = document.createElement("div");
      lighting.className = "wind-transition-lighting";
      strip.append(lighting);
      mesh.append(strip);
      strips.push(strip);
      animations.push(strip.animate(offsets.map((offset) => ({
        offset, opacity: 1,
        transform: projectiveTransform(windState(offset, count, width, height, direction).poses[index], stripWidth * width, height, left * width),
      })), { duration, easing: "linear", fill: "both" }));
      animations.push(lighting.animate(offsets.map((offset) => ({
        offset, opacity: windState(offset, count, width, height, direction).poses[index].light.toFixed(4),
      })), { duration, easing: "linear", fill: "both" }));
    }
    const shadowFrame = document.createElement("div");
    shadowFrame.className = "wind-transition-shadow-frame";
    Object.assign(shadowFrame.style, { left: "50%", top: "50%", width: `${width}px`, height: `${height}px`, transform: baseTransform });
    const shadow = document.createElement("div");
    shadow.className = "wind-transition-sheet-shadow";
    shadowFrame.append(shadow);
    presenter.insertBefore(shadowFrame, stage.nextSibling);
    animations.push(shadow.animate([0, .15, .26, .42, .58, .74, .86, 1].map((offset) => {
      const state = windState(offset, count, width, height, direction);
      return {
        offset,
        opacity: (.2 * Math.sin(Math.PI * state.q)).toFixed(4),
        transform: `translate3d(${(state.travelX * .92).toFixed(3)}px,${(state.travelY + height * .72).toFixed(3)}px,0) scaleX(${(1 - .18 * Math.sin(Math.PI * state.q)).toFixed(4)})`,
      };
    }), { duration, easing: "linear", fill: "both" }));
    synchronize(animations);
    return {
      player: animations[0], animations,
      metrics: { quality, layers: count + 1, nodeCount: underlay.querySelectorAll("*").length, coherentSheet: true },
      cleanup() { strips.forEach((node) => node.remove()); mesh.remove(); shadowFrame.remove(); stage.style.zIndex = "1"; },
    };
  }

  function createRipple(stage, duration, kind, underlay, quality = "high") {
    if (!underlay || !stage.parentElement) return null;
    const presenter = stage.parentElement;
    const { width, height, baseTransform } = stageMetrics(stage, underlay);
    const layerCount = quality === "low" ? 1 : quality === "medium" ? 2 : 3;
    const layers = [];
    const animations = [];
    underlay.style.zIndex = "2";
    stage.style.zIndex = "3";
    const player = underlay.animate([
      { opacity: 1, transform: baseTransform },
      { offset: .7, opacity: 1, transform: `${baseTransform} scale(1.006)` },
      { opacity: 0, transform: `${baseTransform} scale(1.018)` },
    ], { duration, easing: "linear", fill: "both" });
    animations.push(player);
    for (let index = 0; index < layerCount; index += 1) {
      const layer = underlay.cloneNode(true);
      namespaceCloneIds(layer, "ripple");
      layer.classList.remove("slide-transition-underlay");
      layer.classList.add("ripple-refraction-layer");
      removeCloneMedia(layer);
      const polarity = index - 1;
      Object.assign(layer.style, {
        left: "50%", top: "50%", width: `${width}px`, height: `${height}px`, zIndex: "4",
        transform: baseTransform, transformOrigin: "50% 50%",
        "--ripple-band-offset": `${polarity * 1.45}%`,
        "--ripple-band-width": kind === "prism" ? "8.5%" : "7%",
      });
      presenter.insertBefore(layer, stage.nextSibling);
      layers.push(layer);
      animations.push(layer.animate([
        { offset: 0, opacity: 0, transform: `${baseTransform} scale(.992)`, "--ripple-radius": "-9%" },
        { offset: .06, opacity: .78 - Math.abs(polarity) * .1, transform: `${baseTransform} scale(${.997 + polarity * .002})`, "--ripple-radius": "-4%" },
        { offset: .5, opacity: .9 - Math.abs(polarity) * .1, transform: `${baseTransform} scale(${1.012 + polarity * .004})`, "--ripple-radius": "35%" },
        { offset: .88, opacity: .65 - Math.abs(polarity) * .08, transform: `${baseTransform} scale(${1.018 + polarity * .003})`, "--ripple-radius": "69%" },
        { offset: 1, opacity: 0, transform: `${baseTransform} scale(1.022)`, "--ripple-radius": "80%" },
      ], { duration, easing: "linear", fill: "both" }));
    }
    const caustic = document.createElement("div");
    caustic.className = "ripple-refraction-caustic";
    Object.assign(caustic.style, { width: `${width}px`, height: `${height}px`, transform: baseTransform });
    presenter.insertBefore(caustic, stage.nextSibling);
    layers.push(caustic);
    animations.push(caustic.animate([
      { opacity: 0, transform: baseTransform, "--ripple-radius": "-8%" },
      { offset: .08, opacity: .72, "--ripple-radius": "-2%" },
      { offset: .72, opacity: .42, "--ripple-radius": "55%" },
      { opacity: 0, transform: `${baseTransform} scale(1.01)`, "--ripple-radius": "80%" },
    ], { duration, easing: "linear", fill: "both" }));
    animations.push(stage.animate([
      { opacity: .98, clipPath: "circle(0% at 50% 50%)" },
      { offset: .5, opacity: 1, clipPath: "circle(35% at 50% 50%)" },
      { opacity: 1, clipPath: "circle(78% at 50% 50%)" },
    ], { duration, easing: "linear", fill: "both" }));
    synchronize(animations);
    return {
      player, animations,
      metrics: { quality, layers: layerCount, nodeCount: underlay.querySelectorAll("*").length },
      cleanup() { layers.forEach((node) => node.remove()); stage.style.zIndex = "1"; },
    };
  }

  function cloneSlideSurface(source, label, width, height, baseTransform) {
    const node = source.cloneNode(true);
    namespaceCloneIds(node, label);
    node.classList.remove("slide-transition-underlay");
    removeCloneMedia(node);
    Object.assign(node.style, {
      position: "absolute", left: "50%", top: "50%",
      width: `${width}px`, height: `${height}px`,
      transform: baseTransform, transformOrigin: "50% 50%",
      visibility: "visible", pointerEvents: "none",
    });
    return node;
  }

  function monotone(anchors, valueIndex, progress) {
    const count = anchors.length;
    if (!count) return 0;
    if (count === 1) return Number(anchors[0][valueIndex]) || 0;
    const p = clamp(Number(progress) || 0, anchors[0][0], anchors[count - 1][0]);
    const slopes = Array.from({ length: count - 1 }, (_, index) => {
      const dx = Math.max(.000001, anchors[index + 1][0] - anchors[index][0]);
      return (anchors[index + 1][valueIndex] - anchors[index][valueIndex]) / dx;
    });
    const tangents = Array(count).fill(0);
    tangents[0] = slopes[0];
    tangents[count - 1] = slopes[count - 2];
    for (let index = 1; index < count - 1; index += 1) {
      const before = slopes[index - 1];
      const after = slopes[index];
      if (before === 0 || after === 0 || Math.sign(before) !== Math.sign(after)) tangents[index] = 0;
      else {
        const leftWidth = anchors[index][0] - anchors[index - 1][0];
        const rightWidth = anchors[index + 1][0] - anchors[index][0];
        const weightA = 2 * rightWidth + leftWidth;
        const weightB = rightWidth + 2 * leftWidth;
        tangents[index] = (weightA + weightB) / (weightA / before + weightB / after);
      }
    }
    let interval = count - 2;
    for (let index = 0; index < count - 1; index += 1) {
      if (p <= anchors[index + 1][0]) { interval = index; break; }
    }
    const start = anchors[interval];
    const end = anchors[interval + 1];
    const width = Math.max(.000001, end[0] - start[0]);
    const t = clamp((p - start[0]) / width, 0, 1);
    const t2 = t * t;
    const t3 = t2 * t;
    return (2 * t3 - 3 * t2 + 1) * start[valueIndex]
      + (t3 - 2 * t2 + t) * width * tangents[interval]
      + (-2 * t3 + 3 * t2) * end[valueIndex]
      + (t3 - t2) * width * tangents[interval + 1];
  }

  function curlGeometryAt(progress, direction = "left") {
    const turnsRight = ["r", "right"].includes(String(direction).toLowerCase());
    const anchors = [
      [0, .72, 0], [.18, .72, 0], [.42, .58, .34], [.64, .34, .7],
      [.84, .08, .96], [1, 0, 1],
    ];
    const p = clamp(Number(progress) || 0, 0, 1);
    const hinge = monotone(anchors, 1, p);
    const theta = monotone(anchors, 2, p) * Math.PI;
    const materialSpan = Math.max(.0001, 1 - hinge);
    const radius = theta > .00001 ? materialSpan / theta : materialSpan;
    const radiusScale = 1 - .1 * Math.pow(theta / Math.PI, 2);
    const freeEdge = theta > .00001 ? hinge + radius * radiusScale * Math.sin(theta) : 1;
    const crest = theta >= Math.PI / 2 ? hinge + radius * radiusScale : freeEdge;
    const logicalLeft = Math.min(hinge, crest);
    const logicalRight = Math.max(hinge, crest);
    const physicalLeft = turnsRight ? 1 - logicalRight : logicalLeft;
    const physicalRight = turnsRight ? 1 - logicalLeft : logicalRight;
    const physicalHinge = turnsRight ? 1 - hinge : hinge;
    const liftT = clamp((p - .18) / .24, 0, 1);
    const leafLift = liftT * liftT * (3 - 2 * liftT);
    const flatBoundary = turnsRight
      ? leafLift * physicalHinge
      : 1 - leafLift * (1 - physicalHinge);
    return {
      progress: p, turnsRight, hinge, theta, radius, radiusScale, freeEdge, crest,
      sheetOpacity: p <= .889 ? 1 : clamp((1 - p) / .111, 0, 1),
      leafOpacity: p <= .889 ? 1 : clamp((1 - p) / .111, 0, 1),
      flatBoundary,
      hingeBoundary: physicalHinge,
      outerBoundary: turnsRight ? 1 - crest : crest,
      bandLeft: physicalLeft,
      bandRight: physicalRight,
      bandWidth: Math.max(0, physicalRight - physicalLeft),
    };
  }

  function curlFlatClip(geometry) {
    const boundary = clamp(geometry.flatBoundary * 100, 0, 100).toFixed(3);
    return geometry.turnsRight
      ? `polygon(${boundary}% 0,100% 0,100% 100%,${boundary}% 100%)`
      : `polygon(0 0,${boundary}% 0,${boundary}% 100%,0 100%)`;
  }

  function curlStripPose(strip, geometry, stageWidth) {
    if (geometry.theta <= .00001) {
      return { active: false, angle: 0, scaleX: 1, x: strip.center, z: 0, light: 0 };
    }
    const u = clamp(Number(strip.pageU), 0, 1);
    const phi = u * geometry.theta;
    const mappedLogical = geometry.hinge + geometry.radius * geometry.radiusScale * Math.sin(phi);
    const mappedPhysical = geometry.turnsRight ? 1 - mappedLogical : mappedLogical;
    const z = .4 * geometry.radius * stageWidth * (1 - Math.cos(phi));
    const tangentX = geometry.radiusScale * Math.cos(phi);
    const tangentZ = .4 * Math.sin(phi);
    const tangentAngle = Math.atan2(tangentZ, tangentX);
    return {
      active: true,
      angle: (geometry.turnsRight ? 1 : -1) * tangentAngle * 180 / Math.PI,
      scaleX: Math.max(.08, Math.hypot(tangentX, tangentZ)),
      x: mappedPhysical,
      z,
      light: .16 * Math.sin(Math.abs(tangentAngle)),
    };
  }

  function curlStripPlan(direction, requestedCount) {
    const turnsRight = ["r", "right"].includes(String(direction).toLowerCase());
    const requestedNumber = Number(requestedCount);
    const count = requestedCount == null || !Number.isFinite(requestedNumber)
      ? 20
      : clamp(Math.round(requestedNumber), 14, 20);
    const leafSpan = .28;
    const spine = turnsRight ? leafSpan : 1 - leafSpan;
    const strips = Array.from({ length: count }, (_, index) => {
      const width = leafSpan / count;
      const left = turnsRight ? spine - (index + 1) * width : spine + index * width;
      const backLeft = turnsRight ? spine + index * width : spine - (index + 1) * width;
      return {
        index, left, backLeft, width, center: left + width / 2, pageU: (index + .5) / count,
      };
    });
    return { turnsRight, strips };
  }

  function createPageCurl(stage, duration, direction, underlay, quality = "high") {
    if (!underlay || !stage.parentElement) return null;
    const presenter = stage.parentElement;
    const { width, height, baseTransform } = stageMetrics(stage, underlay);
    const plan = curlStripPlan(direction, quality === "low" ? 14 : quality === "medium" ? 16 : 20);
    const offsets = [0, .11, .18, .22, .278, .333, .389, .444, .5, .556, .612, .667, .722, .778, .834, .889, 1];
    const samples = offsets.map((offset) => ({ offset, geometry: curlGeometryAt(offset, direction) }));
    underlay.style.visibility = "hidden";
    stage.style.zIndex = "1";
    const oldFlat = cloneSlideSurface(underlay, "curl-flat", width, height, baseTransform);
    oldFlat.classList.add("page-curl-old-flat");
    oldFlat.style.zIndex = "2";
    presenter.insertBefore(oldFlat, stage.nextSibling);
    const player = oldFlat.animate(samples.map(({ offset, geometry }) => ({
      offset, clipPath: curlFlatClip(geometry),
    })), { duration, easing: "linear", fill: "both" });
    const mesh = document.createElement("div");
    mesh.className = "page-curl-mesh";
    if (plan.turnsRight) mesh.classList.add("turns-right");
    mesh.dataset.pageCurlSurface = "cylindrical-book-leaf";
    Object.assign(mesh.style, {
      left: "50%", top: "50%", width: `${width}px`, height: `${height}px`,
      transform: baseTransform, perspective: `${Math.max(900, width * 1.4).toFixed(0)}px`,
    });
    presenter.insertBefore(mesh, stage.nextSibling);
    const animations = [player];
    const stripNodes = [];
    for (const strip of plan.strips) {
      const shell = document.createElement("div");
      shell.className = "page-curl-mesh-strip";
      Object.assign(shell.style, {
        left: `${(strip.left * 100).toFixed(5)}%`, top: "0",
        width: `calc(${(strip.width * 100).toFixed(5)}% + .8px)`, height: "100%",
      });
      const front = document.createElement("div");
      front.className = "page-curl-strip-face page-curl-strip-front";
      const back = document.createElement("div");
      back.className = "page-curl-strip-face page-curl-strip-back";
      const frontTex = underlay.cloneNode(true);
      namespaceCloneIds(frontTex, "curl-front");
      frontTex.classList.remove("slide-transition-underlay");
      removeCloneMedia(frontTex);
      Object.assign(frontTex.style, {
        position: "absolute", inset: "auto", left: `${(-strip.left * width).toFixed(3)}px`, top: "0",
        width: `${width}px`, height: `${height}px`, transform: "none",
      });
      const backTex = stage.cloneNode(true);
      namespaceCloneIds(backTex, "curl-back");
      removeCloneMedia(backTex);
      Object.assign(backTex.style, {
        position: "absolute", inset: "auto", left: `${(-strip.backLeft * width).toFixed(3)}px`, top: "0",
        width: `${width}px`, height: `${height}px`, transform: "none",
      });
      const light = document.createElement("div");
      light.className = "page-curl-strip-light";
      front.append(frontTex, light);
      back.append(backTex);
      shell.append(front, back);
      mesh.append(shell);
      stripNodes.push(shell);
      animations.push(shell.animate(samples.map(({ offset, geometry }) => {
        const pose = curlStripPose(strip, geometry, width);
        const dx = pose.x * width - strip.center * width;
        return {
          offset,
          opacity: String(pose.active ? geometry.leafOpacity : 0),
          transform: `translate3d(${dx.toFixed(3)}px,0,${pose.z.toFixed(3)}px) rotateY(${pose.angle.toFixed(3)}deg) scaleX(${pose.scaleX.toFixed(4)})`,
        };
      }), { duration, easing: "linear", fill: "both" }));
      animations.push(light.animate(samples.map(({ offset, geometry }) => ({
        offset, opacity: curlStripPose(strip, geometry, width).light.toFixed(4),
      })), { duration, easing: "linear", fill: "both" }));
    }
    const foldFrame = document.createElement("div");
    foldFrame.className = "page-book-shadow-frame page-curl-fold-frame";
    const fold = document.createElement("div");
    fold.className = `page-curl-moving-shadow page-curl-fold-shadow${plan.turnsRight ? " turns-right" : ""}`;
    foldFrame.append(fold);
    const castFrame = document.createElement("div");
    castFrame.className = "page-book-shadow-frame page-curl-cast-frame";
    const cast = document.createElement("div");
    cast.className = `page-curl-moving-shadow page-curl-cast-shadow${plan.turnsRight ? " turns-right" : ""}`;
    castFrame.append(cast);
    for (const frame of [foldFrame, castFrame]) {
      Object.assign(frame.style, {
        left: "50%", top: "50%", width: `${width}px`, height: `${height}px`, transform: baseTransform,
      });
      presenter.insertBefore(frame, stage.nextSibling);
    }
    animations.push(fold.animate(samples.map(({ offset, geometry }) => ({
      offset,
      opacity: geometry.theta > .02 ? Math.min(.58, geometry.theta / Math.PI * .72) * geometry.sheetOpacity : 0,
      transform: `translateX(${(geometry.hingeBoundary * width - 28).toFixed(3)}px)`,
    })), { duration, easing: "linear", fill: "both" }));
    animations.push(cast.animate(samples.map(({ offset, geometry }) => ({
      offset,
      opacity: geometry.theta > .08 ? Math.min(.42, geometry.theta / Math.PI * .48) * geometry.sheetOpacity : 0,
      transform: `translateX(${(geometry.outerBoundary * width - 46).toFixed(3)}px)`,
    })), { duration, easing: "linear", fill: "both" }));
    synchronize(animations);
    return {
      player, animations,
      metrics: { quality, layers: plan.strips.length, nodeCount: underlay.querySelectorAll("*").length },
      cleanup() {
        stripNodes.forEach((node) => node.remove());
        mesh.remove(); oldFlat.remove(); foldFrame.remove(); castFrame.remove();
        stage.style.zIndex = "1";
      },
    };
  }

  function peelRemainderPolygon(progress, direction = "left") {
    const t = clamp(Number(progress) || 0, 0, 1) * 2;
    const d = String(direction).toLowerCase();
    const pct = (value) => `${(value * 100).toFixed(3)}%`;
    if (["r", "right"].includes(d)) {
      if (t <= 1) return `polygon(${pct(t)} 0,100% 0,100% 100%,0 100%,0 ${pct(t)})`;
      const u = t - 1;
      return `polygon(100% 0,100% 100%,${pct(u)} 100%,100% ${pct(u)})`;
    }
    if (["u", "up"].includes(d)) {
      if (t <= 1) return `polygon(0 0,100% 0,100% ${pct(1 - t)},0 ${pct(1 - t)})`;
      return `polygon(0 0,100% 0,100% 0,0 0)`;
    }
    if (["d", "down"].includes(d)) {
      if (t <= 1) return `polygon(0 ${pct(t)},100% ${pct(t)},100% 100%,0 100%)`;
      return `polygon(0 100%,100% 100%,100% 100%,0 100%)`;
    }
    if (t <= 1) return `polygon(0 0,${pct(1 - t)} 0,100% ${pct(t)},100% 100%,0 100%)`;
    const u = t - 1;
    return `polygon(0 ${pct(u)},0 100%,${pct(1 - u)} 100%)`;
  }

  function peelOrigin(direction) {
    const d = String(direction).toLowerCase();
    if (["r", "right"].includes(d)) return "0 0";
    if (["u", "up"].includes(d)) return "50% 100%";
    if (["d", "down"].includes(d)) return "50% 0";
    return "100% 0";
  }

  function createPeel(stage, duration, direction, underlay) {
    if (!underlay || !stage.parentElement) return null;
    const presenter = stage.parentElement;
    const { width, height, baseTransform } = stageMetrics(stage, underlay);
    const origin = peelOrigin(direction);
    const axis = ["u", "up", "d", "down"].includes(String(direction).toLowerCase()) ? "1, 0, 0" : "1, .45, 0";
    underlay.style.visibility = "hidden";
    stage.style.zIndex = "1";
    const remainder = cloneSlideSurface(underlay, "peel-rest", width, height, baseTransform);
    remainder.classList.add("page-turn-peel-sheet");
    remainder.style.zIndex = "5";
    const sheet = cloneSlideSurface(underlay, "peel-flap", width, height, "");
    sheet.classList.add("page-turn-peel-sheet");
    sheet.style.position = "absolute";
    sheet.style.left = "0";
    sheet.style.top = "0";
    sheet.style.transform = "none";
    sheet.style.zIndex = "4";
    sheet.style.transformOrigin = origin;
    const light = document.createElement("div");
    light.className = "page-turn-peel-light";
    sheet.append(light);
    const frame = document.createElement("div");
    frame.className = "page-turn-peel-frame";
    Object.assign(frame.style, {
      left: "50%", top: "50%", width: `${width}px`, height: `${height}px`,
      transform: baseTransform, perspective: `${Math.max(1200, width * 1.5).toFixed(0)}px`,
    });
    presenter.insertBefore(remainder, stage.nextSibling);
    presenter.insertBefore(frame, stage.nextSibling);
    frame.append(sheet);
    const offsets = [0, .12, .28, .46, .64, .82, 1];
    const remainderPlayer = remainder.animate(offsets.map((offset) => ({
      offset, clipPath: peelRemainderPolygon(offset, direction),
    })), { duration, easing: "linear", fill: "both" });
    const sheetPlayer = sheet.animate([
      { offset: 0, opacity: 1, transform: `rotate3d(${axis},0deg)` },
      { offset: .28, opacity: 1, transform: `rotate3d(${axis},42deg)` },
      { offset: .64, opacity: 1, transform: `rotate3d(${axis},108deg) translate3d(4%,-10%,70px)` },
      { offset: 1, opacity: 0, transform: `rotate3d(${axis},168deg) translate3d(10%,-22%,140px)` },
    ], { duration, easing: "linear", fill: "both" });
    const lightPlayer = light.animate([
      { offset: 0, opacity: 0 }, { offset: .28, opacity: .42 }, { offset: .64, opacity: .28 }, { offset: 1, opacity: 0 },
    ], { duration, easing: "linear", fill: "both" });
    const animations = [remainderPlayer, sheetPlayer, lightPlayer];
    synchronize(animations);
    return {
      player: remainderPlayer, animations,
      metrics: { quality: "shared", layers: 2, nodeCount: underlay.querySelectorAll("*").length },
      cleanup() { remainder.remove(); frame.remove(); stage.style.zIndex = "1"; },
    };
  }

  function origamiFlaps() {
    return [
      { clip: "polygon(0 0,100% 0,50% 50%)", origin: "50% 0%", rotate: "rotateX", to: 168, delay: 0 },
      { clip: "polygon(100% 0,100% 100%,50% 50%)", origin: "100% 50%", rotate: "rotateY", to: -168, delay: .08 },
      { clip: "polygon(100% 100%,0 100%,50% 50%)", origin: "50% 100%", rotate: "rotateX", to: -168, delay: .16 },
      { clip: "polygon(0 100%,0 0,50% 50%)", origin: "0 50%", rotate: "rotateY", to: 168, delay: .24 },
    ];
  }

  function createOrigami(stage, duration, underlay) {
    if (!underlay || !stage.parentElement) return null;
    const presenter = stage.parentElement;
    const { width, height, baseTransform } = stageMetrics(stage, underlay);
    underlay.style.visibility = "hidden";
    stage.style.zIndex = "1";
    const mesh = document.createElement("div");
    mesh.className = "page-turn-origami-mesh";
    Object.assign(mesh.style, {
      left: "50%", top: "50%", width: `${width}px`, height: `${height}px`,
      transform: baseTransform, perspective: `${Math.max(1400, width * 1.7).toFixed(0)}px`,
    });
    presenter.insertBefore(mesh, stage.nextSibling);
    const animations = [];
    const flaps = origamiFlaps();
    for (const flap of flaps) {
      const node = cloneSlideSurface(underlay, "origami", width, height, "");
      node.classList.add("page-turn-origami-flap");
      node.style.position = "absolute";
      node.style.left = "0";
      node.style.top = "0";
      node.style.transform = "none";
      node.style.clipPath = flap.clip;
      node.style.transformOrigin = flap.origin;
      const light = document.createElement("div");
      light.className = "page-turn-origami-light";
      node.append(light);
      mesh.append(node);
      const start = flap.delay;
      const foldAt = Math.min(.72, start + .38);
      animations.push(node.animate([
        { offset: 0, transform: `${flap.rotate}(0deg)`, opacity: 1 },
        { offset: start, transform: `${flap.rotate}(0deg)`, opacity: 1 },
        { offset: foldAt, transform: `${flap.rotate}(${flap.to}deg)`, opacity: 1 },
        { offset: 1, transform: `${flap.rotate}(${flap.to}deg) translate3d(0,-18%,80px) scale(.22)`, opacity: 0 },
      ], { duration, easing: "linear", fill: "both" }));
      animations.push(light.animate([
        { offset: 0, opacity: 0 },
        { offset: foldAt, opacity: .35 },
        { offset: 1, opacity: 0 },
      ], { duration, easing: "linear", fill: "both" }));
    }
    synchronize(animations);
    return {
      player: animations[0], animations,
      metrics: { quality: "shared", layers: 4, nodeCount: underlay.querySelectorAll("*").length },
      cleanup() { mesh.remove(); stage.style.zIndex = "1"; },
    };
  }

  function fallAxis(direction) {
    const d = String(direction).toLowerCase();
    if (["l", "left"].includes(d)) return { origin: "0 50%", rotate: "rotateY", sign: -1 };
    if (["r", "right"].includes(d)) return { origin: "100% 50%", rotate: "rotateY", sign: 1 };
    if (["u", "up"].includes(d)) return { origin: "50% 0", rotate: "rotateX", sign: -1 };
    return { origin: "50% 100%", rotate: "rotateX", sign: 1 };
  }

  function createFall(stage, duration, direction, underlay) {
    if (!underlay || !stage.parentElement) return null;
    const presenter = stage.parentElement;
    const { width, height, baseTransform } = stageMetrics(stage, underlay);
    const axis = fallAxis(direction);
    underlay.style.visibility = "hidden";
    stage.style.zIndex = "1";
    const frame = document.createElement("div");
    frame.className = "page-turn-fall-frame";
    Object.assign(frame.style, {
      left: "50%", top: "50%", width: `${width}px`, height: `${height}px`,
      transform: baseTransform, perspective: `${Math.max(1100, width * 1.35).toFixed(0)}px`,
    });
    const sheet = cloneSlideSurface(underlay, "fall", width, height, "");
    sheet.classList.add("page-turn-fall-sheet");
    sheet.style.position = "absolute";
    sheet.style.left = "0";
    sheet.style.top = "0";
    sheet.style.transform = "none";
    sheet.style.transformOrigin = axis.origin;
    const shadow = document.createElement("div");
    shadow.className = "page-turn-fall-shadow";
    Object.assign(shadow.style, {
      top: axis.rotate === "rotateX" && axis.sign > 0 ? "88%" : axis.rotate === "rotateX" ? "-6%" : "41%",
    });
    frame.append(shadow, sheet);
    presenter.insertBefore(frame, stage.nextSibling);
    const angle = (value) => `${axis.rotate}(${axis.sign * value}deg)`;
    const sheetPlayer = sheet.animate([
      { offset: 0, transform: `${angle(0)} scale(1)`, opacity: 1 },
      { offset: .42, transform: `${angle(38)} scale(1.05)`, opacity: 1 },
      { offset: .78, transform: `${angle(72)} scale(1.12)`, opacity: 1 },
      { offset: 1, transform: `${angle(88)} scale(1.16)`, opacity: 0 },
    ], { duration, easing: "linear", fill: "both" });
    const shadowPlayer = shadow.animate([
      { offset: 0, opacity: 0, transform: "scale(.6,1)" },
      { offset: .42, opacity: .28, transform: "scale(1.05,1)" },
      { offset: 1, opacity: 0, transform: "scale(1.35,1)" },
    ], { duration, easing: "linear", fill: "both" });
    const animations = [sheetPlayer, shadowPlayer];
    synchronize(animations);
    return {
      player: sheetPlayer, animations,
      metrics: { quality: "shared", layers: 1, nodeCount: underlay.querySelectorAll("*").length },
      cleanup() { frame.remove(); stage.style.zIndex = "1"; },
    };
  }

  function createSimple(stage, duration, kind, direction) {
    let keyframes;
    if (RADIAL.has(kind)) {
      const start = kind === "diamond" ? "polygon(50% 50%,50% 50%,50% 50%,50% 50%)" : "circle(0% at 50% 50%)";
      const end = kind === "diamond" ? "polygon(50% -50%,150% 50%,50% 150%,-50% 50%)" : "circle(75% at 50% 50%)";
      keyframes = [{ clipPath: start }, { clipPath: end }];
    } else if (SIMPLE_WIPES.has(kind)) {
      const hidden = ["r", "right"].includes(direction) ? "inset(0 100% 0 0)"
        : ["u", "up"].includes(direction) ? "inset(100% 0 0 0)"
          : ["d", "down"].includes(direction) ? "inset(0 0 100% 0)" : "inset(0 0 0 100%)";
      keyframes = [{ clipPath: hidden, opacity: .45 }, { clipPath: "inset(0 0 0 0)", opacity: 1 }];
    } else if (kind === "cut" || kind === "none") {
      const player = stage.animate([{ opacity: 1 }, { opacity: 1 }], { duration: 1, fill: "both" });
      return { player, animations: [player] };
    } else {
      keyframes = [{ opacity: 0, filter: kind === "zoom" ? "blur(5px)" : "none" }, { opacity: 1, filter: "none" }];
    }
    const player = stage.animate(keyframes, { duration, easing: "ease-out", fill: "both" });
    return { player, animations: [player] };
  }

  function resolveDirection(transition, kind, options = {}) {
    if (transition?.direction) return String(transition.direction).toLowerCase();
    if (kind === "wind") return "right";
    // PowerPoint's directionless pageCurlDouble turns the right-hand leaf
    // over the fixed centre spine. Mirroring by destination-page parity makes
    // the incoming half-sheet protrude outside the book and is visibly unlike
    // the native transition.
    if (kind === "pagecurldouble" || kind === "pagecurl" || kind === "peeloff") return "left";
    if (kind === "fallover") return "down";
    return "left";
  }

  function create(stage, transition, underlay = null, options = {}) {
    ensureStyle();
    const duration = Math.max(1, Number(transition?.durationMs) || 700);
    const resolvedTransition = resolveRandomTransition(transition, options);
    const kind = String(resolvedTransition?.kind || "fade").toLowerCase();
    const direction = resolveDirection(resolvedTransition, kind, options);
    const quality = ["low", "medium", "high"].includes(options.quality) ? options.quality : "high";
    if (!SHARED_KINDS.has(kind)) return null;
    if (kind === "pagecurldouble") return createBook(stage, duration, direction, underlay) || createSimple(stage, duration, "fade", direction);
    if (kind === "pagecurl") return createPageCurl(stage, duration, direction, underlay, quality) || createSimple(stage, duration, "fade", direction);
    if (kind === "peeloff") return createPeel(stage, duration, direction, underlay) || createSimple(stage, duration, "fade", direction);
    if (kind === "origami") return createOrigami(stage, duration, underlay) || createSimple(stage, duration, "fade", direction);
    if (kind === "fallover") return createFall(stage, duration, direction, underlay) || createSimple(stage, duration, "fade", direction);
    if (["ripple", "honeycomb", "glitter", "prism"].includes(kind)) return createRipple(stage, duration, kind, underlay, quality) || createSimple(stage, duration, kind, direction);
    if (["wind", "warp", "vortex"].includes(kind)) return createWind(stage, duration, direction, underlay, quality) || createSimple(stage, duration, "wipe", direction);
    if (["curtains", "drape", "doors"].includes(kind)) {
      const player = stage.animate([
        { clipPath: "inset(0 50% 0 50%)", filter: "brightness(.55)" },
        { offset: .48, filter: kind === "drape" ? "brightness(.72) contrast(1.15)" : "brightness(.86)" },
        { clipPath: "inset(0 0 0 0)", filter: "brightness(1)" },
      ], { duration, easing: "cubic-bezier(.3,.06,.16,1)", fill: "both" });
      return { player, animations: [player] };
    }
    return createSimple(stage, duration, kind, direction);
  }

  root.UniPptPresentationTransitions = Object.freeze({
    supports(kind) { return SHARED_KINDS.has(String(kind || "fade").toLowerCase()); },
    captureUnderlay,
    create,
    synchronize,
    windState,
    projectiveTransform,
    resolveRandomTransition,
    resolveDirection,
    curlGeometryAt,
    peelRemainderPolygon,
    origamiFlaps,
  });
})(globalThis);
