(() => {
  "use strict";

  const EFFECTS = new Set(["randomBars", "dissolve", "wheel", "circle", "split", "wipe"]);

  function maskFrame(maskImage, maskPosition, maskSize, offset) {
    return {
      offset,
      opacity: 1,
      maskImage,
      webkitMaskImage: maskImage,
      maskPosition,
      webkitMaskPosition: maskPosition,
      maskSize,
      webkitMaskSize: maskSize,
      maskRepeat: "no-repeat",
      webkitMaskRepeat: "no-repeat",
      maskComposite: "add",
      webkitMaskComposite: "source-over",
    };
  }

  function rankedMaskFrames({ count, steps, positions, shownSize, hiddenSize, rank, layerImage }) {
    const image = Array(count).fill(layerImage || "linear-gradient(#000 0 0)").join(",");
    return Array.from({ length: steps + 1 }, (_, step) => {
      const visible = Math.ceil((step / steps) * count);
      const sizes = Array.from({ length: count }, (_, index) =>
        rank(index) < visible ? shownSize(index) : hiddenSize,
      ).join(",");
      return maskFrame(image, positions, sizes, step / steps);
    });
  }

  function randomBarsFrames(direction = "horizontal") {
    const vertical = String(direction).toLowerCase().includes("vertical");
    // PowerPoint uses many narrow, independently staggered strips.  A small
    // number of hard strips reads as "blinds", especially on photographs.
    const count = 32;
    const stripSpan = (100 / count) + 0.85;
    const positions = Array.from({ length: count }, (_, index) => {
      const position = `${(index * 100) / (count - 1)}%`;
      return vertical ? `${position} 0` : `0 ${position}`;
    }).join(",");
    return rankedMaskFrames({
      count,
      steps: 16,
      positions,
      shownSize: () => (vertical ? `${stripSpan.toFixed(3)}% 100%` : `100% ${stripSpan.toFixed(3)}%`),
      hiddenSize: vertical ? "0% 100%" : "100% 0%",
      layerImage: vertical
        ? "linear-gradient(to right,transparent 0,#000 14%,#000 86%,transparent 100%)"
        : "linear-gradient(to bottom,transparent 0,#000 14%,#000 86%,transparent 100%)",
      // A full-period deterministic permutation gives stable preview and export.
      rank: (index) => ((index * 21) + 7) % count,
    });
  }

  function dissolveFrames() {
    const columns = 8;
    const rows = 6;
    const count = columns * rows;
    const positions = Array.from({ length: count }, (_, index) => {
      const column = index % columns;
      const row = Math.floor(index / columns);
      return `${(column * 100) / (columns - 1)}% ${(row * 100) / (rows - 1)}%`;
    }).join(",");
    return rankedMaskFrames({
      count,
      steps: 12,
      positions,
      shownSize: () => "12.75% 17%",
      hiddenSize: "0% 0%",
      rank: (index) => (index * 29) % count,
    });
  }

  function wheelFrames(direction = "1") {
    const spokes = Math.max(1, Math.min(8, Number.parseInt(direction, 10) || 1));
    const frameCount = 16;
    return Array.from({ length: frameCount + 1 }, (_, frameIndex) => {
      const progress = frameIndex / frameCount;
      const sector = 360 / spokes;
      const reveal = sector * progress;
      let mask;
      if (progress === 0) {
        mask = "linear-gradient(transparent 0 0)";
      } else if (progress === 1) {
        mask = "linear-gradient(#000 0 0)";
      } else {
        const feather = Math.min(1.4, Math.max(0.18, reveal * 0.2));
        const solidEnd = Math.max(0, reveal - feather);
        const softMiddle = solidEnd + (feather * 0.52);
        // CSS conic gradients start at twelve o'clock and advance clockwise,
        // matching the PowerPoint Wheel preset.  The old -90deg origin began
        // at nine o'clock and made every ring visibly a quarter-turn late.
        mask = `repeating-conic-gradient(from 0deg,#000 0deg ${solidEnd.toFixed(3)}deg,rgba(0,0,0,.48) ${softMiddle.toFixed(3)}deg,transparent ${reveal.toFixed(3)}deg ${sector.toFixed(3)}deg)`;
      }
      return maskFrame(mask, "center", "100% 100%", progress);
    });
  }

  function circleFrames(direction = "in") {
    const inward = String(direction).toLowerCase() !== "out";
    const frameCount = 16;
    return Array.from({ length: frameCount + 1 }, (_, index) => {
      const progress = index / frameCount;
      let maskImage;
      if (progress === 0) {
        maskImage = "linear-gradient(transparent 0 0)";
      } else if (progress === 1) {
        maskImage = "linear-gradient(#000 0 0)";
      } else if (inward) {
        const radius = 100 * (1 - progress);
        const softEnd = Math.min(100, radius + 0.8);
        maskImage = `radial-gradient(circle farthest-corner at 50% 50%,transparent 0 ${radius.toFixed(3)}%,rgba(0,0,0,.48) ${(radius + ((softEnd - radius) * 0.52)).toFixed(3)}%,#000 ${softEnd.toFixed(3)}% 100%)`;
      } else {
        const radius = 100 * progress;
        const solidEnd = Math.max(0, radius - 0.8);
        maskImage = `radial-gradient(circle farthest-corner at 50% 50%,#000 0 ${solidEnd.toFixed(3)}%,rgba(0,0,0,.48) ${(solidEnd + ((radius - solidEnd) * 0.52)).toFixed(3)}%,transparent ${radius.toFixed(3)}% 100%)`;
      }
      return maskFrame(maskImage, "0 0", "100% 100%", progress);
    });
  }

  function wipeFrames(direction = "left") {
    const normalized = String(direction).toLowerCase();
    const gradientDirection = normalized.includes("right")
      ? "to right"
      : normalized.includes("up")
        ? "to top"
        : normalized.includes("down")
          ? "to bottom"
          : "to left";
    const frameCount = 14;
    return Array.from({ length: frameCount + 1 }, (_, index) => {
      const progress = index / frameCount;
      let maskImage;
      if (progress === 0) {
        maskImage = "linear-gradient(transparent 0 0)";
      } else if (progress === 1) {
        maskImage = "linear-gradient(#000 0 0)";
      } else {
        const edge = progress * 100;
        const solidEnd = Math.max(0, edge - 0.7);
        const softEnd = Math.min(100, edge + 0.7);
        maskImage = `linear-gradient(${gradientDirection},#000 0 ${solidEnd.toFixed(3)}%,rgba(0,0,0,.48) ${edge.toFixed(3)}%,transparent ${softEnd.toFixed(3)}% 100%)`;
      }
      return maskFrame(maskImage, "0 0", "100% 100%", progress);
    });
  }

  function splitFrames(direction = "outHorizontal") {
    const normalized = String(direction).toLowerCase();
    const vertical = normalized.includes("vertical");
    const frameCount = 8;
    return Array.from({ length: frameCount + 1 }, (_, index) => {
      const progress = index / frameCount;
      const hidden = 50 * (1 - progress);
      return {
        offset: progress,
        opacity: 1,
        clipPath: vertical
          ? `inset(0 ${hidden.toFixed(3)}% 0 ${hidden.toFixed(3)}%)`
          : `inset(${hidden.toFixed(3)}% 0 ${hidden.toFixed(3)}% 0)`,
      };
    });
  }

  const PROPERTY_NAMES = new Set([
    "ppt_x", "ppt_y", "ppt_w", "ppt_h", "ppt_r", "r", "rotation", "style.rotation",
  ]);

  function tokenizeExpression(source) {
    if (typeof source !== "string" || source.length > 512) return null;
    const tokens = source.match(/#[A-Za-z_][\w]*|style\.rotation|[A-Za-z_][\w]*|\$|(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?|[()+\-*/,]/gi);
    if (!tokens?.length) return null;
    const residue = source.replace(/#[A-Za-z_][\w]*|style\.rotation|[A-Za-z_][\w]*|\$|(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?|[()+\-*/,]|\s+/gi, "");
    return residue ? null : tokens;
  }

  function evaluateExpression(source, variables = {}, progress = 0) {
    const tokens = tokenizeExpression(String(source ?? ""));
    if (!tokens) return null;
    let index = 0;
    const primary = () => {
      const token = tokens[index++];
      if (token == null) return null;
      if (token === "(") {
        const value = expression();
        if (tokens[index++] !== ")") return null;
        return value;
      }
      if (token === "+" || token === "-") {
        const value = primary();
        return value == null ? null : token === "-" ? -value : value;
      }
      if (/^[-+]?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?$/i.test(token)) {
        const value = Number(token);
        return Number.isFinite(value) ? value : null;
      }
      if (token === "$") return progress;
      if (token.toLowerCase() === "pi") return Math.PI;
      if (tokens[index] === "(") {
        index += 1;
        const args = [];
        if (tokens[index] !== ")") {
          while (index < tokens.length) {
            const value = expression();
            if (value == null) return null;
            args.push(value);
            if (tokens[index] !== ",") break;
            index += 1;
          }
        }
        if (tokens[index++] !== ")") return null;
        const fn = ({ sin: Math.sin, cos: Math.cos, tan: Math.tan, abs: Math.abs,
          min: Math.min, max: Math.max })[token.toLowerCase()];
        if (!fn) return null;
        const result = fn(...args);
        return Number.isFinite(result) ? result : null;
      }
      const value = variables[token] ?? variables[token.replace(/^#/, "")];
      return Number.isFinite(value) ? value : null;
    };
    const term = () => {
      let value = primary();
      while (value != null && (tokens[index] === "*" || tokens[index] === "/")) {
        const operator = tokens[index++];
        const right = primary();
        if (right == null || (operator === "/" && right === 0)) return null;
        value = operator === "*" ? value * right : value / right;
      }
      return value;
    };
    const expression = () => {
      let value = term();
      while (value != null && (tokens[index] === "+" || tokens[index] === "-")) {
        const operator = tokens[index++];
        const right = term();
        if (right == null) return null;
        value = operator === "+" ? value + right : value - right;
      }
      return value;
    };
    const value = expression();
    return index === tokens.length && Number.isFinite(value) ? value : null;
  }

  function propertyContext(context = {}) {
    context ||= {};
    const node = context.node;
    if (!node) return null;
    const slideWidth = Number(context.slideWidth) || 1280;
    const slideHeight = Number(context.slideHeight) || 720;
    const read = (name, fallback) => {
      const inline = Number.parseFloat(node.style?.[name]);
      return Number.isFinite(inline) ? inline : Number(node[`offset${name[0].toUpperCase()}${name.slice(1)}`]) || fallback;
    };
    const left = read("left", 0);
    const top = read("top", 0);
    const width = Math.max(0.000001, read("width", 0.000001));
    const height = Math.max(0.000001, read("height", 0.000001));
    const pptW = width / slideWidth;
    const pptH = height / slideHeight;
    const variables = {
      "#ppt_x": (left + width / 2) / slideWidth,
      "#ppt_y": (top + height / 2) / slideHeight,
      "#ppt_w": pptW,
      "#ppt_h": pptH,
      "#ppt_r": 0,
      ppt_x: (left + width / 2) / slideWidth,
      ppt_y: (top + height / 2) / slideHeight,
      ppt_w: pptW,
      ppt_h: pptH,
      ppt_r: 0,
      r: 0,
      rotation: 0,
      "style.rotation": 0,
    };
    return { slideWidth, slideHeight, variables, baseTransform: context.baseTransform || "" };
  }

  function resolvePropertyContext(animation, context) {
    if (context?.node || typeof document === "undefined") return context;
    const targetId = animation?.targetObjectId;
    if (!targetId) return context;
    const node = [...document.querySelectorAll("[data-id]")]
      .find((candidate) => candidate.dataset?.id === targetId);
    if (!node) return context;
    const stage = node.closest?.("#stage") || node.parentElement;
    return {
      node,
      slideWidth: Number.parseFloat(stage?.style?.width) || stage?.clientWidth || 1280,
      slideHeight: Number.parseFloat(stage?.style?.height) || stage?.clientHeight || 720,
      baseTransform: node.dataset?.animationBase || node.dataset?.base || node.style?.transform || "",
    };
  }

  function behaviorKeyframes(behavior, attribute, variables) {
    const native = Array.isArray(behavior.keyframes) ? behavior.keyframes : [];
    if (native.length) {
      return native.map((frame) => ({
        offset: Math.max(0, Math.min(1, (Number(frame.time) || 0) / 100000)),
        expression: frame.value,
        formula: frame.formula || null,
      })).sort((left, right) => left.offset - right.offset);
    }
    const base = variables[`#${attribute}`] ?? variables[attribute] ?? 0;
    const from = behavior.from ?? String(base);
    let to = behavior.to;
    if (to == null && behavior.by != null) {
      const delta = evaluateExpression(behavior.by, variables, 1);
      to = delta == null ? String(base) : String(base + delta);
    }
    return [{ offset: 0, expression: from, formula: null }, {
      offset: 1, expression: to ?? String(base), formula: null,
    }];
  }

  function propertyValueAt(keyframes, offset, variables) {
    if (!keyframes.length) return null;
    let left = keyframes[0];
    let right = keyframes.at(-1);
    for (let index = 1; index < keyframes.length; index += 1) {
      if (offset <= keyframes[index].offset) {
        right = keyframes[index];
        left = keyframes[index - 1];
        break;
      }
    }
    const span = Math.max(0.000001, right.offset - left.offset);
    const progress = Math.max(0, Math.min(1, (offset - left.offset) / span));
    if (left.formula) return evaluateExpression(left.formula, variables, progress);
    const from = evaluateExpression(left.expression, variables, progress);
    const to = evaluateExpression(right.expression, variables, progress);
    if (from == null) return to;
    if (to == null) return from;
    return from + ((to - from) * progress);
  }

  function propertyAnimationFrames(animation, context, extraOffsets = []) {
    const environment = propertyContext(resolvePropertyContext(animation, context));
    const behaviors = Array.isArray(animation?.propertyAnimations)
      ? animation.propertyAnimations.filter((behavior) => behavior?.attributes?.some((name) => PROPERTY_NAMES.has(name)))
      : [];
    if (!environment || !behaviors.length) return null;
    const tracks = [];
    const offsets = new Set([0, 1, ...extraOffsets]);
    for (const behavior of behaviors) {
      for (const attribute of behavior.attributes) {
        if (!PROPERTY_NAMES.has(attribute)) continue;
        const keyframes = behaviorKeyframes(behavior, attribute, environment.variables);
        keyframes.forEach((frame) => offsets.add(frame.offset));
        if (keyframes.some((frame) => frame.formula)) {
          for (let index = 1; index < 16; index += 1) offsets.add(index / 16);
        }
        tracks.push({ attribute, keyframes });
      }
    }
    const ordered = [...offsets].filter(Number.isFinite).sort((left, right) => left - right);
    // Native `p:animEffect filter="fade"` rides along most property presets
    // (float in/out, fold, fade-zoom).  Ramp that fade into the sampled frames
    // instead of flattening every offset to full opacity.
    const fade = animation?.fadeFilter === "out" ? "out" : animation?.fadeFilter ? "in" : null;
    return ordered.map((offset) => {
      let x = environment.variables["#ppt_x"];
      let y = environment.variables["#ppt_y"];
      let width = environment.variables["#ppt_w"];
      let height = environment.variables["#ppt_h"];
      let rotation = 0;
      for (const track of tracks) {
        const value = propertyValueAt(track.keyframes, offset, environment.variables);
        if (value == null) continue;
        if (track.attribute === "ppt_x") x = value;
        else if (track.attribute === "ppt_y") y = value;
        else if (track.attribute === "ppt_w") width = value;
        else if (track.attribute === "ppt_h") height = value;
        else rotation = value;
      }
      const deltaX = (x - environment.variables["#ppt_x"]) * environment.slideWidth;
      const deltaY = (y - environment.variables["#ppt_y"]) * environment.slideHeight;
      const scaleX = width / environment.variables["#ppt_w"];
      const scaleY = height / environment.variables["#ppt_h"];
      const opacity = fade === "out" ? 1 - offset : fade === "in" ? offset : 1;
      return {
        offset,
        opacity,
        transform: `${environment.baseTransform} translate(${deltaX}px,${deltaY}px) rotate(${rotation}deg) scale(${scaleX},${scaleY})`,
      };
    });
  }

  function presetFrames(animation) {
    switch (animation.effect) {
      case "randomBars": return randomBarsFrames(animation.direction);
      case "dissolve": return dissolveFrames();
      case "wheel": return wheelFrames(animation.direction);
      case "circle": return circleFrames(animation.direction);
      case "split": return splitFrames(animation.direction);
      case "wipe": return wipeFrames(animation.direction);
      default: return null;
    }
  }

  function frameAt(track, offset) {
    if (!track?.length) return {};
    let selected = track[0];
    for (const frame of track) {
      if ((frame.offset ?? 0) > offset) break;
      selected = frame;
    }
    return selected;
  }

  function frames(animation, context = null) {
    if (!animation) return null;
    const preset = EFFECTS.has(animation.effect) ? presetFrames(animation) : null;
    const properties = propertyAnimationFrames(
      animation,
      context,
      preset?.map((frame) => frame.offset ?? 0) || [],
    );
    if (!properties) return preset;
    if (!preset) return properties;
    return properties.map((property) => ({ ...frameAt(preset, property.offset), ...property }));
  }

  globalThis.UniPptPresetAnimation = Object.freeze({
    frames,
    randomBarsFrames,
    dissolveFrames,
    wheelFrames,
    circleFrames,
    splitFrames,
    wipeFrames,
    evaluateExpression,
    propertyAnimationFrames,
  });
})();
