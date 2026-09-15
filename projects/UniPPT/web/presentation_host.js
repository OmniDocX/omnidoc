"use strict";

/**
 * UniPPT AI-first document host.
 *
 * The host is deliberately independent from the editor DOM. It provides a
 * versioned, atomic ChangeSet protocol over the semantic Deck model, a small
 * validation layer, a tool catalogue suitable for an AI agent, and a safe
 * plugin registry. Native PPTX/OPC details remain owned by the server-side
 * loss-aware exporter and cannot be overwritten through this API.
 */
(function installPresentationHost(global) {
  const HOST_VERSION = "2.0";
  const DOCUMENT_TYPE = "pptx";
  const AI_EXTENSION = "org.unippt.ai";
  const MAX_OPERATIONS = 512;
  const AI_MUTATION_CAPABILITY_TTL_MS = 5 * 60 * 1000;
  const AI_READ_ONLY_TOOLS = new Set(["presentation.inspect", "presentation.validate"]);
  const FORBIDDEN_PATCH_KEYS = new Set([
    "id", "sourceImportId", "sourcePartName", "sourceShapeId", "sourceTimingId",
    "sourceTimingXml", "sourceTransitionXml", "targetShapeId",
  ]);

  class HostError extends Error {
    constructor(code, message, details = null) {
      super(message);
      this.name = "UniPptHostError";
      this.code = code;
      this.details = details;
    }
  }

  function clone(value, seen = new WeakMap()) {
    if (value === null || typeof value !== "object") return value;
    if (seen.has(value)) return seen.get(value);
    if (typeof Blob !== "undefined" && value instanceof Blob) return value;
    if (value instanceof ArrayBuffer) return value.slice(0);
    if (ArrayBuffer.isView(value)) return value.slice?.() || value;
    const output = Array.isArray(value) ? [] : {};
    seen.set(value, output);
    for (const key of Object.keys(value)) output[key] = clone(value[key], seen);
    return output;
  }

  function finite(value, fallback = 0) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  function utf8Bytes(value) {
    return new TextEncoder().encode(String(value || "")).byteLength;
  }

  function weightedGlyphCount(value) {
    return Math.max(1, Array.from(String(value || "")).reduce((sum, character) => {
      if (/\s/.test(character)) return sum + .34;
      if (character.charCodeAt(0) > 0x2e7f) return sum + .98;
      if (/\p{L}|\p{N}/u.test(character)) return sum + .62;
      return sum + .46;
    }, 0));
  }

  function uid(prefix, options) {
    if (typeof options?.idFactory === "function") return String(options.idFactory(prefix));
    const random = global.crypto?.randomUUID?.() || `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
    return `${prefix}-${random}`;
  }

  function ensureDeck(deck) {
    if (!deck || typeof deck !== "object" || !Array.isArray(deck.slides)) {
      throw new HostError("INVALID_DECK", "演示文稿缺少 slides 场景数组");
    }
    return deck;
  }

  function extension(deck) {
    deck.extensions ||= {};
    const metadata = deck.extensions[AI_EXTENSION] ||= {};
    metadata.schemaVersion = 1;
    metadata.document ||= { summary: "", language: null };
    metadata.designTokens ||= {};
    metadata.slides ||= {};
    metadata.objects ||= {};
    return metadata;
  }

  function inferObjectRole(object) {
    const name = String(object?.name || "").toLowerCase();
    const text = String(object?.text || "").trim();
    if (/标题|title/.test(name)) return name.includes("副") || /sub/.test(name) ? "slide-subtitle" : "slide-title";
    if (object?.kind === "chart") return "data-visualization";
    if (object?.kind === "table") return "data-table";
    if (object?.kind === "math") return "formula";
    if (object?.media?.kind === "video") return "video";
    if (object?.media?.kind === "audio") return "audio";
    if (object?.kind === "image") return text ? "illustration-with-caption" : "illustration";
    if (object?.kind === "text") return text.length <= 48 ? "heading" : "body-text";
    if (object?.kind === "group") return "composite";
    return text ? "label" : "decorative";
  }

  function visitObjects(objects, callback, parent = null) {
    for (let index = 0; index < (objects || []).length; index += 1) {
      const object = objects[index];
      callback(object, { parent, objects, index });
      visitObjects(object.children || [], callback, object);
    }
  }

  function enrichSemantics(deck, options = {}) {
    ensureDeck(deck);
    const metadata = extension(deck);
    const slideIds = new Set();
    const objectIds = new Set();
    deck.slides.forEach((slide, slideIndex) => {
      if (!slide.id || slideIds.has(slide.id)) slide.id = uid("slide", options);
      slideIds.add(slide.id);
      metadata.slides[slide.id] ||= {
        role: slideIndex === 0 ? "cover" : "content",
        summary: "",
      };
      visitObjects(slide.objects || [], (object) => {
        if (!object.id || objectIds.has(object.id)) object.id = uid("shape", options);
        objectIds.add(object.id);
        metadata.objects[object.id] ||= {
          role: inferObjectRole(object),
          label: String(object.name || ""),
          description: "",
          styleRef: null,
          constraints: {},
        };
      });
    });
    for (const id of Object.keys(metadata.slides)) if (!slideIds.has(id)) delete metadata.slides[id];
    for (const id of Object.keys(metadata.objects)) if (!objectIds.has(id)) delete metadata.objects[id];
    return deck;
  }

  function deepMerge(target, patch, path = "") {
    if (!patch || typeof patch !== "object" || Array.isArray(patch)) return clone(patch);
    const output = target && typeof target === "object" && !Array.isArray(target) ? target : {};
    for (const [key, value] of Object.entries(patch)) {
      if (FORBIDDEN_PATCH_KEYS.has(key)) {
        throw new HostError("PROTECTED_NATIVE_FIELD", `事务不能修改原生绑定字段 ${path}${key}`);
      }
      output[key] = value && typeof value === "object" && !Array.isArray(value)
        ? deepMerge(output[key], value, `${path}${key}.`)
        : clone(value);
    }
    return output;
  }

  function makeTextParagraph(text, style = {}) {
    return [{
      align: style.align || "left",
      level: 0,
      bullet: null,
      numbering: null,
      lineSpacing: 1.18,
      spaceBefore: 0,
      spaceAfter: 0,
      runs: [{
        text: String(text || ""),
        fontFamily: style.fontFamily || "Aptos, Microsoft YaHei, sans-serif",
        nativeFontFamily: style.nativeFontFamily || null,
        nativeFonts: clone(style.nativeFonts || {}),
        fontSize: finite(style.fontSize, 28),
        color: style.color || "#172033",
        bold: Boolean(style.bold),
        italic: Boolean(style.italic),
        underline: false,
        strikethrough: false,
        // `RichTextRun.baseline` is a required UDOC string.  `null` looks
        // harmless in the browser, but makes every server-side export fail
        // while deserializing the deck.  Keep the explicit neutral value so
        // newly generated AI/OCR text is immediately exportable.
        baseline: "normal",
        hyperlinks: { click: null, hover: null },
      }],
    }];
  }

  function makeObject(input = {}, options = {}) {
    const kind = String(input.kind || "text");
    const textStyle = {
      fontFamily: "Aptos, Microsoft YaHei, sans-serif",
      nativeFontFamily: null,
      fontSize: kind === "text" ? 28 : 20,
      color: "#172033",
      bold: false,
      italic: false,
      align: "left",
      ...(clone(input.textStyle || {})),
    };
    // Model-authored tool arguments can contain explicit nulls even for
    // schema fields that are optional-but-non-null when present.  Defaults
    // applied before the spread above are otherwise overwritten by null and
    // the resulting deck cannot be deserialized by native exporters.
    textStyle.fontFamily = String(textStyle.fontFamily || "Aptos, Microsoft YaHei, sans-serif");
    textStyle.color = String(textStyle.color || "#172033");
    textStyle.align = String(textStyle.align || "left");
    textStyle.fontSize = Math.max(1, finite(textStyle.fontSize, kind === "text" ? 28 : 20));
    for (const key of ["bold", "italic"]) textStyle[key] ??= false;
    const text = String(input.text || "");
    const textParagraphs = input.textParagraphs
      ? clone(input.textParagraphs).map((paragraph) => ({
          ...paragraph,
          align: String(paragraph?.align || textStyle.align || "left"),
          runs: (paragraph?.runs || []).map((run) => ({
            ...run,
            text: String(run?.text || ""),
            fontFamily: String(run?.fontFamily || textStyle.fontFamily),
            fontSize: Math.max(1, finite(run?.fontSize, textStyle.fontSize)),
            color: String(run?.color || textStyle.color),
            bold: run?.bold ?? textStyle.bold,
            italic: run?.italic ?? textStyle.italic,
            baseline: String(run?.baseline || "normal"),
          })),
        }))
      : (text ? makeTextParagraph(text, textStyle) : []);
    const textFrame = { marginLeft: 8, marginRight: 8, marginTop: 5, marginBottom: 5, verticalAlign: "center", verticalType: "horz", wordWrap: true, autoSize: "none", ...(clone(input.textFrame || {})) };
    if (textFrame.autoSize === "shrinkText") textFrame.autoSize = "textToFitShape";
    return {
      id: input.id || uid("shape", options),
      sourceShapeId: null,
      name: String(input.name || ({ text: "文本框", shape: "形状", image: "图片", chart: "图表", table: "表格", math: "公式" }[kind] || "对象")),
      kind,
      frame: {
        x: finite(input.frame?.x, 120), y: finite(input.frame?.y, 120),
        width: Math.max(1, finite(input.frame?.width, 420)),
        height: Math.max(1, finite(input.frame?.height, 120)),
        rotation: finite(input.frame?.rotation, 0),
      },
      flipH: Boolean(input.flipH), flipV: Boolean(input.flipV),
      text,
      textParagraphs,
      textFrame,
      geometry: input.geometry ?? null, customGeometry: clone(input.customGeometry ?? null),
      asset: input.asset ?? null,
      imageCrop: { left: 0, top: 0, right: 0, bottom: 0, ...(clone(input.imageCrop || {})) },
      imageFillRect: { left: 0, top: 0, right: 0, bottom: 0, ...(clone(input.imageFillRect || {})) },
      imageEffects: { duotone: null, softEdgeRadius: 0, ...(clone(input.imageEffects || {})) },
      shapeFillAsset: input.shapeFillAsset ?? null,
      formula: clone(input.formula ?? null), media: clone(input.media ?? null),
      table: clone(input.table ?? null), chart: clone(input.chart ?? null),
      hyperlinks: { click: null, hover: null, ...(clone(input.hyperlinks || {})) },
      style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: 1, gradient: null, strokeDash: null, shadow: null, ...(clone(input.style || {})) },
      textStyle,
      children: (input.children || []).map((child) => makeObject(child, options)),
    };
  }

  function makeAnimation(input = {}, slide, options = {}, fallbackTargetObjectId = null) {
    const targetIndex = Number.isInteger(input.targetIndex) ? input.targetIndex : null;
    let targetObjectId = fallbackTargetObjectId || input.targetObjectId || null;
    if (!targetObjectId && targetIndex != null) targetObjectId = slide.objects[targetIndex]?.id || null;
    if (!targetObjectId && input.targetName) {
      visitObjects(slide.objects || [], (object) => {
        if (!targetObjectId && object.name === input.targetName) targetObjectId = object.id;
      });
    }
    const animation = {
      id: uid("anim", options), sourceTimingId: null,
      targetObjectId, targetShapeId: null,
      effect: input.effect || "fade", class: input.class || "entrance",
      trigger: input.trigger || "onClick",
      durationMs: Math.max(1, finite(input.durationMs, 500)),
      delayMs: Math.max(0, finite(input.delayMs, 0)),
      acceleration: null, deceleration: null, speed: null, timeFilter: null,
      repeatCount: null, repeatDurationMs: null, autoReverse: false,
      order: slide.animations.length, presetId: null, presetSubtype: null,
      direction: input.direction ?? null, motionPath: input.motionPath ?? null,
      propertyAnimations: [], mediaAction: input.mediaAction ?? null,
    };
    const patch = clone(input || {});
    for (const key of ["id", "sourceTimingId", "targetShapeId", "targetIndex", "targetName"]) delete patch[key];
    deepMerge(animation, patch);
    animation.id = animation.id || uid("anim", options);
    animation.sourceTimingId = null;
    animation.targetShapeId = null;
    animation.targetObjectId = targetObjectId;
    animation.order = slide.animations.length;
    return animation;
  }

  function makeSlide(input = {}, deck, options = {}) {
    const width = finite(deck?.width, 1280);
    const height = finite(deck?.height, 720);
    const slide = {
      id: input.id || uid("slide", options), sourcePartName: null,
      name: String(input.name || input.title || `幻灯片 ${(deck?.slides?.length || 0) + 1}`),
      background: input.background || "#ffffff", backgroundAsset: input.backgroundAsset || null,
      notes: String(input.notes || ""), masterObjects: [], layoutObjects: [], inheritedAnimations: [],
      objects: [], animations: [], transition: clone(input.transition ?? null),
      sourceTimingXml: null, sourceTransitionXml: null,
    };
    if (Array.isArray(input.objects) && input.objects.length) {
      slide.objects = input.objects.map((object) => makeObject(object, options));
    } else if (input.title || input.subtitle || input.bullets) {
      if (input.title) slide.objects.push(makeObject({ name: "标题", text: input.title, frame: { x: width * .075, y: height * .09, width: width * .85, height: height * .16 }, textStyle: { fontSize: 48, bold: true, align: "left" } }, options));
      if (input.subtitle) slide.objects.push(makeObject({ name: "副标题", text: input.subtitle, frame: { x: width * .08, y: height * .28, width: width * .84, height: height * .12 }, textStyle: { fontSize: 26, color: "#526078" } }, options));
      if (Array.isArray(input.bullets) && input.bullets.length) slide.objects.push(makeObject({ name: "内容", text: input.bullets.map((item) => `• ${item}`).join("\n"), frame: { x: width * .1, y: height * .35, width: width * .8, height: height * .48 }, textStyle: { fontSize: 28 } }, options));
    }
    for (const animation of input.animations || []) slide.animations.push(makeAnimation(animation, slide, options));
    return slide;
  }

  const AI_DECK_PRESETS = Object.freeze({
    "midnight-consulting": Object.freeze({
      background: "#0A1128", panel: "#10233D", foreground: "#F8F9FA",
      muted: "#94A3B8", primary: "#00DDF5", accent: "#FF6B35",
      rule: "#1C3A5B", fontFamily: "Noto Sans SC, Microsoft YaHei, sans-serif",
    }),
    "clean-executive": Object.freeze({
      background: "#F6F7FB", panel: "#FFFFFF", foreground: "#172033",
      muted: "#667085", primary: "#1565C0", accent: "#E85D3F",
      rule: "#D9DFEA", fontFamily: "Noto Sans SC, Microsoft YaHei, sans-serif",
    }),
  });

  function composedFrame(deck, x, y, width, height) {
    return {
      x: x / 1280 * deck.width, y: y / 720 * deck.height,
      width: width / 1280 * deck.width, height: height / 720 * deck.height,
    };
  }

  function composedText(deck, palette, name, text, x, y, width, height, style = {}) {
    return {
      kind: "text", name, text: String(text || ""), frame: composedFrame(deck, x, y, width, height),
      textStyle: {
        fontFamily: palette.fontFamily, fontSize: 18, color: palette.foreground,
        align: "left", ...style,
      },
      textFrame: {
        marginLeft: 0, marginRight: 0, marginTop: 0, marginBottom: 0,
        verticalAlign: style.verticalAlign || "top", verticalType: "horz",
        wordWrap: style.wordWrap !== false, autoSize: "none",
      },
      style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: 1 },
    };
  }

  function composedShape(deck, name, x, y, width, height, style = {}, geometry = "rect") {
    return {
      kind: "shape", name, frame: composedFrame(deck, x, y, width, height), geometry,
      style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: 1, ...style },
    };
  }

  function composedSourceNotes(notes, sources) {
    const normalized = (sources || []).map((source) => {
      if (typeof source === "string") return source.trim();
      const label = String(source?.label || source?.title || "来源").trim();
      const url = String(source?.url || source?.href || "").trim();
      return url ? `${label} — ${url}` : label;
    }).filter(Boolean);
    if (!normalized.length) return String(notes || "");
    return `${String(notes || "").trim()}${notes ? "\n\n" : ""}[Sources]\n${normalized.map((source) => `- ${source}`).join("\n")}\n[/Sources]`;
  }

  function composedChrome(deck, palette, input, index, slideCount) {
    const section = String(input.section || input.eyebrow || `${String(index).padStart(2, "0")} / INSIGHT`).toUpperCase();
    const objects = [
      composedShape(deck, "top-rule", 60, 58, 1160, 1, { fill: palette.primary, opacity: .72 }),
      composedText(deck, palette, "section-label", section, 60, 82, 360, 24, { fontSize: 13, color: palette.primary, wordWrap: false }),
      composedText(deck, palette, "slide-title", input.title, 60, 116, 1120, 58, { fontSize: 36, color: palette.foreground, wordWrap: false }),
      composedShape(deck, "title-accent", 60, 183, 72, 5, { fill: palette.accent }),
      composedText(deck, palette, "slide-number", String(index).padStart(2, "0"), 1160, 657, 60, 20, { fontSize: 12, color: palette.muted, align: "right", wordWrap: false }),
    ];
    const sourceLabel = (input.sources || []).map((source) => typeof source === "string" ? source : source?.label || source?.title).filter(Boolean).join("；");
    if (sourceLabel) objects.push(composedText(deck, palette, "source-note", `来源：${sourceLabel}`, 60, 657, 900, 20, { fontSize: 10, color: palette.muted, wordWrap: false }));
    if (index === slideCount) objects.push(composedShape(deck, "closing-corner", 1180, 82, 32, 1, { fill: palette.primary }));
    return objects;
  }

  function normalizedItems(value, maximum = 5) {
    return (Array.isArray(value) ? value : []).filter((item) => item != null).slice(0, maximum);
  }

  function composedChart(deck, palette, chart, frame = [742, 258, 430, 286]) {
    const chartType = String(chart?.chartType || "bar");
    return {
      kind: "chart", name: String(chart?.title || "数据图表"), frame: composedFrame(deck, ...frame),
      chart: {
        relationshipId: null, sourcePartName: null, chartType,
        title: String(chart?.title || ""),
        legend: { visible: Boolean(chart?.legend), position: "right", overlay: false },
        categories: normalizedItems(chart?.categories, 12).map(String),
        series: normalizedItems(chart?.series, 6).map((series, seriesIndex) => ({
          sourceIndex: null, name: String(series?.name || `系列 ${seriesIndex + 1}`),
          values: normalizedItems(series?.values, 12).map((value) => Number.isFinite(Number(value)) ? Number(value) : null),
          color: String(series?.color || [palette.primary, palette.accent, palette.muted][seriesIndex % 3]),
          pointColors: [],
        })),
        barDirection: chartType === "bar" ? "column" : null,
        grouping: "clustered", holeSize: chartType === "doughnut" ? .55 : null,
      },
      style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: 1 },
    };
  }

  function composeExecutiveSlide(input, deck, palette, index, slideCount) {
    const role = String(input.role || "insight").toLowerCase();
    const objects = [];
    const title = String(input.title || `关键洞察 ${index}`);
    if (role === "cover") {
      objects.push(
        composedShape(deck, "cover-accent", 84, 228, 78, 6, { fill: palette.accent }),
        composedText(deck, palette, "cover-eyebrow", input.eyebrow || "EXECUTIVE KEYNOTE", 84, 172, 520, 30, { fontSize: 15, color: palette.primary, wordWrap: false }),
        composedText(deck, palette, "cover-title", title, 84, 264, 700, 142, { fontSize: 54, color: palette.foreground, bold: false }),
        composedText(deck, palette, "cover-subtitle", input.subtitle || input.statement || "", 88, 430, 650, 54, { fontSize: 23, color: palette.muted }),
        composedShape(deck, "cover-grid-v", 830, 78, 1, 556, { fill: palette.primary, opacity: .32 }),
        composedShape(deck, "cover-grid-h", 722, 475, 410, 1, { fill: palette.primary, opacity: .28 }),
        composedShape(deck, "cover-panel", 830, 0, 450, 720, { fill: palette.panel, opacity: .32 }),
        composedShape(deck, "cover-cross-h", 1160, 86, 38, 1, { fill: palette.primary }),
        composedShape(deck, "cover-cross-v", 1178, 68, 1, 38, { fill: palette.primary }),
        composedText(deck, palette, "cover-footer", input.footer || input.audience || "", 84, 642, 780, 24, { fontSize: 11, color: palette.muted, wordWrap: false }),
      );
    } else if (role === "closing") {
      objects.push(
        composedShape(deck, "closing-accent", 567, 205, 140, 6, { fill: palette.accent }),
        composedText(deck, palette, "closing-title", title, 200, 268, 880, 70, { fontSize: 44, color: palette.foreground, align: "center", wordWrap: false }),
        composedText(deck, palette, "closing-statement", input.statement || input.subtitle || "", 160, 356, 960, 72, { fontSize: 30, color: palette.primary, align: "center" }),
        composedText(deck, palette, "closing-footer", input.footer || "", 260, 485, 760, 42, { fontSize: 17, color: palette.muted, align: "center" }),
      );
    } else {
      objects.push(...composedChrome(deck, palette, { ...input, title }, index, slideCount));
      const takeaway = String(input.takeaway || "");
      if (role === "big-number" || input.metric) {
        const metric = input.metric || {};
        objects.push(
          composedText(deck, palette, "metric-value", metric.value || input.value || "—", 60, 244, 470, 112, { fontSize: 72, color: metric.color || palette.primary, wordWrap: false }),
          composedText(deck, palette, "metric-label", metric.label || input.statement || "", 66, 365, 520, 52, { fontSize: 22, color: palette.foreground }),
          composedText(deck, palette, "metric-context", metric.context || input.subtitle || "", 66, 438, 520, 100, { fontSize: 18, color: palette.muted }),
          composedShape(deck, "metric-divider", 700, 235, 1, 330, { fill: palette.rule }),
        );
        if (input.chart?.categories?.length && input.chart?.series?.length) objects.push(composedChart(deck, palette, input.chart));
        else objects.push(composedText(deck, palette, "metric-implication", input.implication || takeaway, 748, 300, 420, 170, { fontSize: 30, color: palette.foreground }));
      } else if (["three-columns", "decision", "pillars"].includes(role)) {
        const columns = normalizedItems(input.columns || input.items, 3);
        const width = columns.length === 2 ? 520 : 346;
        const gap = columns.length === 2 ? 70 : 35;
        columns.forEach((item, itemIndex) => {
          const x = 60 + itemIndex * (width + gap);
          if (itemIndex) objects.push(composedShape(deck, `column-divider-${itemIndex}`, x - gap / 2, 250, 1, 300, { fill: palette.rule }));
          objects.push(
            composedText(deck, palette, `column-number-${itemIndex + 1}`, item.number || String(itemIndex + 1).padStart(2, "0"), x, 242, width, 62, { fontSize: 44, color: item.accent ? palette.accent : palette.primary, wordWrap: false }),
            composedText(deck, palette, `column-title-${itemIndex + 1}`, item.title || "", x, 316, width, 50, { fontSize: 25, color: palette.foreground }),
            composedText(deck, palette, `column-body-${itemIndex + 1}`, item.body || item.description || "", x, 382, width, 150, { fontSize: 17, color: palette.muted }),
          );
        });
      } else if (["process", "roadmap"].includes(role)) {
        const steps = normalizedItems(input.steps || input.phases, 5);
        const width = (1130 - Math.max(0, steps.length - 1) * 22) / Math.max(1, steps.length);
        objects.push(composedShape(deck, "process-line", 80, 390, 1120, 2, { fill: palette.primary, opacity: .6 }));
        steps.forEach((step, stepIndex) => {
          const x = 75 + stepIndex * (width + 22);
          objects.push(
            composedText(deck, palette, `step-period-${stepIndex + 1}`, step.period || step.number || String(stepIndex + 1).padStart(2, "0"), x, 244, width, 34, { fontSize: 21, color: palette.primary, align: "center", wordWrap: false }),
            composedText(deck, palette, `step-title-${stepIndex + 1}`, step.title || "", x, 292, width, 54, { fontSize: 19, color: palette.foreground, align: "center" }),
            composedShape(deck, `step-node-${stepIndex + 1}`, x + width / 2 - 24, 366, 48, 48, { fill: palette.accent, stroke: palette.accent, strokeWidth: 1 }),
            composedText(deck, palette, `step-index-${stepIndex + 1}`, String(stepIndex + 1).padStart(2, "0"), x + width / 2 - 22, 379, 44, 24, { fontSize: 14, color: palette.background, align: "center", wordWrap: false }),
            composedText(deck, palette, `step-body-${stepIndex + 1}`, step.body || step.description || "", x, 442, width, 118, { fontSize: 16, color: palette.muted, align: "center" }),
          );
        });
      } else if (["layers", "operating-system"].includes(role)) {
        const layers = normalizedItems(input.layers || input.items, 5);
        layers.forEach((layer, layerIndex) => {
          const y = 235 + layerIndex * 82;
          objects.push(
            composedShape(deck, `layer-panel-${layerIndex + 1}`, 185, y, 990, 60, { fill: palette.panel, stroke: layer.accent ? palette.accent : palette.rule, strokeWidth: 1 }),
            composedText(deck, palette, `layer-title-${layerIndex + 1}`, layer.title || "", 215, y + 15, 230, 30, { fontSize: 20, color: layer.accent ? palette.accent : palette.primary, wordWrap: false }),
            composedText(deck, palette, `layer-body-${layerIndex + 1}`, layer.body || layer.description || "", 460, y + 15, 680, 34, { fontSize: 17, color: layer.accent ? palette.foreground : palette.muted, wordWrap: false }),
          );
        });
      } else if (["comparison", "two-column", "governance"].includes(role)) {
        const columns = normalizedItems(input.columns || [input.left, input.right], 2);
        columns.forEach((item, itemIndex) => {
          const x = itemIndex ? 695 : 60;
          objects.push(
            composedShape(deck, `comparison-panel-${itemIndex + 1}`, x, 250, 525, 275, { fill: palette.panel, stroke: itemIndex ? palette.primary : palette.rule, strokeWidth: 1 }),
            composedText(deck, palette, `comparison-kicker-${itemIndex + 1}`, item?.kicker || item?.number || "", x + 30, 280, 465, 30, { fontSize: 15, color: itemIndex ? palette.primary : palette.muted, wordWrap: false }),
            composedText(deck, palette, `comparison-title-${itemIndex + 1}`, item?.title || "", x + 30, 329, 465, 50, { fontSize: 28, color: palette.foreground }),
            composedText(deck, palette, `comparison-body-${itemIndex + 1}`, item?.body || item?.description || "", x + 30, 405, 465, 95, { fontSize: 18, color: palette.muted }),
          );
        });
        if (input.bridge) objects.push(composedText(deck, palette, "comparison-bridge", input.bridge, 540, 360, 200, 40, { fontSize: 16, color: palette.primary, align: "center" }));
      } else if (role === "matrix" || input.table) {
        const columns = normalizedItems(input.table?.columns || input.columns, 4);
        const rows = normalizedItems(input.table?.rows || input.rows, 4);
        const columnWidth = 1120 / Math.max(1, columns.length);
        columns.forEach((column, columnIndex) => {
          const x = 80 + columnIndex * columnWidth;
          objects.push(
            composedShape(deck, `matrix-header-${columnIndex + 1}`, x, 245, columnWidth, 54, { fill: palette.panel, stroke: palette.rule, strokeWidth: 1 }),
            composedText(deck, palette, `matrix-column-${columnIndex + 1}`, column.title || column, x + 12, 260, columnWidth - 24, 28, { fontSize: 18, color: palette.primary, align: "center", wordWrap: false }),
          );
        });
        rows.forEach((row, rowIndex) => {
          const values = Array.isArray(row) ? row : row.values || [];
          values.slice(0, columns.length).forEach((value, columnIndex) => {
            const x = 80 + columnIndex * columnWidth;
            const y = 299 + rowIndex * 72;
            objects.push(
              composedShape(deck, `matrix-cell-${rowIndex + 1}-${columnIndex + 1}`, x, y, columnWidth, 72, { fill: "transparent", stroke: palette.rule, strokeWidth: 1 }),
              composedText(deck, palette, `matrix-value-${rowIndex + 1}-${columnIndex + 1}`, typeof value === "string" ? value : value?.text || "", x + 14, y + 16, columnWidth - 28, 44, { fontSize: 16, color: palette.foreground, align: "center" }),
            );
          });
        });
      } else {
        objects.push(
          composedText(deck, palette, "insight-statement", input.statement || input.subtitle || "", 60, 245, 1130, 90, { fontSize: 31, color: palette.foreground }),
          composedShape(deck, "insight-divider", 60, 354, 1120, 1, { fill: palette.rule }),
        );
        normalizedItems(input.bullets || input.items, 5).forEach((item, itemIndex) => {
          objects.push(
            composedText(deck, palette, `bullet-number-${itemIndex + 1}`, String(itemIndex + 1).padStart(2, "0"), 75, 385 + itemIndex * 48, 50, 30, { fontSize: 16, color: palette.primary, wordWrap: false }),
            composedText(deck, palette, `bullet-${itemIndex + 1}`, typeof item === "string" ? item : item.title || item.body || "", 145, 382 + itemIndex * 48, 1000, 36, { fontSize: 19, color: palette.muted }),
          );
        });
      }
      if (takeaway && !objects.some((object) => object.name === "metric-implication")) {
        objects.push(
          composedShape(deck, "takeaway-rule", 60, 590, 1120, 1, { fill: palette.rule }),
          composedText(deck, palette, "takeaway", takeaway, 60, 610, 1120, 34, { fontSize: 20, color: palette.foreground, align: "center", wordWrap: false }),
        );
      }
    }
    return {
      name: String(input.name || title), background: palette.background,
      notes: composedSourceNotes(input.notes, input.sources), objects,
      transition: clone(input.transition ?? { kind: "fade", durationMs: 650, advanceOnClick: true, advanceAfterMs: null, direction: null }),
      _semantic: { role, summary: String(input.summary || input.takeaway || input.statement || title) },
    };
  }

  function composeExecutiveDeck(args = {}, options = {}) {
    const width = Math.max(1, finite(args.width, 1280));
    const height = Math.max(1, finite(args.height, 720));
    const presetName = AI_DECK_PRESETS[args.designPreset] ? args.designPreset : "midnight-consulting";
    const palette = { ...AI_DECK_PRESETS[presetName], ...(clone(args.design || {})) };
    const deck = {
      format: "unippt", version: 1, title: String(args.title || "演示文稿"), width, height,
      sourceWidthEmu: Math.round(width * 9525), sourceHeightEmu: Math.round(height * 9525),
      sourceImportId: null, fonts: [], extensions: {}, slides: [],
    };
    const requestedSlides = normalizedItems(args.slides, 24);
    const slideInputs = requestedSlides.length ? requestedSlides : [{ role: "cover", title: args.title || "演示文稿", subtitle: args.purpose || "" }];
    slideInputs.forEach((input, index) => {
      const spec = composeExecutiveSlide(input || {}, deck, palette, index + 1, slideInputs.length);
      const semantic = spec._semantic;
      delete spec._semantic;
      const slide = makeSlide(spec, deck, options);
      deck.slides.push(slide);
      slide._aiSemantic = semantic;
    });
    enrichSemantics(deck, options);
    const metadata = extension(deck);
    metadata.document = {
      summary: String(args.centralTakeaway || args.purpose || ""),
      language: String(args.language || "zh-CN"), audience: String(args.audience || ""),
      purpose: String(args.purpose || ""),
    };
    metadata.designTokens = {
      generator: "executive-composer-v1", preset: presetName, ...palette,
      minimumFontSizes: { deckTitle: 50, slideTitle: 35, body: 16 },
    };
    deck.slides.forEach((slide) => {
      metadata.slides[slide.id] = { ...(metadata.slides[slide.id] || {}), ...(slide._aiSemantic || {}) };
      delete slide._aiSemantic;
    });
    return deck;
  }

  function findSlide(deck, id) {
    const index = deck.slides.findIndex((slide) => slide.id === id);
    if (index < 0) throw new HostError("SLIDE_NOT_FOUND", `找不到幻灯片 ${id}`);
    return { slide: deck.slides[index], index };
  }

  function findObject(deck, id, slideId = null) {
    const slides = slideId ? [findSlide(deck, slideId).slide] : deck.slides;
    for (const slide of slides) {
      let found = null;
      visitObjects(slide.objects || [], (object, context) => {
        if (!found && object.id === id) found = { slide, object, ...context };
      });
      if (found) return found;
    }
    throw new HostError("OBJECT_NOT_FOUND", `找不到场景对象 ${id}`);
  }

  function findAnimation(slide, id) {
    const index = (slide.animations || []).findIndex((animation) => animation.id === id);
    if (index < 0) throw new HostError("ANIMATION_NOT_FOUND", `找不到动画 ${id}`);
    return { animation: slide.animations[index], index };
  }

  function removeSemanticObject(metadata, object) {
    delete metadata.objects[object.id];
    visitObjects(object.children || [], (child) => { delete metadata.objects[child.id]; });
  }

  function applyOperation(deck, operation, context, options) {
    const op = String(operation?.op || "");
    if (!op) throw new HostError("INVALID_OPERATION", "ChangeSet 操作缺少 op");
    const metadata = extension(deck);
    switch (op) {
      case "setTitle":
        deck.title = String(operation.title || "演示文稿");
        break;
      case "setDocumentMetadata":
        metadata.document = deepMerge(metadata.document, operation.patch || {});
        break;
      case "setDesignTokens":
        metadata.designTokens = deepMerge(metadata.designTokens, operation.tokens || {});
        break;
      case "insertSlide": {
        const slide = makeSlide(operation.slide || {}, deck, options);
        let index = deck.slides.length;
        if (operation.beforeSlideId) index = findSlide(deck, operation.beforeSlideId).index;
        else if (operation.afterSlideId) index = findSlide(deck, operation.afterSlideId).index + 1;
        else if (Number.isInteger(operation.index)) index = Math.max(0, Math.min(deck.slides.length, operation.index));
        deck.slides.splice(index, 0, slide);
        metadata.slides[slide.id] = { role: index === 0 ? "cover" : "content", summary: String(operation.summary || "") };
        visitObjects(slide.objects, (object) => { metadata.objects[object.id] = { role: inferObjectRole(object), label: object.name, description: "", styleRef: null, constraints: {} }; });
        context.activeSlideId = slide.id;
        break;
      }
      case "removeSlide": {
        if (deck.slides.length <= 1) throw new HostError("LAST_SLIDE", "演示文稿至少需要一张幻灯片");
        const { slide, index } = findSlide(deck, operation.slideId);
        visitObjects(slide.objects, (object) => removeSemanticObject(metadata, object));
        delete metadata.slides[slide.id];
        deck.slides.splice(index, 1);
        context.activeSlideId = deck.slides[Math.min(index, deck.slides.length - 1)].id;
        context.selectedObjectId = null;
        break;
      }
      case "moveSlide": {
        const { slide, index } = findSlide(deck, operation.slideId);
        const target = operation.beforeSlideId
          ? findSlide(deck, operation.beforeSlideId).index
          : operation.afterSlideId
            ? findSlide(deck, operation.afterSlideId).index + 1
            : Math.max(0, Math.min(deck.slides.length - 1, finite(operation.index, index)));
        deck.slides.splice(index, 1);
        deck.slides.splice(Math.max(0, Math.min(deck.slides.length, target > index ? target - 1 : target)), 0, slide);
        context.activeSlideId = slide.id;
        break;
      }
      case "updateSlide": {
        const { slide } = findSlide(deck, operation.slideId);
        const allowed = {};
        for (const key of ["name", "background", "backgroundAsset", "notes"]) if (key in (operation.patch || {})) allowed[key] = operation.patch[key];
        deepMerge(slide, allowed);
        if (operation.semantic) metadata.slides[slide.id] = deepMerge(metadata.slides[slide.id] || {}, operation.semantic);
        context.activeSlideId = slide.id;
        break;
      }
      case "setTransition": {
        const { slide } = findSlide(deck, operation.slideId);
        slide.transition = operation.transition == null ? null : {
          kind: String(operation.transition.kind || "fade"),
          durationMs: Math.max(1, finite(operation.transition.durationMs, 700)),
          advanceOnClick: operation.transition.advanceOnClick !== false,
          advanceAfterMs: operation.transition.advanceAfterMs == null ? null : Math.max(0, finite(operation.transition.advanceAfterMs)),
          direction: operation.transition.direction ?? null,
        };
        context.activeSlideId = slide.id;
        break;
      }
      case "addObject": {
        const { slide } = findSlide(deck, operation.slideId);
        const object = makeObject(operation.object || {}, options);
        const index = Number.isInteger(operation.index) ? Math.max(0, Math.min(slide.objects.length, operation.index)) : slide.objects.length;
        slide.objects.splice(index, 0, object);
        metadata.objects[object.id] = { role: operation.semantic?.role || inferObjectRole(object), label: object.name, description: "", styleRef: null, constraints: {}, ...(clone(operation.semantic || {})) };
        context.activeSlideId = slide.id;
        context.selectedObjectId = object.id;
        break;
      }
      case "updateObject": {
        const found = findObject(deck, operation.objectId, operation.slideId);
        deepMerge(found.object, operation.patch || {});
        if (operation.semantic) metadata.objects[found.object.id] = deepMerge(metadata.objects[found.object.id] || {}, operation.semantic);
        context.activeSlideId = found.slide.id;
        context.selectedObjectId = found.object.id;
        break;
      }
      case "updateText": {
        const found = findObject(deck, operation.objectId, operation.slideId);
        found.object.text = String(operation.text ?? "");
        found.object.textParagraphs = makeTextParagraph(found.object.text, found.object.textStyle || {});
        context.activeSlideId = found.slide.id;
        context.selectedObjectId = found.object.id;
        break;
      }
      case "removeObject": {
        const found = findObject(deck, operation.objectId, operation.slideId);
        found.objects.splice(found.index, 1);
        found.slide.animations = (found.slide.animations || []).filter((animation) => animation.targetObjectId !== found.object.id);
        removeSemanticObject(metadata, found.object);
        context.activeSlideId = found.slide.id;
        context.selectedObjectId = null;
        break;
      }
      case "reorderObject": {
        const found = findObject(deck, operation.objectId, operation.slideId);
        const [object] = found.objects.splice(found.index, 1);
        const index = operation.position === "front" ? found.objects.length
          : operation.position === "back" ? 0
            : Math.max(0, Math.min(found.objects.length, finite(operation.index, found.index)));
        found.objects.splice(index, 0, object);
        context.activeSlideId = found.slide.id;
        context.selectedObjectId = object.id;
        break;
      }
      case "addAnimation": {
        const { slide } = findSlide(deck, operation.slideId);
        if (operation.targetObjectId) findObject(deck, operation.targetObjectId, slide.id);
        slide.animations ||= [];
        const animation = makeAnimation(operation.animation || {}, slide, options, operation.targetObjectId);
        slide.animations.push(animation);
        context.activeSlideId = slide.id;
        context.selectedAnimationId = animation.id;
        break;
      }
      case "updateAnimation": {
        const { slide } = findSlide(deck, operation.slideId);
        const found = findAnimation(slide, operation.animationId);
        deepMerge(found.animation, operation.patch || {});
        context.activeSlideId = slide.id;
        context.selectedAnimationId = found.animation.id;
        break;
      }
      case "removeAnimation": {
        const { slide } = findSlide(deck, operation.slideId);
        const found = findAnimation(slide, operation.animationId);
        slide.animations.splice(found.index, 1);
        slide.animations.forEach((animation, index) => { animation.order = index; });
        context.activeSlideId = slide.id;
        context.selectedAnimationId = null;
        break;
      }
      default:
        throw new HostError("UNSUPPORTED_OPERATION", `不支持的 ChangeSet 操作：${op}`);
    }
    context.applied.push(op);
  }

  function validateDeck(deck) {
    const issues = [];
    const add = (severity, code, path, message) => issues.push({ severity, code, path, message });
    try { ensureDeck(deck); } catch (error) { return [{ severity: "error", code: error.code, path: "/", message: error.message }]; }
    if (!deck.slides.length) add("error", "EMPTY_PRESENTATION", "/slides", "演示文稿至少需要一张幻灯片");
    const slideIds = new Set();
    const objectIds = new Set();
    deck.slides.forEach((slide, slideIndex) => {
      const slidePath = `/slides/${slideIndex}`;
      if (!slide.id || slideIds.has(slide.id)) add("error", "DUPLICATE_SLIDE_ID", `${slidePath}/id`, "幻灯片 ID 缺失或重复");
      slideIds.add(slide.id);
      visitObjects(slide.objects || [], (object) => {
        const path = `${slidePath}/objects/${object.id || "?"}`;
        if (!object.id || objectIds.has(object.id)) add("error", "DUPLICATE_OBJECT_ID", `${path}/id`, "对象 ID 缺失或重复");
        objectIds.add(object.id);
        const frame = object.frame || {};
        if (![frame.x, frame.y, frame.width, frame.height, frame.rotation || 0].every((value) => Number.isFinite(Number(value)))) add("error", "INVALID_FRAME", `${path}/frame`, "对象坐标必须是有限数值");
        if (finite(frame.width) <= 0 || finite(frame.height) <= 0) add("error", "INVALID_SIZE", `${path}/frame`, "对象宽高必须大于零");
        if (finite(frame.x) + finite(frame.width) < 0 || finite(frame.y) + finite(frame.height) < 0 || finite(frame.x) > finite(deck.width) || finite(frame.y) > finite(deck.height)) add("warning", "OUTSIDE_SLIDE", `${path}/frame`, "对象完全位于幻灯片可视区域之外");
        const text = String(object.text || "");
        const fontSize = Math.max(1, finite(object.textStyle?.fontSize, 20));
        const capacity = Math.max(1, finite(frame.width) * finite(frame.height) / (fontSize * fontSize * .72));
        if (text.length > capacity * 1.45 && !["textToFitShape", "normAutofit", "shrinkText"].includes(object.textFrame?.autoSize)) add("warning", "POSSIBLE_TEXT_OVERFLOW", `${path}/text`, "文字可能溢出，建议缩小字号或启用自动适应");
      });
      const slideObjectIds = new Set();
      visitObjects(slide.objects || [], (object) => slideObjectIds.add(object.id));
      const animationIds = new Set();
      for (const animation of slide.animations || []) {
        if (!animation.id || animationIds.has(animation.id)) add("error", "DUPLICATE_ANIMATION_ID", `${slidePath}/animations`, "动画 ID 缺失或重复");
        animationIds.add(animation.id);
        if (animation.targetObjectId && !slideObjectIds.has(animation.targetObjectId)) add("error", "MISSING_ANIMATION_TARGET", `${slidePath}/animations/${animation.id}`, "动画目标对象不存在");
        if (finite(animation.durationMs) <= 0) add("error", "INVALID_ANIMATION_DURATION", `${slidePath}/animations/${animation.id}/durationMs`, "动画持续时间必须大于零");
      }
    });
    issues.push(...nativeTextIssues(deck));
    return issues;
  }

  // Shared by browser transactions, cached exports and the headless compiler.
  // Only complete omitted/null defaults; never coerce "false" to true or
  // rebuild imported objects (which would discard native binding metadata).
  function visitTextContainers(deck, callback) {
    const visit = (objects, path) => (objects || []).forEach((object, index) => {
      const at = `${path}/${index}`;
      callback(object, at);
      for (const [ri, row] of (object.table?.rows || []).entries())
        for (const [ci, cell] of (row.cells || []).entries()) callback(cell, `${at}/table/rows/${ri}/cells/${ci}`);
      visit(object.children, `${at}/children`);
    });
    for (const [i, slide] of (deck.slides || []).entries())
      for (const key of ["objects", "masterObjects", "layoutObjects"]) visit(slide[key], `/slides/${i}/${key}`);
  }

  function normalizeNativeText(deck) {
    visitTextContainers(deck, (object) => {
      const style = object.textStyle ||= {};
      const defaults = {fontFamily:"Aptos, Microsoft YaHei, sans-serif", fontSize:28, color:"#172033", bold:false, italic:false, align:"left"};
      for (const [key, value] of Object.entries(defaults)) style[key] ??= value;
      if (!Array.isArray(object.textParagraphs)) return;
      for (const paragraph of object.textParagraphs) {
        if (!paragraph || typeof paragraph !== "object") continue;
        paragraph.align ??= style.align;
        if (!Array.isArray(paragraph.runs)) continue;
        for (const run of paragraph.runs) {
          if (!run || typeof run !== "object") continue;
          for (const key of ["fontFamily", "fontSize", "color", "bold", "italic"]) run[key] ??= style[key];
          run.text ??= "";
          run.baseline ??= "normal";
          for (const key of ["underline", "strikethrough"]) if (run[key] === null) run[key] = false;
        }
      }
    });
    return deck;
  }

  function nativeTextIssues(deck) {
    const issues = [];
    const issue = (path, type) => issues.push({severity:"error", code:"INVALID_NATIVE_TEXT", path, message:`${path}: expected ${type}`});
    const styleCheck = (value, path) => {
      for (const key of ["bold", "italic"]) if (typeof value?.[key] !== "boolean") issue(`${path}/${key}`, "boolean");
      for (const key of ["fontFamily", "color"]) if (typeof value?.[key] !== "string") issue(`${path}/${key}`, "string");
      if (!Number.isFinite(value?.fontSize)) issue(`${path}/fontSize`, "finite number");
    };
    visitTextContainers(deck, (object, path) => {
      // Existing imported objects can omit the entire style (native Default).
      if (object.textStyle != null) styleCheck(object.textStyle, `${path}/textStyle`);
      if (object.textParagraphs == null) return;
      if (!Array.isArray(object.textParagraphs)) { issue(`${path}/textParagraphs`, "array"); return; }
      object.textParagraphs.forEach((paragraph, pi) => {
        const at = `${path}/textParagraphs/${pi}`;
        if (!Array.isArray(paragraph?.runs)) {issue(`${at}/runs`, "array"); return;}
        paragraph.runs.forEach((run, ri) => {
          const rp = `${at}/runs/${ri}`;
          styleCheck(run, rp);
          if (typeof run?.text !== "string") issue(`${rp}/text`, "string");
          for (const key of ["underline", "strikethrough"]) if (run?.[key] !== undefined && typeof run[key] !== "boolean") issue(`${rp}/${key}`, "boolean");
        });
      });
    });
    return issues;
  }

  function prepareNativeExport(sourceDeck) {
    const deck = normalizeNativeText(clone(sourceDeck));
    const errors = nativeTextIssues(deck);
    if (errors.length) throw new HostError("INVALID_NATIVE_TEXT", errors[0].message, errors);
    return deck;
  }

  function auditDeckQuality(deck) {
    ensureDeck(deck);
    const metadata = extension(deck);
    const strict = ["executive-composer-v1", "ai-html-composer-v1"].includes(metadata.designTokens?.generator);
    const findings = [];
    const add = (severity, code, slideIndex, message) => findings.push({ severity, code, slideIndex, message });
    const roles = new Set();
    deck.slides.forEach((slide, slideIndex) => {
      const semantic = metadata.slides?.[slide.id] || {};
      roles.add(String(semantic.role || "content"));
      const objects = [];
      visitObjects(slide.objects || [], (object) => objects.push(object));
      const title = objects.find((object) => /(?:cover-title|slide-title|closing-title|^标题$)/i.test(String(object.name || "")));
      if (!title || !String(title.text || "").trim()) add("error", "MISSING_TAKEAWAY_TITLE", slideIndex, "页面缺少面向观众的结论标题");
      const titleSize = finite(title?.textStyle?.fontSize, 0);
      const minimumTitle = slideIndex === 0 || semantic.role === "cover" ? 50 : 35;
      if (strict && titleSize < minimumTitle) add("warning", "TITLE_TOO_SMALL", slideIndex, `标题字号应至少 ${minimumTitle}`);
      if (strict && String(title?.text || "").length > 34) add("warning", "TITLE_TOO_LONG", slideIndex, "标题过长，可能削弱单页主张或发生换行");
      const visibleText = objects.filter((object) => String(object.text || "").trim());
      const characterCount = visibleText.reduce((sum, object) => sum + String(object.text || "").length, 0);
      if (strict && characterCount > 360) add("warning", "SLIDE_TOO_DENSE", slideIndex, "页面文字密度过高，应删减文案或拆页");
      for (const object of visibleText) {
        const name = String(object.name || "");
        if (/source-note|slide-number|section-label|cover-footer|cover-eyebrow|(?:^|-)index(?:-|$)|(?:^|-)kicker(?:-|$)/i.test(name)) continue;
        if (finite(object.textStyle?.fontSize, 18) < 16) add("warning", "BODY_TEXT_TOO_SMALL", slideIndex, `${name || "文本"} 小于 16pt`);
      }
      const requiresSource = objects.some((object) => object.kind === "chart" || /metric-value/.test(String(object.name || "")) && /\d/.test(String(object.text || "")));
      const sourceBlock = String(slide.notes || "").match(/\[Sources\]([\s\S]*?)\[\/Sources\]/i)?.[1] || "";
      const substantiveSources = sourceBlock.split(/\r?\n/).map((line) => line.replace(/^\s*[-*]\s*/, "").trim()).filter((line) => (
        line && !/^(?:no external sources(?:; authored synthesis\.)?|authored synthesis|none|n\/?a|无外部来源|无)$/i.test(line)
      ));
      if (strict && requiresSource && !substantiveSources.length) {
        add("error", "EVIDENCE_SOURCE_MISSING", slideIndex, "数据页缺少 speaker notes 的 [Sources] 来源块");
      }
      if (strict && slideIndex > 0 && slideIndex < deck.slides.length - 1 && objects.length < 5) {
        add("warning", "VISUAL_STRUCTURE_TOO_THIN", slideIndex, "内容页缺少支撑结论的视觉结构");
      }
    });
    if (strict && deck.slides.length >= 6 && roles.size < 4) findings.push({ severity: "warning", code: "LAYOUT_VARIETY_LOW", slideIndex: null, message: "全稿页面轮廓变化不足" });
    const structural = validateDeck(deck);
    for (const issue of structural) if (issue.severity === "error" || /OVERFLOW|OUTSIDE/.test(issue.code)) findings.push({ ...issue, slideIndex: null });
    const penalty = findings.reduce((sum, finding) => sum + (finding.severity === "error" ? 24 : 7), 0);
    return {
      score: Math.max(0, 100 - penalty), passed: !findings.some((finding) => finding.severity === "error") && Math.max(0, 100 - penalty) >= 80,
      strict, slideCount: deck.slides.length, layoutRoleCount: roles.size, findings,
    };
  }

  function applyChangeSet(sourceDeck, changeSet, options = {}) {
    ensureDeck(sourceDeck);
    if (!changeSet || typeof changeSet !== "object" || !Array.isArray(changeSet.operations)) throw new HostError("INVALID_CHANGESET", "ChangeSet 必须包含 operations 数组");
    if (!changeSet.operations.length) throw new HostError("EMPTY_CHANGESET", "ChangeSet 没有任何操作");
    if (changeSet.operations.length > MAX_OPERATIONS) throw new HostError("TOO_MANY_OPERATIONS", `单次事务最多 ${MAX_OPERATIONS} 个操作`);
    const revision = Number.isInteger(options.revision) ? options.revision : 0;
    if (changeSet.baseRevision != null && Number(changeSet.baseRevision) !== revision) throw new HostError("REVISION_CONFLICT", `文档已更新：期望 revision ${changeSet.baseRevision}，当前为 ${revision}`, { expected: changeSet.baseRevision, actual: revision });
    const deck = clone(sourceDeck);
    enrichSemantics(deck, options);
    const context = { applied: [], activeSlideId: null, selectedObjectId: null, selectedAnimationId: null };
    for (const operation of changeSet.operations) applyOperation(deck, operation, context, options);
    normalizeNativeText(deck);
    enrichSemantics(deck, options);
    const issues = validateDeck(deck);
    const errors = issues.filter((issue) => issue.severity === "error");
    if (errors.length) throw new HostError("VALIDATION_FAILED", `事务会产生无效演示文稿: ${errors[0].path}: ${errors[0].message}`, errors);
    return {
      transactionId: String(changeSet.transactionId || uid("tx", options)),
      baseRevision: revision,
      deck,
      issues,
      ...context,
    };
  }

  // Compile the visual inventory once. Coordinates are in original-image
  // pixels; OCR is supporting evidence, not a replacement for the inventory.
  function compileNativeImageSlide(args, deck) {
    const spec = args.slide || {};
    const source = args.sourceSize || args.ocrDetection?.image || {};
    const sw = Number(source.width), sh = Number(source.height);
    if (!(sw > 0 && sh > 0)) throw new HostError("IMAGE_SIZE_MISSING", "原生重建缺少原图尺寸");
    const scale = Math.min(deck.width / sw, deck.height / sh);
    const ox = (deck.width - sw * scale) / 2, oy = (deck.height - sh * scale) / 2;
    const regions = args.ocrDetection?.regions || [];
    const normalize = (text) => String(text || "").normalize("NFKC").replace(/[\s_{}^̂~]/g, "").toLowerCase();
    const consumed = new Set();
    const ignored = new Set();
    for (const item of spec.ignoredOcr || []) {
      if (!Number.isInteger(item.index) || !regions[item.index] || !String(item.reason || "").trim()) throw new HostError("INVALID_OCR_EXCLUSION", "忽略 OCR 必须给出有效 index 和误检原因");
      ignored.add(item.index);
    }
    const sourceObjects = clone(spec.objects || []);
    for (const object of sourceObjects.filter((item) => item.kind === "text")) {
      if (Array.isArray(object.textRuns) && object.textRuns.length) object.text = object.textRuns.map((run) => run.text || "").join("");
      let index = object.ocrIndex;
      if (index != null && (!Number.isInteger(index) || !regions[index])) throw new HostError("INVALID_OCR_INDEX", `无效 OCR index：${index}`);
      if (index == null) {
        // Repeated labels (Encoder, Input...) are matched by position, not
        // first occurrence; text-only matching corrupts repeated modules.
        const candidates = regions.map((region, i) => ({ region, i })).filter(({ region, i }) => !consumed.has(i) && normalize(region.text) === normalize(object.text));
        candidates.sort((a, b) => {
          const distance = ({ region }) => {
            const p = region.location || [];
            return Math.hypot((p[0] + p[4]) / 2 - (object.frame?.x + object.frame?.width / 2), (p[1] + p[5]) / 2 - (object.frame?.y + object.frame?.height / 2));
          };
          return distance(a) - distance(b);
        });
        index = candidates[0]?.i;
      }
      if (index != null) consumed.add(index);
    }
    // Preserve unaccounted OCR regions and report them for visual review.
    // The model may correct a region by referencing its index, or explicitly
    // exclude noise with a reason; it cannot silently drop recognised text.
    let fallbackTextCount = 0;
    regions.forEach((region, index) => {
      if (consumed.has(index) || ignored.has(index)) return;
      const p = region.location || [];
      if (p.length !== 8 || !p.every(Number.isFinite) || !String(region.text || "").trim()) return;
      const width = Math.hypot(p[2] - p[0], p[3] - p[1]);
      const height = Math.hypot(p[6] - p[0], p[7] - p[1]);
      sourceObjects.push({ kind: "text", name: `待校对 OCR ${index}`, text: region.text,
        frame: { x: (p[0] + p[4]) / 2 - width / 2, y: (p[1] + p[5]) / 2 - height / 2, width: Math.max(1, width), height: Math.max(1, height), rotation: Math.atan2(p[3] - p[1], p[2] - p[0]) * 180 / Math.PI },
        textStyle: { fontSize: Math.max(5, height * .8), color: region.foreground || "#172033" } });
      fallbackTextCount++;
    });
    const objects = [];
    const compile = (input, dx = 0, dy = 0) => {
      const object = clone(input);
      const frame = object.frame || {};
      if (![frame.x, frame.y, frame.width, frame.height, frame.rotation ?? 0].every(Number.isFinite) || frame.width <= 0 || frame.height <= 0) throw new HostError("INVALID_NATIVE_FRAME", `${object.name || object.text || "图层"} 必须提供有限的 x/y/width/height，宽高大于零`);
      object.frame = { x: ox + (frame.x + dx) * scale, y: oy + (frame.y + dy) * scale, width: frame.width * scale, height: frame.height * scale, rotation: frame.rotation || 0 };
      if (!["text", "shape", "connector"].includes(object.kind)) throw new HostError("INVALID_NATIVE_KIND", "原生图层仅支持 text/shape/connector；照片使用 sourceCrops");
      object.style = { fill: "transparent", stroke: "transparent", strokeWidth: 0, ...(object.style || {}) };
      object.style.strokeWidth *= scale;
      if (object.kind === "text") {
        object.textStyle = { fontFamily: "Arial", color: "#172033", fontSize: Math.max(5, frame.height * .8), ...(object.textStyle || {}) };
        object.textStyle.fontSize *= scale;
        object.textFrame = { marginLeft: 0, marginRight: 0, marginTop: 0, marginBottom: 0, wordWrap: false, autoSize: "none", verticalAlign: "center", verticalType: "horz", ...(object.textFrame || {}) };
        if (object.textRuns?.length) {
          object.textParagraphs = makeTextParagraph("", object.textStyle);
          object.textParagraphs[0].runs = object.textRuns.map((run) => ({ ...makeTextParagraph(String(run.text || ""), object.textStyle)[0].runs[0], ...run,
            fontSize: Number.isFinite(run.fontSize) ? run.fontSize * scale : object.textStyle.fontSize * (["sub", "super"].includes(run.baseline) ? .65 : 1), baseline: run.baseline || "normal" }));
        }
      } else {
        object.geometry ||= object.kind === "connector" ? "line" : "rect";
        let path = object.path;
        if (Array.isArray(object.points)) path = object.points.map((point, index) => `${index ? "L" : "M"} ${point[0]} ${point[1]}`).join(" ") + (object.closed === false ? "" : " Z");
        if (!path && object.geometry === "roundRect") {
          const w = frame.width, h = frame.height, r = Math.max(0, Math.min(finite(object.cornerRadius, Math.min(12, w * .1, h * .1)), w / 2, h / 2)), k = r * .55228475;
          path = `M ${r} 0 L ${w-r} 0 C ${w-r+k} 0 ${w} ${r-k} ${w} ${r} L ${w} ${h-r} C ${w} ${h-r+k} ${w-r+k} ${h} ${w-r} ${h} L ${r} ${h} C ${r-k} ${h} 0 ${h-r+k} 0 ${h-r} L 0 ${r} C 0 ${r-k} ${r-k} 0 ${r} 0 Z`;
        }
        if (object.kind === "connector" && path) {
          const tokens = path.replace(/,/g, " ").match(/[MLCZ]|[-+]?(?:\d*\.)?\d+(?:e[-+]?\d+)?/g) || [];
          if (tokens.length === 6 && tokens[0] === "M" && tokens[3] === "L") {
            object.flipH = Number(tokens[4]) < Number(tokens[1]);
            object.flipV = Number(tokens[5]) < Number(tokens[2]);
            object.geometry = "line";
            path = null;
          } else {
            // PowerPoint freeform curves are native editable shapes. Keep
            // ordinary straight edges as p:cxnSp and curves as p:sp paths.
            object.kind = "shape";
          }
        }
        if (path) {
          if (typeof path !== "string" || path.length > 16000 || !/^[MLCZmlcz\d.,+\-\seE]+$/.test(path)) throw new HostError("INVALID_NATIVE_PATH", "自由形状路径只允许绝对 M/L/C/Z 命令和有限数值");
          object.customGeometry = { width: Math.max(1, Math.round(frame.width * 100)), height: Math.max(1, Math.round(frame.height * 100)), pathData: path.replace(/[-+]?(?:\d*\.)?\d+(?:e[-+]?\d+)?/gi, (value) => String(Math.round(Number(value) * 100))) };
        }
      }
      for (const key of ["repeat", "ocrIndex", "textRuns", "points", "path", "closed", "cornerRadius"]) delete object[key];
      // Geometry checks use the rotated box; a vertical label's unrotated
      // frame can legitimately start outside the slide.
      const radians = object.frame.rotation * Math.PI / 180;
      const bw = Math.abs(Math.cos(radians)) * object.frame.width + Math.abs(Math.sin(radians)) * object.frame.height;
      const bh = Math.abs(Math.sin(radians)) * object.frame.width + Math.abs(Math.cos(radians)) * object.frame.height;
      const cx = object.frame.x + object.frame.width / 2, cy = object.frame.y + object.frame.height / 2;
      if (cx + bw / 2 < 0 || cy + bh / 2 < 0 || cx - bw / 2 > deck.width || cy - bh / 2 > deck.height) throw new HostError("NATIVE_OBJECT_OUTSIDE", `${object.name || object.text || "图层"} 完全超出页面`);
      objects.push(object);
    };
    for (const input of sourceObjects) {
      const count = input.repeat?.count ?? 1;
      if (!Number.isInteger(count) || count < 1 || count > 64 || objects.length + count > 800) throw new HostError("NATIVE_OBJECT_LIMIT", "重复数量必须为 1–64，总对象数不得超过 800");
      for (let i = 0; i < count; i++) compile(input, finite(input.repeat?.dx) * i, finite(input.repeat?.dy) * i);
    }
    // Each crop is an independently movable photo, never a full-page layer.
    for (const crop of spec.sourceCrops || []) {
      const f = crop.frame || {}, c = crop.crop || {};
      if (![f.x, f.y, f.width, f.height].every(Number.isFinite) || f.width <= 0 || f.height <= 0 || f.width * f.height > sw * sh * .35) throw new HostError("NATIVE_RASTER_COVERAGE", "原生重建只允许局部照片裁剪，不允许整页底图");
      const insets = [c.left ?? 0, c.top ?? 0, c.right ?? 0, c.bottom ?? 0];
      if (!insets.every((v) => Number.isFinite(v) && v >= 0 && v < 1) || insets[0] + insets[2] >= 1 || insets[1] + insets[3] >= 1) throw new HostError("INVALID_NATIVE_CROP", "照片 crop 为 0–1 之间的四边裁剪比例");
      objects.push({ kind: "image", name: crop.name || "局部照片", frame: { x: ox + f.x * scale, y: oy + f.y * scale, width: f.width * scale, height: f.height * scale }, asset: args.sourceImage, imageCrop: c });
    }
    const textObjects = objects.filter((o) => o.kind === "text" && String(o.text || "").trim());
    const report = { mode: "native", nativeText: textObjects.length, nativeShapes: objects.filter((o) => o.kind === "shape").length, nativeConnectors: objects.filter((o) => o.kind === "connector").length,
      rotatedText: textObjects.filter((o) => Math.abs(o.frame.rotation) > 5 || ![undefined, "horz"].includes(o.textFrame?.verticalType)).length, photos: (spec.sourceCrops || []).length, fallbackTextCount, ignoredOcrCount: ignored.size, visualReviewRequired: true };
    const checks = spec.verification;
    if (!checks || !Array.isArray(checks.labels) || !checks.labels.length || !Number.isInteger(checks.rotatedTextCount)) throw new HostError("NATIVE_INVENTORY_MISSING", "先清点原图：verification 必须包含 labels、rotatedTextCount、diagram，再提交原生重建");
    const missing = checks.labels.filter((text) => !textObjects.some((o) => normalize(o.text) === normalize(text)));
    if (missing.length || report.rotatedText < checks.rotatedTextCount || (checks.diagram && report.nativeShapes + report.nativeConnectors === 0)) throw new HostError("NATIVE_COVERAGE_FAILED", `原生覆盖检查失败：缺少标签 ${missing.join("、") || "无"}；旋转文字 ${report.rotatedText}/${checks.rotatedTextCount}；形状/连接线 ${report.nativeShapes}/${report.nativeConnectors}`);
    return { slide: { name: spec.name || args.attachmentName, background: spec.background || "#ffffff", notes: spec.notes || "", objects }, report };
  }

  // Use the same model normalization as an approved slide insertion. A raw
  // compiled inventory is not a complete serializable Rust Deck.
  function compileNativeImageDeck(args) {
    const { width, height } = args.sourceSize;
    const deck = { format: "unippt", version: 1, title: args.slide.name || "Private quality candidate", width, height,
      sourceWidthEmu: Math.round(width * 9525), sourceHeightEmu: Math.round(height * 9525), sourceImportId: null, fonts: [], slides: [] };
    deck.slides.push(makeSlide(compileNativeImageSlide(args, deck).slide, deck));
    return deck;
  }

  const AI_FRAME_SCHEMA = Object.freeze({
    type: "object",
    properties: {
      x: { type: "number" }, y: { type: "number" },
      width: { type: "number" }, height: { type: "number" },
      rotation: { type: "number" },
    },
  });
  const AI_OBJECT_SCHEMA = Object.freeze({
    type: "object",
    required: ["kind"],
    properties: {
      kind: { type: "string", description: "text/shape/connector/image/table/chart/formula/media/dynamic/group" },
      name: { type: "string" }, text: { type: "string" },
      frame: AI_FRAME_SCHEMA,
      textStyle: { type: "object" }, style: { type: "object" },
      assetId: { type: "string" }, source: { type: "string" },
      geometry: { type: "string", description: "PowerPoint 预设，如 rect/roundRect/ellipse/line" },
      customGeometry: { type: "object" }, textFrame: { type: "object" },
      ocrIndex: { type: "integer", description: "此文字校正/替代的 OCR 索引，从 0 开始；漏检的文字不填" },
      textRuns: { type: "array", items: { type: "object", required: ["text"], properties: { text: { type: "string" }, baseline: { type: "string", enum: ["normal", "sub", "super"] }, italic: { type: "boolean" }, bold: { type: "boolean" } } } },
      path: { type: "string", description: "自由形状的绝对 M/L/C/Z 路径，局部原图像素坐标" },
      points: { type: "array", description: "多边形局部像素顶点 [[x,y],...]", items: { type: "array", items: { type: "number" }, minItems: 2, maxItems: 2 } },
      closed: { type: "boolean" },
      cornerRadius: { type: "number", minimum: 0 },
      repeat: { type: "object", properties: { count: { type: "integer", minimum: 1, maximum: 64 }, dx: { type: "number" }, dy: { type: "number" } } },
      children: { type: "array", items: { type: "object" } },
    },
  });
  const AI_SLIDE_SCHEMA = Object.freeze({
    type: "object",
    description: "完整的新页面定义。简单标题页直接填写 title/subtitle；列表页填写 title/bullets；复杂页面在 objects 中一次给出全部对象，避免创建页面后猜测新 ID。",
    properties: {
      name: { type: "string" }, title: { type: "string" }, subtitle: { type: "string" },
      bullets: { type: "array", items: { type: "string" } },
      background: { type: "string" }, notes: { type: "string" },
      objects: { type: "array", items: AI_OBJECT_SCHEMA },
      animations: { type: "array", items: { type: "object" } },
      transition: { type: ["object", "null"] },
    },
  });
  const AI_CHANGESET_OPERATION_SCHEMA = Object.freeze({
    oneOf: [
      ["setTitle", ["title"], { title: { type: "string" } }],
      ["setDocumentMetadata", ["patch"], { patch: { type: "object" } }],
      ["setDesignTokens", ["tokens"], { tokens: { type: "object" } }],
      ["insertSlide", ["slide"], { slide: AI_SLIDE_SCHEMA }],
      ["removeSlide", ["slideId"], {}],
      ["moveSlide", ["slideId"], {}],
      ["updateSlide", ["slideId", "patch"], { patch: { type: "object" }, semantic: { type: "object" } }],
      ["setTransition", ["slideId", "transition"], { transition: { type: ["object", "null"] } }],
      ["addObject", ["slideId", "object"], { object: AI_OBJECT_SCHEMA, semantic: { type: "object" } }],
      ["updateObject", ["objectId", "patch"], { patch: { type: "object" }, semantic: { type: "object" } }],
      ["updateText", ["objectId", "text"], { text: { type: "string" } }],
      ["removeObject", ["objectId"], {}],
      ["reorderObject", ["objectId"], { position: { type: "string", enum: ["front", "back"] } }],
      ["addAnimation", ["slideId", "animation"], { animation: { type: "object" }, targetObjectId: { type: "string" } }],
      ["updateAnimation", ["slideId", "animationId", "patch"], { patch: { type: "object" } }],
      ["removeAnimation", ["slideId", "animationId"], {}],
    ].map(([op, required, properties]) => ({ type: "object", required: ["op", ...required], additionalProperties: false,
      properties: { op: { type: "string", enum: [op] }, slideId: { type: "string" }, objectId: { type: "string" }, animationId: { type: "string" }, beforeSlideId: { type: "string" }, afterSlideId: { type: "string" }, index: { type: "integer" }, summary: { type: "string" }, ...properties } })),
  });
  const AI_COMPOSE_SOURCE_SCHEMA = Object.freeze({
    type: ["object", "string"],
    properties: { label: { type: "string" }, url: { type: "string" } },
  });
  const AI_COMPOSE_ITEM_SCHEMA = Object.freeze({
    type: "object",
    properties: {
      number: { type: "string" }, period: { type: "string" }, kicker: { type: "string" },
      title: { type: "string" }, body: { type: "string" }, description: { type: "string" },
      accent: { type: "boolean" }, values: { type: "array" },
    },
  });
  const AI_COMPOSE_SLIDE_SCHEMA = Object.freeze({
    type: "object",
    required: ["role", "title"],
    properties: {
      role: { type: "string", description: "cover/big-number/three-columns/process/roadmap/layers/comparison/two-column/governance/matrix/decision/insight/closing" },
      section: { type: "string" }, eyebrow: { type: "string" }, title: { type: "string" },
      subtitle: { type: "string" }, statement: { type: "string" }, implication: { type: "string" },
      takeaway: { type: "string" }, footer: { type: "string" }, notes: { type: "string" },
      metric: { type: "object", properties: { value: { type: "string" }, label: { type: "string" }, context: { type: "string" }, color: { type: "string" } } },
      bullets: { type: "array", items: { type: "string" } },
      columns: { type: "array", items: AI_COMPOSE_ITEM_SCHEMA },
      items: { type: "array", items: AI_COMPOSE_ITEM_SCHEMA },
      steps: { type: "array", items: AI_COMPOSE_ITEM_SCHEMA },
      phases: { type: "array", items: AI_COMPOSE_ITEM_SCHEMA },
      layers: { type: "array", items: AI_COMPOSE_ITEM_SCHEMA },
      left: AI_COMPOSE_ITEM_SCHEMA, right: AI_COMPOSE_ITEM_SCHEMA,
      bridge: { type: "string" },
      table: { type: "object", properties: { columns: { type: "array" }, rows: { type: "array" } } },
      chart: {
        type: "object",
        properties: {
          chartType: { type: "string" }, title: { type: "string" }, legend: { type: "boolean" },
          categories: { type: "array", items: { type: "string" } },
          series: { type: "array", items: { type: "object", properties: { name: { type: "string" }, values: { type: "array", items: { type: "number" } }, color: { type: "string" } } } },
        },
      },
      sources: { type: "array", items: AI_COMPOSE_SOURCE_SCHEMA },
      transition: { type: ["object", "null"] },
    },
  });
  const AI_COMPOSE_SCHEMA = Object.freeze({
    type: "object",
    required: ["title", "audience", "purpose", "centralTakeaway", "slides"],
    properties: {
      title: { type: "string" }, audience: { type: "string" }, purpose: { type: "string" },
      centralTakeaway: { type: "string" }, language: { type: "string" },
      designPreset: { type: "string", enum: ["midnight-consulting", "clean-executive"] },
      design: { type: "object", description: "可选颜色/字体覆盖；通常保留预设以维持全稿一致性" },
      width: { type: "number" }, height: { type: "number" },
      slides: { type: "array", minItems: 2, maxItems: 24, items: AI_COMPOSE_SLIDE_SCHEMA },
    },
  });
  const AI_FREE_HTML_SCHEMA = Object.freeze({
    type: "object",
    required: ["attachmentName"],
    properties: {
      attachmentName: { type: "string", description: "本轮自由 HTML 附件的精确文件名；运行时安全注入源码" },
      title: { type: "string", description: "转换后的演示文稿标题；省略时使用 HTML title 或文件名" },
      width: { type: "number", description: "目标幻灯片宽度，默认 1280" },
      height: { type: "number", description: "目标幻灯片高度，默认 720" },
    },
  });
  const AI_IMAGE_RECONSTRUCTION_SCHEMA = Object.freeze({
    type: "object",
    description: "将图片重建为原生 PPT 对象。native 模式坐标使用原图像素，OCR 仅作参考，保留旋转文字、模块、箭头、层级；照片单独裁剪。",
    required: ["attachmentName", "slide"],
    properties: {
      attachmentName: { type: "string", description: "本轮图片附件的精确文件名；运行时会安全注入原图，不要输出 data URL" },
      afterSlideId: { type: "string", description: "新页面插入到此页之后；省略时插入当前页之后" },
      slide: {
        type: "object",
        required: ["name", "objects", "verification"],
        properties: {
          name: { type: "string" },
          mode: { type: "string", enum: ["native", "hybrid"], description: "图片转可编辑 PPT 使用 native：原图坐标、原生模块和连接线、无整页底图" },
          verification: { type: "object", required: ["labels", "rotatedTextCount", "diagram"], properties: { labels: { type: "array", items: { type: "string" } }, rotatedTextCount: { type: "integer", minimum: 0 }, diagram: { type: "boolean" } } },
          ignoredOcr: { type: "array", items: { type: "object", required: ["index", "reason"], properties: { index: { type: "integer" }, reason: { type: "string" } } } },
          background: { type: "string", description: "从原图估计的页面底色" },
          notes: { type: "string" },
          preserveSourceImage: { type: "boolean", description: "默认 true：将原图作为全页保真底层" },
          autoMaskText: { type: "boolean", description: "默认 true：为每个 OCR 文本框自动生成不透明文字击穿遮罩，避免原图文字与可编辑文字重影" },
          textRegionFill: { type: "string", description: "自动文字遮罩的颜色；应取文字区附近背景色，默认使用 slide.background" },
          sourceCrops: {
            type: "array",
            description: "仅在 preserveSourceImage=false、页面没有整页保真底图时使用：从原图裁剪成可移动的独立图片层。整页底图已存在时宿主会忽略这些裁剪，避免重复叠图",
            items: { type: "object", required: ["name", "frame", "crop"], properties: { name: { type: "string" }, frame: { ...AI_FRAME_SCHEMA, required: ["x", "y", "width", "height"] }, crop: { type: "object", additionalProperties: false, required: ["left", "top", "right", "bottom"], properties: { left: { type: "number" }, top: { type: "number" }, right: { type: "number" }, bottom: { type: "number" } } }, style: { type: "object" } } },
          },
          occlusions: {
            type: "array",
            description: "仅在没有专用 OCR 检测结果时回退使用：覆盖原图中文字的矩形/渐变遮罩。存在专用 OCR 时宿主会忽略这里的内容并生成逐文字区紧贴遮罩",
            items: { type: "object", required: ["name", "frame", "style"], properties: { name: { type: "string" }, frame: AI_FRAME_SCHEMA, style: { type: "object" } } },
          },
          objects: {
            type: "array",
            minItems: 1,
            description: "一次提交全部原生文本、形状和 connector。native 模式保留视觉校正的内容和坐标，ocrIndex 关联被校正的 OCR；repeat 压缩层叠结构，textRuns 保存上下标。",
            items: { ...AI_OBJECT_SCHEMA, required: ["kind", "frame"], additionalProperties: false,
              properties: { ...AI_OBJECT_SCHEMA.properties,
                kind: { type: "string", enum: ["text", "shape", "connector"] },
                frame: { ...AI_FRAME_SCHEMA, required: ["x", "y", "width", "height"] },
                path: { type: "string", pattern: "^[MLCZ0-9.,+ eE\\s-]+$", description: "局部坐标，必须给 frame，只支持绝对 M L C Z；Q 必须转为 C" },
                textStyle: { type: "object", additionalProperties: false, properties: { fontFamily: { type: "string" }, fontSize: { type: "number" }, color: { type: "string" }, bold: { type: "boolean" }, italic: { type: "boolean" }, align: { type: "string" } } },
              } },
          },
          transition: { type: ["object", "null"] },
        },
      },
    },
  });

  const TOOL_DEFINITIONS = Object.freeze([
    { name: "presentation.inspect", description: "读取演示文稿、幻灯片和对象的结构化摘要", inputSchema: { type: "object", properties: { slideId: { type: "string" }, includeObjects: { type: "boolean" } } } },
    { name: "presentation.validate", description: "检查对象越界、文字溢出、ID 与动画目标", inputSchema: { type: "object", properties: {} } },
    { name: "presentation.compose", description: "内部兼容用的咨询级确定性整稿网格：模型只提交受众、目的、核心结论、证据来源和页面角色，宿主生成原生图表和来源备注；普通 U AI 新建整稿应走自由 HTML 主链路，本工具不作为默认入口", inputSchema: AI_COMPOSE_SCHEMA },
    { name: "presentation.importFreeHtml", description: "把 UniDoc 或其他静态自由 HTML 一次性编译为可编辑 PPT：浏览器负责最终布局，文本、形状、图片、SVG 与 Canvas 分层转换，并返回可编辑率和不支持项；不要把整页仅作为截图", inputSchema: AI_FREE_HTML_SCHEMA },
    { name: "presentation.create", description: "创建新的结构化演示文稿；每页可直接包含标题、副标题、正文、对象、动画和切换", inputSchema: { type: "object", required: ["title", "slides"], properties: { title: { type: "string" }, width: { type: "number" }, height: { type: "number" }, slides: { type: "array", items: AI_SLIDE_SCHEMA } } } },
    { name: "slide.add", description: "在指定位置一次性新增完整幻灯片。slide 支持 title、subtitle、bullets、objects、animations 和 transition；不要先建空白页再猜测新 ID", inputSchema: { type: "object", required: ["slide"], properties: { afterSlideId: { type: "string" }, beforeSlideId: { type: "string" }, slide: AI_SLIDE_SCHEMA } } },
    { name: "slide.reconstructFromImage", description: "把图片附件一次性重建成原生可编辑页面：mode=native，全部模块、文字和连接线独立，照片局部裁剪，无整页底图。先提供 verification 清点，再提交 objects。只新增一页，不删除或替换原页面；图片转 PPT 只调用一次本工具。", inputSchema: AI_IMAGE_RECONSTRUCTION_SCHEMA },
    { name: "slide.update", description: "更新幻灯片名称、背景、备注或语义", inputSchema: { type: "object", required: ["slideId", "patch"], properties: { slideId: { type: "string" }, patch: { type: "object" }, semantic: { type: "object" } } } },
    { name: "slide.remove", description: "删除一张幻灯片", inputSchema: { type: "object", required: ["slideId"], properties: { slideId: { type: "string" } } } },
    { name: "object.add", description: "向现有幻灯片添加文本、形状、图片、图表、表格、公式、媒体或动态内容对象", inputSchema: { type: "object", required: ["slideId", "object"], properties: { slideId: { type: "string" }, object: AI_OBJECT_SCHEMA, semantic: { type: "object" } } } },
    { name: "object.update", description: "精准更新对象字段与语义，不触碰原生绑定", inputSchema: { type: "object", required: ["objectId", "patch"], properties: { slideId: { type: "string" }, objectId: { type: "string" }, patch: { type: "object" }, semantic: { type: "object" } } } },
    { name: "object.setText", description: "替换一个文本对象的内容", inputSchema: { type: "object", required: ["objectId", "text"], properties: { slideId: { type: "string" }, objectId: { type: "string" }, text: { type: "string" } } } },
    { name: "object.remove", description: "删除对象及其关联动画", inputSchema: { type: "object", required: ["objectId"], properties: { slideId: { type: "string" }, objectId: { type: "string" } } } },
    { name: "animation.add", description: "给对象添加结构化 PowerPoint 动画", inputSchema: { type: "object", required: ["slideId", "targetObjectId", "animation"], properties: { slideId: { type: "string" }, targetObjectId: { type: "string" }, animation: { type: "object" } } } },
    { name: "transition.set", description: "设置幻灯片切换效果", inputSchema: { type: "object", required: ["slideId", "transition"], properties: { slideId: { type: "string" }, transition: { type: ["object", "null"] } } } },
    { name: "presentation.applyChangeSet", description: "原子执行批量修改。operations 必须为对象数组；每项使用 op（如 removeObject/updateText/updateObject），不可用 type 或 object.remove。对象 ID 必须来自 inspect。", inputSchema: { type: "object", required: ["operations"], properties: { baseRevision: { type: "integer" }, operations: { type: "array", minItems: 1, maxItems: MAX_OPERATIONS, items: AI_CHANGESET_OPERATION_SCHEMA } } } },
  ]);

  function summarize(deck, args = {}) {
    const metadata = extension(deck);
    const slides = args.slideId ? [findSlide(deck, args.slideId).slide] : deck.slides;
    return {
      type: DOCUMENT_TYPE,
      title: deck.title,
      size: { width: deck.width, height: deck.height },
      slideCount: deck.slides.length,
      designTokens: clone(metadata.designTokens),
      slides: slides.map((slide, index) => ({
        id: slide.id, index: deck.slides.indexOf(slide), name: slide.name,
        notes: slide.notes, semantic: clone(metadata.slides[slide.id] || {}),
        objectCount: (slide.objects || []).length,
        animationCount: (slide.animations || []).length,
        transition: clone(slide.transition),
        objects: args.includeObjects ? (slide.objects || []).map((object) => ({ id: object.id, name: object.name, kind: object.kind, text: object.text, frame: clone(object.frame), semantic: clone(metadata.objects[object.id] || {}) })) : undefined,
      })),
    };
  }

  function deckText(deck) {
    const parts = [];
    for (const slide of deck.slides || []) {
      visitObjects(slide.objects || [], (object) => {
        const text = String(object.text || "").trim();
        if (text) parts.push(text);
      });
      if (slide.notes) parts.push(String(slide.notes));
    }
    return parts.join("\n");
  }

  function create(adapter = {}, options = {}) {
    if (typeof adapter.getDeck !== "function" || typeof adapter.commit !== "function") throw new HostError("INVALID_ADAPTER", "Presentation Host 需要 getDeck 与 commit 适配器");
    const listeners = new Map();
    const plugins = new Map();
    const preparedHtmlImports = new Map();
    const emit = (event, payload) => {
      for (const handler of listeners.get(event) || []) {
        try { handler(payload); } catch (error) { console.error("[UniPPT host event]", error); }
      }
    };
    const on = (event, handler) => { if (typeof handler !== "function") return () => {}; const set = listeners.get(event) || new Set(); set.add(handler); listeners.set(event, set); return () => set.delete(handler); };
    const off = (event, handler) => listeners.get(event)?.delete(handler);

    // Mutation capabilities deliberately live only in this host closure.  The
    // public UniPpt/UniDoc object receives no issuer, so another same-origin
    // script cannot turn a direct `ai.callTool(...)` invocation into a write.
    // A capability authorizes exactly the approved tool names/counts, is
    // reserved before execution, and is consumed only after a successful
    // mutation.  Restoring a failed reservation lets an already-approved
    // deterministic retry (for example the HTML quality repair pass) complete
    // without presenting a second approval dialog.
    const aiMutationCapabilities = new WeakMap();
    const mutationAuthorizer = Object.freeze({
      issue(requests = []) {
        const remaining = new Map();
        for (const request of requests || []) {
          const name = String(request?.name || request || "");
          if (!name || AI_READ_ONLY_TOOLS.has(name) || !TOOL_DEFINITIONS.some((tool) => tool.name === name)) continue;
          remaining.set(name, (remaining.get(name) || 0) + 1);
        }
        if (!remaining.size) throw new HostError("AI_TOOL_AUTH_SCOPE_EMPTY", "批准批次没有可授权的 AI 修改工具");
        const capability = Object.freeze(Object.create(null));
        aiMutationCapabilities.set(capability, {
          remaining,
          pending: new Map(),
          expiresAt: Date.now() + AI_MUTATION_CAPABILITY_TTL_MS,
        });
        return capability;
      },
    });
    adapter.ai?.bindMutationAuthorizer?.(mutationAuthorizer);

    const reserveMutationCapability = (name, callOptions = {}) => {
      const capability = callOptions?.capability;
      if (!capability || (typeof capability !== "object" && typeof capability !== "function")) {
        throw new HostError("AI_TOOL_AUTH_REQUIRED", `AI 修改工具 ${name} 需要用户批准后签发的一次性 capability`);
      }
      const record = aiMutationCapabilities.get(capability);
      if (!record) throw new HostError("AI_TOOL_AUTH_INVALID", `AI 修改工具 ${name} 收到无效或已消耗的 capability`);
      if (Date.now() > record.expiresAt) {
        aiMutationCapabilities.delete(capability);
        throw new HostError("AI_TOOL_AUTH_EXPIRED", `AI 修改工具 ${name} 的批准已过期，请重新确认`);
      }
      const available = Number(record.remaining.get(name) || 0);
      if (available < 1) throw new HostError("AI_TOOL_AUTH_SCOPE_MISMATCH", `本次批准未授权 AI 修改工具 ${name}`);
      record.remaining.set(name, available - 1);
      record.pending.set(name, Number(record.pending.get(name) || 0) + 1);
      let settled = false;
      const settle = (committed) => {
        if (settled) return;
        settled = true;
        const pending = Math.max(0, Number(record.pending.get(name) || 0) - 1);
        if (pending) record.pending.set(name, pending);
        else record.pending.delete(name);
        if (!committed && Date.now() <= record.expiresAt) {
          record.remaining.set(name, Number(record.remaining.get(name) || 0) + 1);
        }
        const left = [...record.remaining.values()].reduce((sum, count) => sum + Number(count || 0), 0);
        const inFlight = [...record.pending.values()].reduce((sum, count) => sum + Number(count || 0), 0);
        if (!left && !inFlight) aiMutationCapabilities.delete(capability);
      };
      return settle;
    };

    const transact = async (changeSet) => {
      const revision = Number(adapter.getRevision?.() || 0);
      const outcome = applyChangeSet(adapter.getDeck(), changeSet, { ...options, revision });
      await adapter.commit(outcome.deck, outcome);
      const result = { ...outcome, deck: undefined, revision: Number(adapter.getRevision?.() ?? revision + 1) };
      emit("change", result);
      return result;
    };

    const executeTool = async (name, args = {}, callOptions = {}) => {
      if (name === "presentation.inspect") return summarize(adapter.getDeck(), args);
      if (name === "presentation.validate") return { issues: validateDeck(adapter.getDeck()), quality: auditDeckQuality(adapter.getDeck()) };
      if (name === "presentation.importFreeHtml") {
        for (const [id, prepared] of preparedHtmlImports) {
          if (Date.now() > prepared.expiresAt) preparedHtmlImports.delete(id);
        }
        const preparedId = String(args.preparedId || "");
        if (preparedId) {
          const prepared = preparedHtmlImports.get(preparedId);
          if (!prepared || Date.now() > prepared.expiresAt) {
            preparedHtmlImports.delete(preparedId);
            throw new HostError("HTML_PREPARE_EXPIRED", "自由 HTML 预编译结果已过期，请重新编译");
          }
          const destination = callOptions?.destination;
          const opensNewDocument = destination?.kind === "new-window";
          if (opensNewDocument && typeof adapter.openDocument !== "function") {
            throw new HostError("NEW_WINDOW_UNAVAILABLE", "当前编辑器不支持在新窗口打开生成的演示文稿");
          }
          if (!opensNewDocument) {
            const currentDeck = adapter.getDeck();
            const currentRevision = Number(adapter.getRevision?.() || 0);
            if (currentDeck !== prepared.baseDeck || currentRevision !== prepared.baseRevision) {
              const reason = currentDeck !== prepared.baseDeck
                ? "目标文稿已经切换"
                : `文稿已从 revision ${prepared.baseRevision} 更新到 ${currentRevision}`;
              throw new HostError("REVISION_CONFLICT", `HTML 预编译后${reason}，已阻止覆盖`);
            }
          }
          const commitOptions = opensNewDocument
            ? { replace: true, destination }
            : { replace: true, expectedRevision: prepared.baseRevision, expectedDeckRef: prepared.baseDeck };
          if (opensNewDocument) await adapter.openDocument(clone(prepared.deck), prepared.outcome, commitOptions);
          else await adapter.commit(clone(prepared.deck), prepared.outcome, commitOptions);
          preparedHtmlImports.delete(preparedId);
          const revision = opensNewDocument ? 0 : Number(adapter.getRevision?.() || 0);
          emit(opensNewDocument ? "open" : "load", { ...prepared.outcome, revision, destination: opensNewDocument ? "new-window" : "current-window" });
          return {
            ...prepared.outcome, revision, report: clone(prepared.outcome.report || {}),
            slideIds: prepared.deck.slides.map((slide) => slide.id),
            destination: opensNewDocument ? "new-window" : "current-window",
          };
        }
        const baseDeck = adapter.getDeck();
        const baseRevision = Number(adapter.getRevision?.() || 0);
        const sourceHtml = String(args.sourceHtml || "");
        if (!sourceHtml.trim()) throw new HostError("HTML_ATTACHMENT_MISSING", "自由 HTML 工具没有收到对应附件源码");
        const maximumHtmlBytes = args.generatedByAi ? 2 * 1024 * 1024 : 16 * 1024 * 1024;
        if (utf8Bytes(sourceHtml) > maximumHtmlBytes) throw new HostError("HTML_ATTACHMENT_TOO_LARGE", `自由 HTML 源码超过 ${args.generatedByAi ? 2 : 16} MiB UTF-8 转换上限`);
        const converter = global.UniPptHtmlToPpt;
        if (typeof converter?.convert !== "function") throw new HostError("HTML_CONVERTER_MISSING", "自由 HTML 转 PPT 运行时未加载");
        const compiled = await converter.convert(sourceHtml, {
          title: args.title, sourceName: args.attachmentName,
          width: Math.max(1, finite(args.width, 1280)), height: Math.max(1, finite(args.height, 720)),
          restrictNetwork: Boolean(args.generatedByAi),
        });
        const width = Math.max(1, finite(compiled.width, 1280));
        const height = Math.max(1, finite(compiled.height, 720));
        const deck = {
          format: "unippt", version: 1, title: String(compiled.title || args.title || args.attachmentName || "自由 HTML"),
          width, height, sourceWidthEmu: Math.round(width * 9525), sourceHeightEmu: Math.round(height * 9525),
          sourceImportId: null, fonts: [], extensions: {}, slides: [],
        };
        const specs = compiled.slides || [];
        const expectedPageCount = Number(args.pageCount || 0);
        const reportedPageCount = Number(compiled.report?.pageCount || specs.length);
        if (args.generatedByAi && expectedPageCount > 0 && (specs.length !== expectedPageCount || reportedPageCount !== expectedPageCount)) {
          throw new HostError("AI_HTML_PAGE_COUNT_MISMATCH", `预检为 ${expectedPageCount} 页，但浏览器编译得到 ${specs.length} 页（report=${reportedPageCount}）`);
        }
        for (const spec of specs) deck.slides.push(makeSlide(spec, deck, options));
        if (!deck.slides.length) throw new HostError("HTML_EMPTY", "自由 HTML 没有产生可转换页面");
        enrichSemantics(deck, options);
        const metadata = extension(deck);
        metadata.document = {
          summary: String(args.centralTakeaway || `由自由 HTML ${args.attachmentName || ""} 编译`),
          language: args.language ? String(args.language) : (args.generatedByAi ? "zh-CN" : null),
          audience: String(args.audience || ""), purpose: String(args.purpose || ""),
        };
        metadata.designTokens = {
          generator: args.generatedByAi ? "ai-html-composer-v1" : "html-dom-compiler-v1",
          source: String(args.attachmentName || ""),
          minimumFontSizes: { deckTitle: 50, slideTitle: 35, body: 16 },
        };
        metadata.importReport = clone(compiled.report || {});
        deck.slides.forEach((slide, index) => {
          const semantic = specs[index]?.semantic || {};
          metadata.slides[slide.id] = {
            ...(metadata.slides[slide.id] || {}),
            role: String(semantic.role || (index === 0 ? "cover" : index === deck.slides.length - 1 ? "closing" : "content")),
            summary: String(semantic.summary || slide.name || ""),
          };
          const applyObjectSemantics = (sourceObjects, targetObjects) => {
            (targetObjects || []).forEach((object, objectIndex) => {
              const source = sourceObjects?.[objectIndex] || {};
              const role = String(source.semanticRole || "").trim();
              if (role) metadata.objects[object.id] = { ...(metadata.objects[object.id] || {}), role, label: String(object.name || "") };
              applyObjectSemantics(source.children, object.children);
            });
          };
          applyObjectSemantics(specs[index]?.objects, slide.objects);
        });
        const issues = validateDeck(deck);
        const errors = issues.filter((issue) => issue.severity === "error");
        if (errors.length) throw new HostError("VALIDATION_FAILED", "自由 HTML 转换结果无效", errors);
        const quality = auditDeckQuality(deck);
        if (args.generatedByAi) {
          const report = compiled.report || {};
          const reportErrors = [];
          if (String(report.fidelityRisk || "").toLowerCase() === "high") reportErrors.push({ severity: "error", code: "HTML_FIDELITY_RISK_HIGH", message: "HTML 编译保真风险为 high" });
          if (Array.isArray(report.unsupported) && report.unsupported.length) reportErrors.push({ severity: "error", code: "HTML_UNSUPPORTED_CONTENT", message: `HTML 编译仍有 ${report.unsupported.length} 个不支持项` });
          if (!quality.passed || reportErrors.length) {
            const details = [...quality.findings, ...reportErrors];
            const summary = details.slice(0, 6).map((finding) => `${finding.code}${Number.isInteger(finding.slideIndex) ? `@${finding.slideIndex + 1}` : ""}`).join("、");
            throw new HostError("AI_HTML_QUALITY_FAILED", `AI 自由 HTML 未通过整稿质量门槛${summary ? `：${summary}` : ""}`, details);
          }
        }
        const currentDeck = adapter.getDeck();
        const currentRevision = Number(adapter.getRevision?.() || 0);
        if (currentDeck !== baseDeck || currentRevision !== baseRevision) {
          const reason = currentDeck !== baseDeck
            ? "目标文稿已经切换"
            : `文稿已从 revision ${baseRevision} 更新到 ${currentRevision}`;
          throw new HostError("REVISION_CONFLICT", `HTML 编译期间${reason}，已阻止覆盖`);
        }
        const outcome = {
          transactionId: uid("tx", options), baseRevision,
          activeSlideId: deck.slides[0].id, selectedObjectId: null, selectedAnimationId: null,
          applied: [args.generatedByAi ? "composeFromHtml" : "importFreeHtml"], issues, quality, report: clone(compiled.report || {}),
        };
        if (args.prepareOnly) {
          const preparedId = uid("html-prepared", options);
          preparedHtmlImports.set(preparedId, {
            deck, outcome, baseDeck, baseRevision,
            expiresAt: Date.now() + AI_MUTATION_CAPABILITY_TTL_MS,
          });
          return {
            preparedId, title: deck.title, slideCount: deck.slides.length,
            slideIds: deck.slides.map((slide) => slide.id), quality,
            report: clone(compiled.report || {}),
          };
        }
        await adapter.commit(deck, outcome, { replace: true, expectedRevision: baseRevision, expectedDeckRef: baseDeck });
        emit("load", { ...outcome, revision: Number(adapter.getRevision?.() || 0) });
        return { ...outcome, revision: Number(adapter.getRevision?.() || 0), slideIds: deck.slides.map((slide) => slide.id) };
      }
      if (name === "presentation.compose") {
        const deck = composeExecutiveDeck(args, options);
        const issues = validateDeck(deck);
        const errors = issues.filter((issue) => issue.severity === "error");
        if (errors.length) throw new HostError("VALIDATION_FAILED", "AI 编排的演示文稿无效", errors);
        const quality = auditDeckQuality(deck);
        const outcome = {
          transactionId: uid("tx", options), baseRevision: Number(adapter.getRevision?.() || 0),
          activeSlideId: deck.slides[0].id, selectedObjectId: null, selectedAnimationId: null,
          applied: ["composePresentation"], issues, quality,
        };
        await adapter.commit(deck, outcome, { replace: true });
        emit("load", { ...outcome, revision: Number(adapter.getRevision?.() || 0) });
        return { ...outcome, revision: Number(adapter.getRevision?.() || 0), slideIds: deck.slides.map((slide) => slide.id) };
      }
      if (name === "presentation.create") {
        const width = Math.max(1, finite(args.width, 1280));
        const height = Math.max(1, finite(args.height, 720));
        const deck = { format: "unippt", version: 1, title: String(args.title || "演示文稿"), width, height, sourceWidthEmu: Math.round(width * 9525), sourceHeightEmu: Math.round(height * 9525), sourceImportId: null, fonts: [], extensions: {}, slides: [] };
        for (const spec of args.slides || []) deck.slides.push(makeSlide(spec, deck, options));
        if (!deck.slides.length) deck.slides.push(makeSlide({ title: args.title || "演示文稿" }, deck, options));
        enrichSemantics(deck, options);
        const issues = validateDeck(deck);
        if (issues.some((issue) => issue.severity === "error")) throw new HostError("VALIDATION_FAILED", "AI 生成的演示文稿无效", issues);
        const outcome = { transactionId: uid("tx", options), baseRevision: Number(adapter.getRevision?.() || 0), activeSlideId: deck.slides[0].id, selectedObjectId: null, selectedAnimationId: null, applied: ["createPresentation"], issues };
        await adapter.commit(deck, outcome, { replace: true });
        emit("load", { ...outcome, revision: Number(adapter.getRevision?.() || 0) });
        return { ...outcome, revision: Number(adapter.getRevision?.() || 0), slideIds: deck.slides.map((slide) => slide.id) };
      }
      if (name === "slide.reconstructFromImage") {
        const deck = adapter.getDeck();
        const sourceImage = String(args.sourceImage || "");
        if (!/^data:image\//i.test(sourceImage)) throw new HostError("IMAGE_ATTACHMENT_MISSING", "图片重建工具没有收到对应的图片附件");
        if (sourceImage.length > 18 * 1024 * 1024) throw new HostError("IMAGE_ATTACHMENT_TOO_LARGE", "图片重建附件超过安全大小限制");
        const spec = args.slide || {};
        if (spec.mode === "native") {
          const compiled = compileNativeImageSlide(args, deck);
          const result = await transact({ transactionId: uid("tx", options), baseRevision: adapter.getRevision?.(),
            operations: [{ op: "insertSlide", afterSlideId: args.afterSlideId || adapter.getSelection?.()?.slideId || deck.slides.at(-1)?.id, slide: compiled.slide }] });
          return { ...result, slideId: result.activeSlideId, preservedSlideCount: deck.slides.length, report: compiled.report,
            editableObjectCount: compiled.report.nativeText + compiled.report.nativeShapes + compiled.report.nativeConnectors,
            completionText: `已生成原生重建页：${compiled.report.nativeText} 个文本、${compiled.report.nativeShapes} 个形状、${compiled.report.nativeConnectors} 条连接线、${compiled.report.rotatedText} 个旋转文本、${compiled.report.photos} 张局部照片。${compiled.report.fallbackTextCount ? `其中 ${compiled.report.fallbackTextCount} 个 OCR 文本待校对。` : ""}结构检查通过，仍需对照原图检查渲染与遗漏。` };
        }
        const detectedRegions = Array.isArray(args.ocrDetection?.regions) ? args.ocrDetection.regions : [];
        const cleanedImage = String(args.ocrDetection?.cleanedImage || "");
        const hasCleanedBackground = /^data:image\//i.test(cleanedImage) && cleanedImage.length <= 18 * 1024 * 1024;
        if (detectedRegions.length && !hasCleanedBackground) {
          const reason = String(args.ocrDetection?.cleaning?.fallbackReason || "Big LaMa 无字底图缺失或超过安全大小限制");
          throw new HostError("IMAGE_BACKGROUND_REPAIR_MISSING", `OCR 已完成，但 AI 擦字背景不可用；已阻止生成低质量色块遮罩。${reason}`);
        }
        if ((!Array.isArray(spec.objects) || !spec.objects.length) && !detectedRegions.length) throw new HostError("EMPTY_RECONSTRUCTION", "图片重建至少需要一个 OCR 文本或可编辑图层");
        const width = Math.max(1, finite(deck.width, 1280));
        const height = Math.max(1, finite(deck.height, 720));
        const background = String(spec.textRegionFill || spec.background || "#0b1020");
        const backgroundMatch = background.match(/^#([0-9a-f]{6})$/i);
        const backgroundRgb = backgroundMatch
          ? [0, 2, 4].map((offset) => Number.parseInt(backgroundMatch[1].slice(offset, offset + 2), 16))
          : [11, 16, 32];
        const defaultTextColor = backgroundRgb[0] * .299 + backgroundRgb[1] * .587 + backgroundRgb[2] * .114 < 150 ? "#ffffff" : "#172033";
        const normalizedText = (value) => String(value || "").normalize?.("NFKC").replace(/[\s·•・，。,:：;；'"“”‘’()（）\-_]/g, "").toLowerCase() || "";
        const modelTextLayers = (spec.objects || []).filter((object) => String(object.kind || "") === "text");
        const usedModelText = new Set();
        const sourceWidth = Math.max(1, finite(args.ocrDetection?.image?.width, 1));
        const sourceHeight = Math.max(1, finite(args.ocrDetection?.image?.height, 1));
        const authoritativeTextLayers = detectedRegions.map((region, index) => {
          const location = Array.isArray(region.location) ? region.location.map(Number) : [];
          if (location.length !== 8 || location.some((value) => !Number.isFinite(value)) || !String(region.text || "").trim()) return null;
          const points = [0, 2, 4, 6].map((offset) => ({ x: location[offset] / sourceWidth * width, y: location[offset + 1] / sourceHeight * height }));
          const center = points.reduce((result, point) => ({ x: result.x + point.x / 4, y: result.y + point.y / 4 }), { x: 0, y: 0 });
          const boxWidth = Math.max(1, Math.hypot(points[1].x - points[0].x, points[1].y - points[0].y));
          const boxHeight = Math.max(1, Math.hypot(points[3].x - points[0].x, points[3].y - points[0].y));
          const rotation = Math.atan2(points[1].y - points[0].y, points[1].x - points[0].x) * 180 / Math.PI;
          const target = normalizedText(region.text);
          let matchedIndex = modelTextLayers.findIndex((layer, candidateIndex) => !usedModelText.has(candidateIndex) && normalizedText(layer.text) === target);
          if (matchedIndex < 0) matchedIndex = modelTextLayers.findIndex((layer, candidateIndex) => {
            const candidate = normalizedText(layer.text);
            return !usedModelText.has(candidateIndex) && candidate && target && (candidate.includes(target) || target.includes(candidate));
          });
          if (matchedIndex >= 0) usedModelText.add(matchedIndex);
          const matched = matchedIndex >= 0 ? clone(modelTextLayers[matchedIndex]) : {};
          const frameWidth = boxWidth * 1.04;
          const frameHeight = boxHeight * 1.16;
          const text = String(region.text);
          const heightFit = boxHeight * .86;
          const widthFit = frameWidth / (weightedGlyphCount(text) * .92);
          const authoritativeFontSize = Math.max(7, Math.min(heightFit, widthFit * 1.04));
          const matchedStyle = clone(matched.textStyle || {});
          const matchedTextFrame = clone(matched.textFrame || {});
          const maskLocation = Array.isArray(region.maskLocation) ? region.maskLocation.map(Number) : [];
          const maskPoints = maskLocation.length === 8 && maskLocation.every(Number.isFinite)
            ? [0, 2, 4, 6].map((offset) => ({ x: maskLocation[offset] / sourceWidth * width, y: maskLocation[offset + 1] / sourceHeight * height }))
            : [];
          const maskCenter = maskPoints.length
            ? maskPoints.reduce((result, point) => ({ x: result.x + point.x / 4, y: result.y + point.y / 4 }), { x: 0, y: 0 })
            : null;
          const authoritativeStyle = {
            ...matchedStyle,
            fontFamily: String(matchedStyle.fontFamily || "Microsoft YaHei, Noto Sans CJK SC, Arial, sans-serif"),
            color: String(region.foreground || defaultTextColor),
            fontSize: Math.round(authoritativeFontSize * 100) / 100,
            bold: boxHeight >= height * .046 || (text.length <= 8 && boxHeight >= height * .031),
            align: String(matchedStyle.align || matchedStyle.textAlign || "left"),
          };
          return {
            ...matched,
            kind: "text",
            name: String(matched.name || `OCR · ${text.slice(0, 24)}`),
            text,
            frame: { x: center.x - frameWidth / 2, y: center.y - frameHeight / 2, width: frameWidth, height: frameHeight, rotation },
            textStyle: authoritativeStyle,
            textFrame: { ...matchedTextFrame, marginLeft: 0, marginRight: 0, marginTop: 0, marginBottom: 0, verticalAlign: "center", verticalType: "horz", wordWrap: false, autoSize: "none" },
            _ocrFill: String(region.background || background),
            _ocrIndex: index,
            _ocrMaskFrame: maskCenter ? {
              x: maskCenter.x - Math.hypot(maskPoints[1].x - maskPoints[0].x, maskPoints[1].y - maskPoints[0].y) / 2,
              y: maskCenter.y - Math.hypot(maskPoints[3].x - maskPoints[0].x, maskPoints[3].y - maskPoints[0].y) / 2,
              width: Math.max(1, Math.hypot(maskPoints[1].x - maskPoints[0].x, maskPoints[1].y - maskPoints[0].y)),
              height: Math.max(1, Math.hypot(maskPoints[3].x - maskPoints[0].x, maskPoints[3].y - maskPoints[0].y)),
              rotation: Math.atan2(maskPoints[1].y - maskPoints[0].y, maskPoints[1].x - maskPoints[0].x) * 180 / Math.PI,
            } : null,
          };
        }).filter(Boolean);
        const sourceLayers = authoritativeTextLayers.length
          ? [...(spec.objects || []).filter((object) => String(object.kind || "") !== "text"), ...authoritativeTextLayers]
          : (spec.objects || []);
        const editableLayers = sourceLayers.map((object) => {
          const layer = clone(object);
          if (String(layer.kind || "") !== "text") return layer;
          const frameHeight = Math.max(1, finite(layer.frame?.height, 48));
          const frameWidth = Math.max(1, finite(layer.frame?.width, 240));
          const text = String(layer.text || "");
          const suppliedStyle = clone(layer.textStyle || {});
          const authoritativeOcr = layer._ocrIndex != null;
          const minimumFontSize = authoritativeOcr ? 7 : 12;
          const preferredFontSize = Math.max(minimumFontSize, finite(suppliedStyle.fontSize, Math.max(16, Math.min(144, Math.round(frameHeight * .7)))));
          const weightedGlyphs = weightedGlyphCount(text);
          const widthFitFontSize = Math.max(12, Math.floor(frameWidth / (weightedGlyphs * 1.04)));
          layer.textStyle = {
            fontFamily: "Microsoft YaHei, SimHei, Aptos, sans-serif",
            color: defaultTextColor,
            bold: frameHeight >= 50 || String(layer.text || "").length <= 10,
            align: "left",
            ...suppliedStyle,
            fontSize: authoritativeOcr ? Math.min(144, preferredFontSize) : Math.min(144, preferredFontSize, widthFitFontSize),
          };
          layer.textFrame = { ...(layer.textFrame || {}), autoSize: authoritativeOcr ? "none" : "textToFitShape" };
          return layer;
        });
        const objects = [];
        const preservesFullSource = spec.preserveSourceImage !== false;
        if (preservesFullSource) objects.push({
          kind: "image",
          name: `${hasCleanedBackground ? "文字已擦除底图" : "保真底图"} · ${String(args.attachmentName || "图片")}`,
          frame: { x: 0, y: 0, width, height },
          asset: hasCleanedBackground ? cleanedImage : sourceImage,
          style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: 1 },
        });
        // A crop from the same source cannot add fidelity when the complete
        // source is already the bottom layer. Keeping both creates the exact
        // duplicate-image failure seen in live reconstruction. Crops remain
        // available as ordinary draggable image objects only for layouts that
        // explicitly opt out of the full-page source layer.
        const sourceCrops = preservesFullSource ? [] : (spec.sourceCrops || []);
        for (const crop of sourceCrops) objects.push({
          kind: "image",
          name: String(crop.name || "原图裁剪层"),
          frame: clone(crop.frame || {}),
          asset: sourceImage,
          imageCrop: clone(crop.crop || {}),
          style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: 1, ...(clone(crop.style || {})) },
        });
        if (spec.autoMaskText === false && !authoritativeTextLayers.length) {
          for (const mask of spec.occlusions || []) objects.push({
            kind: "shape",
            name: String(mask.name || "文字区域遮罩"),
            frame: clone(mask.frame || {}),
            geometry: "rect",
            style: { fill: String(spec.background || "#0b1020"), stroke: "transparent", strokeWidth: 0, ...(clone(mask.style || {})), opacity: 1 },
          });
        } else if (!authoritativeTextLayers.length) {
          for (const layer of editableLayers) {
            if (String(layer.kind || "") !== "text" || !String(layer.text || "").trim()) continue;
            const frame = layer.frame || {};
            const frameHeight = Math.max(1, finite(frame.height, 1));
            // Vision bounding boxes are approximate. Generous vertical bleed
            // removes source glyphs above/below the editable replacement while
            // keeping complex artwork to the left and right untouched.
            const paddingX = Math.max(8, Math.round(Math.min(width, height) * .01));
            const paddingY = Math.max(18, Math.round(frameHeight * 1.35));
            const frameWidth = Math.max(1, finite(frame.width, 1));
            const maskWidth = Math.min(frameWidth, Math.max(frameWidth * .85, Array.from(String(layer.text || "")).reduce((sum, character) => sum + (character.charCodeAt(0) > 255 ? layer.textStyle.fontSize * .95 : layer.textStyle.fontSize * .62), 0)));
            const x = Math.max(0, finite(frame.x) - paddingX);
            const y = Math.max(0, finite(frame.y) - paddingY);
            const right = Math.min(width, finite(frame.x) + maskWidth + paddingX);
            const bottom = Math.min(height, finite(frame.y) + frameHeight + paddingY);
            objects.push({
              kind: "shape",
              name: `文字击穿遮罩 · ${String(layer.name || layer.text).slice(0, 24)}`,
              frame: { x, y, width: Math.max(1, right - x), height: Math.max(1, bottom - y) },
              geometry: "rect",
              style: { fill: background, stroke: "transparent", strokeWidth: 0, opacity: 1 },
            });
          }
        }
        objects.push(...editableLayers.map((layer) => {
          const cleanLayer = clone(layer);
          delete cleanLayer._ocrFill;
          delete cleanLayer._ocrIndex;
          delete cleanLayer._ocrMaskFrame;
          return cleanLayer;
        }));
        const slide = {
          name: String(spec.name || `图片重建 · ${args.attachmentName || "未命名"}`),
          background: String(spec.background || "#0b1020"),
          notes: String(spec.notes || `由图片附件 ${args.attachmentName || ""} 进行混合结构化重建；底图保真，文本和覆盖层可编辑。`),
          objects,
          transition: clone(spec.transition ?? null),
        };
        const selectionSlideId = adapter.getSelection?.()?.slideId;
        const afterSlideId = args.afterSlideId || selectionSlideId || deck.slides.at(-1)?.id;
        const result = await transact({
          transactionId: uid("tx", options),
          baseRevision: adapter.getRevision?.(),
          operations: [{ op: "insertSlide", afterSlideId, slide, summary: `图片重建：${args.attachmentName || "图片"}` }],
        });
        return {
          ...result,
          slideId: result.activeSlideId,
          preservedSlideCount: deck.slides.length,
          editableObjectCount: editableLayers.length,
          ocrRegionCount: authoritativeTextLayers.length,
          suppressedSourceCropCount: preservesFullSource ? (spec.sourceCrops || []).length : 0,
          backgroundRepair: hasCleanedBackground ? clone(args.ocrDetection?.cleaning || { strategy: "browser-local" }) : { strategy: "overlay-mask-fallback" },
        };
      }
      const operationByTool = {
        "slide.add": { op: "insertSlide", ...args },
        "slide.update": { op: "updateSlide", ...args },
        "slide.remove": { op: "removeSlide", ...args },
        "object.add": { op: "addObject", ...args },
        "object.update": { op: "updateObject", ...args },
        "object.setText": { op: "updateText", ...args },
        "object.remove": { op: "removeObject", ...args },
        "animation.add": { op: "addAnimation", ...args },
        "transition.set": { op: "setTransition", ...args },
      };
      if (name === "presentation.applyChangeSet") return transact(args);
      if (!operationByTool[name]) throw new HostError("TOOL_NOT_FOUND", `未知 AI 工具：${name}`);
      const result = await transact({ transactionId: uid("tx", options), baseRevision: adapter.getRevision?.(), operations: [operationByTool[name]] });
      if (name === "slide.add") return { ...result, slideId: result.activeSlideId };
      if (name === "object.add") return { ...result, objectId: result.selectedObjectId };
      if (name === "animation.add") return { ...result, animationId: result.selectedAnimationId };
      return result;
    };

    const callTool = async (name, args = {}, callOptions = {}) => {
      const toolName = String(name || "");
      if (AI_READ_ONLY_TOOLS.has(toolName)) return executeTool(toolName, args);
      const settleCapability = reserveMutationCapability(toolName, callOptions);
      try {
        const result = await executeTool(toolName, args, callOptions);
        settleCapability(true);
        return result;
      } catch (error) {
        settleCapability(false);
        throw error;
      }
    };

    const api = {
      version: HOST_VERSION,
      unidoc_type: DOCUMENT_TYPE,
      capabilities: Object.freeze(["structured-scene", "atomic-changeset", "undo-redo", "semantic-index", "animations", "transitions", "loss-aware-pptx", "lossless-html-v4", "udoc", "ai-tools", "plugins"]),
      document: {
        type: DOCUMENT_TYPE,
        getRevision: () => Number(adapter.getRevision?.() || 0),
        getSnapshot: () => clone(adapter.getDeck()),
        getSummary: (args) => summarize(adapter.getDeck(), args),
        transact,
        validate: () => validateDeck(adapter.getDeck()),
      },
      // Compatibility surface shared with generic UniDoc plugins. Word-only
      // plugins can reject `unidoc_type: pptx`; document-agnostic plugins can
      // keep using getUdoc/getText/pageCount without branching.
      doc: {
        getUdoc: () => clone(adapter.getDeck()),
        getText: () => deckText(adapter.getDeck()),
        pageCount: () => adapter.getDeck()?.slides?.length || 0,
        transact,
      },
      slides: {
        list: () => summarize(adapter.getDeck()).slides,
        add: (args) => executeTool("slide.add", args),
        update: (args) => executeTool("slide.update", args),
        remove: (slideId) => executeTool("slide.remove", { slideId }),
      },
      objects: {
        add: (args) => executeTool("object.add", args),
        update: (args) => executeTool("object.update", args),
        setText: (args) => executeTool("object.setText", args),
        remove: (objectId, slideId) => executeTool("object.remove", { objectId, slideId }),
      },
      selection: {
        get: () => clone(adapter.getSelection?.() || {}),
        getText: () => {
          const selection = adapter.getSelection?.() || {};
          if (!selection.objectId) return "";
          try { return String(findObject(adapter.getDeck(), selection.objectId, selection.slideId).object.text || ""); } catch (_) { return ""; }
        },
        set: (selection) => adapter.setSelection?.(clone(selection || {})),
      },
      exportAs: {
        udoc: (...args) => adapter.exportAs?.udoc?.(...args),
        pptx: (...args) => adapter.exportAs?.pptx?.(...args),
        html: (...args) => adapter.exportAs?.html?.(...args),
      },
      ui: adapter.ui || {},
      ai: {
        previewImageReconstruction: (args = {}) => compileNativeImageSlide(args, adapter.getDeck()).report,
        tools: TOOL_DEFINITIONS,
        callTool,
        prepareFreeHtml: (args = {}) => executeTool("presentation.importFreeHtml", { ...args, prepareOnly: true }),
        open: (prefill) => adapter.ai?.open?.(prefill) ?? emit("ai-open", { prefill: String(prefill || "") }),
        ask: async (prompt, askOptions = {}) => {
          if (typeof adapter.ai?.ask === "function") return adapter.ai.ask(prompt, askOptions);
          if (typeof global.UniPptAiProvider?.ask === "function") return global.UniPptAiProvider.ask(prompt, askOptions);
          throw new HostError("AI_PROVIDER_MISSING", "尚未安装 AI 模型提供器插件");
        },
      },
      exec: (command, value) => adapter.exec?.(command, value),
      on, off,
      _emit: emit,
      registerPlugin(manifest) {
        if (!manifest?.id || typeof manifest.id !== "string") throw new HostError("INVALID_PLUGIN", "插件必须声明唯一 id");
        if (plugins.has(manifest.id)) throw new HostError("PLUGIN_EXISTS", `插件已存在：${manifest.id}`);
        const supported = !Array.isArray(manifest.supportedTypes) || manifest.supportedTypes.includes(DOCUMENT_TYPE);
        const record = { manifest, enabled: false, compatible: supported, cleanup: null };
        plugins.set(manifest.id, record);
        if (supported && manifest.enabledByDefault !== false) api.plugins.enable(manifest.id);
        emit("plugins", api.plugins.list());
        return record;
      },
      plugins: {
        list: () => [...plugins.values()].map((record) => ({ id: record.manifest.id, name: record.manifest.name || record.manifest.id, version: record.manifest.version || "0", enabled: record.enabled, compatible: record.compatible, supportedTypes: record.manifest.supportedTypes || ["docs", "pptx"] })),
        enable(id) {
          const record = plugins.get(id);
          if (!record) throw new HostError("PLUGIN_NOT_FOUND", `找不到插件 ${id}`);
          if (!record.compatible) throw new HostError("PLUGIN_INCOMPATIBLE", `插件 ${id} 不支持 pptx 文档`);
          if (record.enabled) return;
          record.enabled = true;
          try { record.cleanup = record.manifest.activate?.(api) || null; } catch (error) { record.enabled = false; throw error; }
          emit("plugins", api.plugins.list());
        },
        disable(id) {
          const record = plugins.get(id);
          if (!record?.enabled) return;
          record.enabled = false;
          try { record.cleanup?.(); record.manifest.deactivate?.(); } finally { adapter.ui?.removePluginContributions?.(id); record.cleanup = null; }
          emit("plugins", api.plugins.list());
        },
      },
    };
    return Object.freeze(api);
  }

  global.UniPptPresentationHost = Object.freeze({
    HOST_VERSION, DOCUMENT_TYPE, AI_EXTENSION, HostError,
    create, applyChangeSet, validateDeck, nativeTextIssues, prepareNativeExport, auditDeckQuality, composeExecutiveDeck, enrichSemantics, inferObjectRole,
    toolDefinitions: TOOL_DEFINITIONS, compileNativeImageSlide, compileNativeImageDeck,
  });
})(globalThis);
