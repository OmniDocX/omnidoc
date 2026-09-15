(function installHtmlToPptRuntime(global) {
  "use strict";

  const DEFAULT_WIDTH = 1280;
  const DEFAULT_HEIGHT = 720;
  const PAGE_SELECTORS = [
    ".omnidoc-page", "[data-omnidoc-page]", ".slides > section", ".slides > .slide",
    ".slides-container > .slide", ".deck > .slide", "section.slide", "[data-slide]", "[data-page]",
  ];
  const SKIP_TAGS = new Set(["SCRIPT", "STYLE", "LINK", "META", "HEAD", "NOSCRIPT", "TEMPLATE"]);
  const TEXT_CONTROL_TAGS = new Set(["INPUT", "TEXTAREA", "SELECT", "OPTION", "BUTTON"]);
  const CSS_VIRTUAL_FONTS = new Set([
    "-apple-system", "blinkmacsystemfont", "system-ui", "ui-sans-serif", "ui-serif", "ui-monospace",
    "sans-serif", "serif", "monospace", "cursive", "fantasy", "math", "emoji", "fangsong",
  ]);
  const WEB_FONT_FALLBACKS = new Map([
    ["inter", "Segoe UI"], ["roboto", "Segoe UI"], ["helvetica", "Arial"], ["helvetica neue", "Arial"],
    ["sf pro display", "Segoe UI"], ["sf pro text", "Segoe UI"], ["arial", "Arial"],
    ["calibri", "Calibri"], ["aptos", "Aptos"], ["segoe ui", "Segoe UI"],
  ]);
  const EAST_ASIAN_FONT_NAMES = new Map([
    ["microsoft yahei", "Microsoft YaHei"], ["microsoft yahei ui", "Microsoft YaHei UI"], ["微软雅黑", "Microsoft YaHei"],
    ["dengxian", "DengXian"], ["等线", "DengXian"], ["simhei", "SimHei"], ["黑体", "SimHei"],
    ["simsun", "SimSun"], ["宋体", "SimSun"], ["noto sans sc", "Noto Sans SC"],
    ["noto sans cjk sc", "Noto Sans CJK SC"], ["source han sans sc", "Source Han Sans SC"],
    ["yu gothic", "Yu Gothic"], ["yu mincho", "Yu Mincho"], ["meiryo", "Meiryo"],
    ["ms gothic", "MS Gothic"], ["noto sans jp", "Noto Sans JP"], ["noto serif jp", "Noto Serif JP"],
    ["malgun gothic", "Malgun Gothic"], ["apple sd gothic neo", "Apple SD Gothic Neo"],
    ["noto sans kr", "Noto Sans KR"], ["noto serif kr", "Noto Serif KR"],
  ]);

  function number(value, fallback = 0) {
    const parsed = Number.parseFloat(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  function clamp(value, minimum, maximum) {
    return Math.max(minimum, Math.min(maximum, value));
  }

  function colorByte(value) {
    return Math.round(clamp(number(value), 0, 255)).toString(16).padStart(2, "0").toUpperCase();
  }

  function colorAlpha(value, fallback = 1) {
    const color = String(value || "").trim();
    if (!color) return fallback;
    if (color === "transparent") return 0;
    const hex = color.match(/^#([0-9a-f]{3,8})$/i)?.[1];
    if (hex?.length === 4) return Number.parseInt(hex[3] + hex[3], 16) / 255;
    if (hex?.length === 8) return Number.parseInt(hex.slice(6), 16) / 255;
    const functional = color.match(/^(?:rgba?|hsla?|color)\([^)]*(?:,|\/)\s*(-?(?:\d+\.?\d*|\.\d+)%?)\s*\)$/i);
    if (!functional) return fallback;
    const token = functional[1];
    return clamp(number(token) / (/%$/.test(token) ? 100 : 1), 0, 1);
  }

  function visibleColor(value, fallback = "transparent") {
    const color = String(value || "").trim();
    if (!color || color === "rgba(0, 0, 0, 0)" || color === "transparent") return fallback;
    const hex = color.match(/^#([0-9a-f]{3,8})$/i)?.[1];
    if (hex) {
      const rgb = hex.length === 3 || hex.length === 4
        ? hex.slice(0, 3).split("").map((digit) => digit + digit).join("")
        : hex.slice(0, 6);
      return `#${rgb.toUpperCase()}`;
    }
    const rgb = color.match(/^rgba?\(\s*([\d.]+)%?[,\s]+([\d.]+)%?[,\s]+([\d.]+)%?/i);
    if (rgb) {
      const percentages = /%/.test(rgb[0]);
      const scale = percentages ? 2.55 : 1;
      return `#${colorByte(number(rgb[1]) * scale)}${colorByte(number(rgb[2]) * scale)}${colorByte(number(rgb[3]) * scale)}`;
    }
    const srgb = color.match(/^color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)/i);
    if (srgb) return `#${colorByte(number(srgb[1]) * 255)}${colorByte(number(srgb[2]) * 255)}${colorByte(number(srgb[3]) * 255)}`;
    return fallback;
  }

  function compositeColor(value, backgroundValue, fallback = "#000000") {
    const foreground = visibleColor(value, fallback);
    const background = visibleColor(backgroundValue, "#FFFFFF");
    const alpha = colorAlpha(value, 1);
    if (alpha >= .999 || foreground === "transparent") return foreground;
    const parse = (color) => {
      const match = String(color).match(/^#([0-9A-F]{2})([0-9A-F]{2})([0-9A-F]{2})$/i);
      return match ? match.slice(1).map((component) => Number.parseInt(component, 16)) : [0, 0, 0];
    };
    const front = parse(foreground), back = parse(background);
    return `#${front.map((component, index) => colorByte(component * alpha + back[index] * (1 - alpha))).join("")}`;
  }

  function cssOpacity(style) {
    return clamp(number(style?.opacity, 1), 0, 1);
  }

  function visualBoxOpacity(style) {
    const image = String(style?.backgroundImage || "none");
    if (/^(?:linear|radial)-gradient\(/i.test(image)) return cssOpacity(style);
    const fillAlpha = colorAlpha(style?.backgroundColor, 0);
    if (fillAlpha > 0) return cssOpacity(style) * fillAlpha;
    return cssOpacity(style) * colorAlpha(style?.borderTopColor, 1);
  }

  function rectangle(rect, root) {
    return {
      x: rect.left - root.left,
      y: rect.top - root.top,
      width: rect.width,
      height: rect.height,
    };
  }

  function intersecting(rect, root) {
    return rect.width > .5 && rect.height > .5
      && rect.right > root.left && rect.bottom > root.top
      && rect.left < root.right && rect.top < root.bottom;
  }

  function computedRotation(transform) {
    const match = String(transform || "").match(/^matrix\(([^)]+)\)$/);
    if (!match) return 0;
    const values = match[1].split(",").map(Number);
    if (values.length < 2 || values.some((value) => !Number.isFinite(value))) return 0;
    return Math.atan2(values[1], values[0]) * 180 / Math.PI;
  }

  function boxShadow(style) {
    const value = String(style?.boxShadow || "");
    if (!value || value === "none" || value.includes("inset")) return null;
    const color = value.match(/rgba?\([^)]*\)|#[0-9a-f]{3,8}/i)?.[0] || "#000000";
    const lengths = value.replace(color, "").match(/-?\d+(?:\.\d+)?px/g)?.map(number) || [];
    if (lengths.length < 2) return null;
    return {
      color: visibleColor(color, "#000000"),
      opacity: colorAlpha(color, .24),
      offsetX: lengths[0], offsetY: lengths[1], blur: Math.max(0, lengths[2] || 0), inset: false,
    };
  }

  function border(style) {
    const widths = [style?.borderTopWidth, style?.borderRightWidth, style?.borderBottomWidth, style?.borderLeftWidth].map(number);
    const width = Math.max(...widths, 0);
    if (width <= 0 || String(style?.borderTopStyle || "none") === "none") return { stroke: "transparent", width: 0, dash: null };
    const borderStyle = String(style?.borderTopStyle || "solid");
    return {
      stroke: visibleColor(style?.borderTopColor, "#000000"),
      width,
      dash: borderStyle === "dashed" ? "dash" : borderStyle === "dotted" ? "dot" : null,
    };
  }

  function resolvedBorder(style, backgroundValue) {
    const outline = border(style);
    if (outline.width <= 0 || outline.stroke === "transparent") return { ...outline, stroke: "transparent" };
    return { ...outline, stroke: compositeColor(style?.borderTopColor, backgroundValue, outline.stroke) };
  }

  function background(style) {
    const image = String(style?.backgroundImage || "none");
    if (/^(?:linear|radial)-gradient\(/i.test(image)) return image;
    return visibleColor(style?.backgroundColor);
  }

  function splitGradientArguments(value) {
    const start = value.indexOf("(");
    const end = value.lastIndexOf(")");
    if (start < 0 || end <= start) return [];
    const output = [];
    let depth = 0, token = "";
    for (const character of value.slice(start + 1, end)) {
      if (character === "(") depth += 1;
      if (character === ")") depth -= 1;
      if (character === "," && depth === 0) { output.push(token.trim()); token = ""; }
      else token += character;
    }
    if (token.trim()) output.push(token.trim());
    return output;
  }

  function gradientStop(value) {
    const colorToken = value.match(/#[0-9a-f]{3,8}|rgba?\([^)]*\)|color\(srgb[^)]*\)/i)?.[0] || "";
    const color = visibleColor(colorToken);
    if (color === "transparent") return null;
    const position = value.match(/(-?\d+(?:\.\d+)?)%\s*$/)?.[1];
    return { color, position: position == null ? null : clamp(number(position) / 100, 0, 1), opacity: colorAlpha(colorToken, 1) };
  }

  function linearGradient(value) {
    if (!/^linear-gradient\(/i.test(String(value || ""))) return null;
    const args = splitGradientArguments(value);
    let angle = 180;
    if (/deg$/i.test(args[0] || "")) angle = number(args.shift(), 180);
    const stops = args.map(gradientStop).filter(Boolean);
    if (stops.length < 2) return null;
    const unresolved = stops.map((stop, index) => stop.position == null ? index : null).filter((index) => index != null);
    for (const index of unresolved) stops[index].position = index / Math.max(1, stops.length - 1);
    return { angle, stops };
  }

  function hasVisualBox(style) {
    return background(style) !== "transparent"
      || border(style).width > 0
      || (style?.boxShadow && style.boxShadow !== "none");
  }

  function cssFontFamilies(value) {
    const output = [];
    let token = "", quote = "";
    for (const character of String(value || "")) {
      if (quote) {
        if (character === quote) quote = "";
        else token += character;
      } else if (character === "\"" || character === "'") quote = character;
      else if (character === ",") {
        if (token.trim()) output.push(token.trim());
        token = "";
      } else token += character;
    }
    if (token.trim()) output.push(token.trim());
    return output;
  }

  function officeFontProfile(style, text = "") {
    const cssFamily = String(style?.fontFamily || "Aptos, Microsoft YaHei, sans-serif");
    const families = cssFontFamilies(cssFamily);
    const normalized = families.map((family) => family.toLowerCase());
    const generic = normalized.find((family) => ["serif", "monospace", "ui-serif", "ui-monospace", "fangsong"].includes(family));
    const firstPhysicalIndex = normalized.findIndex((family) => !CSS_VIRTUAL_FONTS.has(family));
    const firstPhysical = firstPhysicalIndex >= 0 ? normalized[firstPhysicalIndex] : "";
    let latin = firstPhysical
      ? EAST_ASIAN_FONT_NAMES.get(firstPhysical) || WEB_FONT_FALLBACKS.get(firstPhysical) || families[firstPhysicalIndex]
      : "";
    if (!latin) latin = generic === "serif" || generic === "ui-serif" ? "Times New Roman"
      : generic === "monospace" || generic === "ui-monospace" ? "Consolas" : "Segoe UI";
    const eastAsia = normalized.map((family) => EAST_ASIAN_FONT_NAMES.get(family)).find(Boolean) || "Microsoft YaHei";
    const content = String(text || "");
    const hasJapaneseText = /[\u3040-\u30ff]/u.test(content);
    const hasKoreanText = /[\uac00-\ud7af]/u.test(content);
    const hasEastAsianText = /[\u2e80-\u9fff\uf900-\ufaff\u3040-\u30ff\uac00-\ud7af]/u.test(content);
    return {
      cssFamily,
      nativeFontFamily: latin,
      nativeFonts: {
        latin,
        eastAsia: hasEastAsianText ? eastAsia : latin,
        complexScript: latin,
        symbol: /^Segoe UI$/i.test(latin) ? "Segoe UI Symbol" : latin,
        languageId: hasJapaneseText ? "ja-JP" : hasKoreanText ? "ko-KR" : hasEastAsianText ? "zh-CN" : "en-US",
      },
    };
  }

  function rectEdges(rect) {
    if (!rect) return null;
    const left = number(rect.left, number(rect.x));
    const top = number(rect.top, number(rect.y));
    const width = Math.max(0, number(rect.width, number(rect.right) - left));
    const height = Math.max(0, number(rect.height, number(rect.bottom) - top));
    return { left, top, right: number(rect.right, left + width), bottom: number(rect.bottom, top + height), width, height };
  }

  function officeTextRect(textRect, containerRect, style = {}, rootRect = null, multiline = false) {
    const text = rectEdges(textRect);
    if (!text) return textRect;
    const root = rectEdges(rootRect) || { left: -Infinity, top: -Infinity, right: Infinity, bottom: Infinity };
    const container = rectEdges(containerRect);
    const fontSize = Math.max(1, number(style.fontSize, 20));
    const horizontalSafety = Math.max(8, fontSize * .28);
    const verticalSafety = Math.max(2, fontSize * .08);
    const vertical = /vertical/.test(String(style.writingMode || ""));
    let left = text.left, right = text.right, top = text.top, bottom = text.bottom;
    if (vertical) {
      bottom = Math.min(root.bottom, Math.max(text.bottom + verticalSafety, container?.bottom || 0));
      right = Math.min(root.right, text.right + Math.max(2, fontSize * .08));
    } else {
      const paddingLeft = Math.max(0, number(style.paddingLeft));
      const paddingRight = Math.max(0, number(style.paddingRight));
      const contentLeft = container ? container.left + paddingLeft : text.left;
      const contentRight = container ? container.right - paddingRight : text.right;
      const align = textAlign(style.textAlign);
      if (container && contentRight > contentLeft && container.width > text.width + .5) {
        if (["right", "center", "justify"].includes(align)) left = contentLeft;
        right = Math.max(text.right + horizontalSafety, contentRight);
      } else right = text.right + horizontalSafety;
      left = Math.max(root.left, left);
      right = Math.min(root.right, Math.max(right, left + 1));
      bottom = Math.min(root.bottom, text.bottom + (multiline ? verticalSafety : Math.max(2, verticalSafety * .5)));
    }
    return { left, top, right, bottom, width: Math.max(1, right - left), height: Math.max(1, bottom - top) };
  }

  function textContainer(element, view) {
    let current = element;
    while (current?.getBoundingClientRect) {
      const style = view.getComputedStyle(current);
      const display = String(style?.display || "");
      if (display && display !== "inline" && display !== "contents") {
        const rect = current.getBoundingClientRect();
        if (rect.width > .5 && rect.height > .5) return { element: current, rect, style };
      }
      current = current.parentElement;
    }
    return null;
  }

  function normalizedCssText(fragment) {
    const mode = String(fragment?.style?.whiteSpace || "normal").toLowerCase();
    let text = String(fragment?.text || "").replace(/\r\n?/g, "\n");
    if (fragment?.hardBreak) return { text: "\n", mode: "hard-break", preserve: true };
    if (["pre", "pre-wrap", "break-spaces"].includes(mode)) return { text, mode, preserve: true };
    text = text.replace(/\u00a0/g, " ").replace(/[\t\f\v]/g, " ");
    if (mode === "pre-line") {
      text = text.split("\n").map((line) => line.replace(/ +/g, " ").trim()).join("\n");
      return { text, mode, preserve: false };
    }
    return { text: text.replace(/[ \n]+/g, " "), mode, preserve: false };
  }

  function coalesceTextFragments(fragments) {
    const output = [];
    for (const fragment of fragments || []) {
      const normalized = normalizedCssText(fragment);
      let text = normalized.text;
      if (!text) continue;
      const previous = output[output.length - 1];
      if (!normalized.preserve && previous && /[ \n]$/.test(previous.text)) text = text.replace(/^ +/, "");
      if (!normalized.preserve && previous?.text.endsWith("\n") && normalized.mode !== "pre-line") text = text.replace(/^\n+/, "");
      if (text) output.push({ ...fragment, text, whiteSpaceMode: normalized.mode, preserveWhiteSpace: normalized.preserve });
    }
    while (output.length && !output[0].preserveWhiteSpace && !output[0].text.replace(/^[ \n]+/, "")) output.shift();
    while (output.length && !output[output.length - 1].preserveWhiteSpace && !output[output.length - 1].text.replace(/[ \n]+$/, "")) output.pop();
    if (output.length) {
      if (!output[0].preserveWhiteSpace) {
        const leading = output[0].whiteSpaceMode === "pre-line" ? /^ +/ : /^[ \n]+/;
        output[0] = { ...output[0], text: output[0].text.replace(leading, "") };
      }
      const last = output.length - 1;
      if (!output[last].preserveWhiteSpace) {
        const trailing = output[last].whiteSpaceMode === "pre-line" ? / +$/ : /[ \n]+$/;
        output[last] = { ...output[last], text: output[last].text.replace(trailing, "") };
      }
    }
    return output.filter((fragment) => fragment.text);
  }

  function distinctTextLineCount(rects) {
    const lines = [];
    for (const rect of rects || []) {
      if (!rect || rect.width <= .5 || rect.height <= .5) continue;
      const center = number(rect.top) + number(rect.height) / 2;
      const tolerance = Math.max(1, number(rect.height) * .3);
      if (!lines.some((line) => Math.abs(line.center - center) <= Math.max(line.tolerance, tolerance))) {
        lines.push({ center, tolerance });
      }
    }
    return lines.length;
  }

  function unionTextRects(rects) {
    const visible = Array.from(rects || []).filter((rect) => rect && rect.width > .5 && rect.height > .5);
    if (!visible.length) return null;
    const left = Math.min(...visible.map((rect) => rect.left));
    const top = Math.min(...visible.map((rect) => rect.top));
    const right = Math.max(...visible.map((rect) => rect.right));
    const bottom = Math.max(...visible.map((rect) => rect.bottom));
    return { left, top, right, bottom, width: right - left, height: bottom - top };
  }

  function inlineTextFragments(container, view) {
    const fragments = [];
    const lineRects = [];
    const visit = (parent) => {
      for (const node of parent.childNodes || []) {
        if (node.nodeType === 3) {
          const range = parent.ownerDocument.createRange();
          range.selectNodeContents(node);
          const rects = Array.from(range.getClientRects?.() || []).filter((rect) => rect.width > .5 && rect.height > .5);
          const rect = rectEdges(range.getBoundingClientRect());
          const style = view.getComputedStyle(parent);
          fragments.push({ text: String(node.nodeValue || ""), style, rect });
          lineRects.push(...rects);
          range.detach?.();
          continue;
        }
        if (node.nodeType !== 1 || SKIP_TAGS.has(node.tagName)) continue;
        const style = view.getComputedStyle(node);
        if (style.display === "none" || style.visibility === "hidden" || cssOpacity(style) <= .005) continue;
        if (node.tagName === "BR") {
          fragments.push({ text: "\n", style, rect: null, hardBreak: true });
          continue;
        }
        const display = String(style.display || "");
        if (display && display !== "inline" && display !== "contents") continue;
        visit(node);
      }
    };
    visit(container);
    return { fragments: coalesceTextFragments(fragments), lineRects };
  }

  function textAlign(value) {
    const align = String(value || "left");
    return ["left", "right", "center", "justify"].includes(align) ? align : "left";
  }

  function textFromControl(element) {
    if (!TEXT_CONTROL_TAGS.has(element.tagName)) return "";
    return String(element.value || element.textContent || "").trim();
  }

  function directTextRuns(element, rootRect, view) {
    const runs = [];
    if (TEXT_CONTROL_TAGS.has(element.tagName)) {
      const text = textFromControl(element);
      const rect = element.getBoundingClientRect();
      if (text && intersecting(rect, rootRect)) runs.push({ text, rect, style: view.getComputedStyle(element), wordWrap: element.tagName === "TEXTAREA" });
      return runs;
    }
    const container = textContainer(element, view);
    if (!container || container.element !== element) return runs;
    const collected = inlineTextFragments(element, view);
    const text = collected.fragments.map((fragment) => fragment.text).join("");
    const rect = unionTextRects(collected.fragments.map((fragment) => fragment.rect));
    if (!text || !rect) return runs;
    const style = container.style;
    const multiline = text.includes("\n") || distinctTextLineCount(collected.lineRects) > 1;
    const largestFontSize = Math.max(number(style.fontSize, 20), ...collected.fragments.map((fragment) => number(fragment.style?.fontSize, 0)));
    const safeRect = officeTextRect(rect, container.rect, {
      fontSize: largestFontSize, textAlign: style.textAlign, writingMode: style.writingMode,
      paddingLeft: style.paddingLeft, paddingRight: style.paddingRight,
    }, rootRect, multiline);
    if (intersecting(safeRect, rootRect)) {
      runs.push({ text, rect: safeRect, style, richRuns: collected.fragments, wordWrap: multiline });
    }
    return runs;
  }

  function svgDataUrl(element) {
    try {
      const source = new XMLSerializer().serializeToString(element);
      const bytes = new TextEncoder().encode(source);
      let binary = "";
      bytes.forEach((byte) => { binary += String.fromCharCode(byte); });
      return `data:image/svg+xml;base64,${global.btoa(binary)}`;
    } catch (_) { return ""; }
  }

  function canvasDataUrl(element) {
    try { return element.toDataURL("image/png"); } catch (_) { return ""; }
  }

  function elementImage(element) {
    if (element.tagName === "IMG") return String(element.currentSrc || element.src || "");
    if (element.tagName === "SVG") return svgDataUrl(element);
    if (element.tagName === "CANVAS") return canvasDataUrl(element);
    if (element.tagName === "VIDEO") return String(element.poster || "");
    return "";
  }

  function rasterFallbackReason(element, style) {
    if (["IFRAME", "OBJECT", "EMBED"].includes(element.tagName)) return "isolated-document";
    if (element.tagName === "VIDEO" && !element.poster) return "video-without-poster";
    if (String(style?.filter || "none") !== "none") return "css-filter";
    if (String(style?.backdropFilter || "none") !== "none") return "backdrop-filter";
    if (![/^none$/i, /^normal$/i].some((pattern) => pattern.test(String(style?.mixBlendMode || "normal")))) return "mix-blend-mode";
    if (String(style?.clipPath || "none") !== "none") return "clip-path";
    if (String(style?.maskImage || "none") !== "none") return "css-mask";
    if (String(style?.perspective || "none") !== "none" || /matrix3d\(/i.test(String(style?.transform || ""))) return "3d-transform";
    return "";
  }

  function rasterCaptureRect(rect, style, rootRect) {
    const filter = String(style?.filter || "none");
    let left = 0, top = 0, right = 0, bottom = 0;
    const shadow = filter.match(/drop-shadow\((.+)\)/i);
    if (shadow) {
      const lengths = shadow[1].match(/-?\d+(?:\.\d+)?px/g)?.map(number) || [];
      const x = lengths[0] || 0, y = lengths[1] || 0, blur = Math.max(0, lengths[2] || 0) * 2;
      left = Math.max(left, blur + Math.max(0, -x));
      right = Math.max(right, blur + Math.max(0, x));
      top = Math.max(top, blur + Math.max(0, -y));
      bottom = Math.max(bottom, blur + Math.max(0, y));
    }
    const blur = filter.match(/(?:^|\s)blur\(\s*(\d+(?:\.\d+)?)px/i);
    if (blur) left = top = right = bottom = Math.max(left, top, right, bottom, number(blur[1]) * 2);
    const capture = {
      left: Math.max(rootRect.left, rect.left - left),
      top: Math.max(rootRect.top, rect.top - top),
      right: Math.min(rootRect.right, rect.right + right),
      bottom: Math.min(rootRect.bottom, rect.bottom + bottom),
    };
    capture.width = Math.max(1, capture.right - capture.left);
    capture.height = Math.max(1, capture.bottom - capture.top);
    return capture;
  }

  function canvasGradientStops(value) {
    const args = splitGradientArguments(value);
    if (args.length && !/(?:#[0-9a-f]{3,8}|rgba?\(|hsla?\(|color\()/i.test(args[0])) args.shift();
    const stops = args.map((part, index) => {
      const color = part.match(/#[0-9a-f]{3,8}|rgba?\([^)]*\)|hsla?\([^)]*\)|color\([^)]*\)|\b(?:transparent|white|black|red|green|blue)\b/i)?.[0];
      const position = part.match(/(-?\d+(?:\.\d+)?)%\s*$/)?.[1];
      return color ? { color, position: position == null ? null : clamp(number(position) / 100, 0, 1), index } : null;
    }).filter(Boolean);
    for (let index = 0; index < stops.length; index += 1) {
      if (stops[index].position == null) stops[index].position = index / Math.max(1, stops.length - 1);
    }
    return stops;
  }

  function canvasPaint(context, style, rect) {
    const value = String(style?.backgroundImage || "none");
    if (/^linear-gradient\(/i.test(value)) {
      const first = splitGradientArguments(value)[0] || "";
      const angle = /deg$/i.test(first) ? number(first, 180) : 180;
      const radians = (angle - 90) * Math.PI / 180;
      const distance = Math.abs(rect.width * Math.cos(radians)) + Math.abs(rect.height * Math.sin(radians));
      const centerX = rect.x + rect.width / 2, centerY = rect.y + rect.height / 2;
      const dx = Math.cos(radians) * distance / 2, dy = Math.sin(radians) * distance / 2;
      const gradient = context.createLinearGradient(centerX - dx, centerY - dy, centerX + dx, centerY + dy);
      for (const stop of canvasGradientStops(value)) gradient.addColorStop(stop.position, stop.color);
      return gradient;
    }
    if (/^radial-gradient\(/i.test(value)) {
      const at = value.match(/\bat\s+(-?\d+(?:\.\d+)?)%\s+(-?\d+(?:\.\d+)?)%/i);
      const centerX = rect.x + rect.width * (at ? number(at[1], 50) / 100 : .5);
      const centerY = rect.y + rect.height * (at ? number(at[2], 50) / 100 : .5);
      const radius = Math.max(
        Math.hypot(centerX - rect.x, centerY - rect.y),
        Math.hypot(rect.x + rect.width - centerX, centerY - rect.y),
        Math.hypot(centerX - rect.x, rect.y + rect.height - centerY),
        Math.hypot(rect.x + rect.width - centerX, rect.y + rect.height - centerY),
      );
      const gradient = context.createRadialGradient(centerX, centerY, 0, centerX, centerY, Math.max(1, radius));
      for (const stop of canvasGradientStops(value)) gradient.addColorStop(stop.position, stop.color);
      return gradient;
    }
    return String(style?.backgroundColor || "transparent");
  }

  function roundedRectPath(context, rect, radius) {
    const bounded = clamp(number(radius), 0, Math.min(rect.width, rect.height) / 2);
    context.beginPath();
    if (typeof context.roundRect === "function") context.roundRect(rect.x, rect.y, rect.width, rect.height, bounded);
    else context.rect(rect.x, rect.y, rect.width, rect.height);
  }

  function clipCoordinate(token, origin, extent) {
    const value = String(token || "").trim();
    return origin + (/%$/.test(value) ? number(value) / 100 * extent : number(value));
  }

  function applyCanvasClip(context, style, rect) {
    const value = String(style?.clipPath || "none");
    if (!value || value === "none") return;
    context.beginPath();
    const circle = value.match(/^circle\(\s*([^\s]+)?(?:\s+at\s+([^\s]+)\s+([^\s]+))?\s*\)$/i);
    if (circle) {
      const centerX = clipCoordinate(circle[2] || "50%", rect.x, rect.width);
      const centerY = clipCoordinate(circle[3] || "50%", rect.y, rect.height);
      const radius = /%/.test(circle[1] || "")
        ? number(circle[1], 50) / 100 * Math.min(rect.width, rect.height)
        : number(circle[1], Math.min(rect.width, rect.height) / 2);
      context.arc(centerX, centerY, Math.max(0, radius), 0, Math.PI * 2);
      context.clip();
      return;
    }
    const inset = value.match(/^inset\(([^)]+)\)$/i);
    if (inset) {
      const parts = inset[1].split(/\s+/).filter((part) => !/^round$/i.test(part));
      const values = [parts[0], parts[1] || parts[0], parts[2] || parts[0], parts[3] || parts[1] || parts[0]];
      const top = /%/.test(values[0]) ? number(values[0]) / 100 * rect.height : number(values[0]);
      const right = /%/.test(values[1]) ? number(values[1]) / 100 * rect.width : number(values[1]);
      const bottom = /%/.test(values[2]) ? number(values[2]) / 100 * rect.height : number(values[2]);
      const left = /%/.test(values[3]) ? number(values[3]) / 100 * rect.width : number(values[3]);
      context.rect(rect.x + left, rect.y + top, Math.max(0, rect.width - left - right), Math.max(0, rect.height - top - bottom));
      context.clip();
      return;
    }
    const polygon = value.match(/^polygon\(([^)]+)\)$/i);
    if (polygon) {
      const points = polygon[1].split(",").map((point) => point.trim().split(/\s+/));
      points.forEach((point, index) => {
        const x = clipCoordinate(point[0], rect.x, rect.width), y = clipCoordinate(point[1], rect.y, rect.height);
        if (index) context.lineTo(x, y); else context.moveTo(x, y);
      });
      context.closePath();
      context.clip();
    }
  }

  function drawCanvasBox(context, style, rect) {
    const fill = background(style);
    const outline = border(style);
    if (fill !== "transparent") {
      const shadow = boxShadow(style);
      if (shadow) {
        context.shadowColor = shadow.color;
        context.shadowBlur = shadow.blur;
        context.shadowOffsetX = shadow.offsetX;
        context.shadowOffsetY = shadow.offsetY;
      }
      const gradient = /^(?:linear|radial)-gradient\(/i.test(String(style?.backgroundImage || "none"));
      const baseColor = String(style?.backgroundColor || "transparent");
      if (gradient && colorAlpha(baseColor, 0) > 0) {
        roundedRectPath(context, rect, style.borderTopLeftRadius);
        context.fillStyle = baseColor;
        context.fill();
        context.shadowColor = "transparent";
        context.shadowBlur = 0;
        context.shadowOffsetX = 0;
        context.shadowOffsetY = 0;
      }
      roundedRectPath(context, rect, style.borderTopLeftRadius);
      context.fillStyle = canvasPaint(context, style, rect);
      context.fill();
      context.shadowColor = "transparent";
      context.shadowBlur = 0;
      context.shadowOffsetX = 0;
      context.shadowOffsetY = 0;
    }
    if (outline.width > 0) {
      roundedRectPath(context, rect, style.borderTopLeftRadius);
      context.strokeStyle = outline.stroke;
      context.lineWidth = outline.width;
      if (outline.dash === "dash") context.setLineDash([outline.width * 4, outline.width * 2]);
      if (outline.dash === "dot") context.setLineDash([outline.width, outline.width * 2]);
      context.stroke();
      context.setLineDash([]);
    }
  }

  function canvasFont(style) {
    return `${String(style.fontStyle || "normal")} ${Math.max(1, number(style.fontWeight, 400))} ${Math.max(1, number(style.fontSize, 20))}px ${fontFamily(style)}`;
  }

  function drawCanvasText(context, run, rootRect) {
    const rect = rectangle(run.rect, rootRect);
    const style = run.style;
    const text = String(run.text || "");
    if (!text) return;
    context.save();
    context.font = canvasFont(style);
    context.fillStyle = String(style.color || "#172033");
    context.textBaseline = "top";
    context.textAlign = textAlign(style.textAlign) === "center" ? "center" : textAlign(style.textAlign) === "right" ? "right" : "left";
    const anchor = context.textAlign === "center" ? rect.x + rect.width / 2 : context.textAlign === "right" ? rect.x + rect.width : rect.x;
    const lineHeight = number(style.lineHeight, number(style.fontSize, 20) * 1.2);
    const words = /\s/.test(text) ? text.split(/(\s+)/) : Array.from(text);
    const lines = [];
    let line = "";
    for (const word of words) {
      const next = line + word;
      if (line && context.measureText(next).width > rect.width) { lines.push(line); line = word.trimStart(); }
      else line = next;
    }
    if (line) lines.push(line);
    lines.slice(0, Math.max(1, Math.floor(rect.height / Math.max(1, lineHeight)))).forEach((value, index) => {
      context.fillText(value, anchor, rect.y + index * lineHeight, rect.width);
    });
    context.restore();
  }

  function loadCanvasImage(source) {
    return new Promise((resolve, reject) => {
      const image = new Image();
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error("局部图片无法解码"));
      image.src = source;
    });
  }

  async function canvasSafeImageSource(source, options) {
    const value = String(source || "");
    if (!value || /^data:image\//i.test(value)) return value;
    if (options?.restrictNetwork) return "";
    if (typeof global.fetch !== "function") return "";
    try {
      const response = await global.fetch(new URL(value, global.location.href), { credentials: "same-origin" });
      if (!response.ok) return "";
      const blob = await response.blob();
      const maximum = Math.max(1024, number(options.maximumEmbeddedImageBytes, 10 * 1024 * 1024));
      if (!/^image\//i.test(blob.type) || blob.size > maximum) return "";
      return blobDataUrl(blob);
    } catch (_) { return ""; }
  }

  async function paintCanvasElement(context, element, rootRect, view, options) {
    if (SKIP_TAGS.has(element.tagName)) return;
    const style = view.getComputedStyle(element);
    const clientRect = element.getBoundingClientRect();
    if (style.display === "none" || style.visibility === "hidden" || cssOpacity(style) <= .005 || clientRect.width <= .5 || clientRect.height <= .5) return;
    const rect = rectangle(clientRect, rootRect);
    context.save();
    context.globalAlpha *= cssOpacity(style);
    applyCanvasClip(context, style, rect);
    if (element !== rootRect.__root || hasVisualBox(style)) drawCanvasBox(context, style, rect);
    const asset = elementImage(element);
    if (asset) {
      try {
        const safeAsset = await canvasSafeImageSource(asset, options);
        const image = safeAsset ? await loadCanvasImage(safeAsset) : null;
        if (!image) throw new Error("局部图片不能安全内嵌");
        context.drawImage(image, rect.x, rect.y, rect.width, rect.height);
      } catch (_) {}
    }
    for (const run of directTextRuns(element, rootRect, view)) drawCanvasText(context, run, rootRect);
    for (const child of element.children || []) await paintCanvasElement(context, child, rootRect, view, options);
    context.restore();
  }

  function rasterCanvas(document, rect, options) {
    const width = Math.max(1, Math.ceil(rect.width));
    const height = Math.max(1, Math.ceil(rect.height));
    const maximumPixels = Math.max(250_000, number(options.maximumRasterPixels, 8_000_000));
    const scale = Math.max(1, Math.min(2, Math.sqrt(maximumPixels / Math.max(1, width * height))));
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(width * scale));
    canvas.height = Math.max(1, Math.round(height * scale));
    const context = canvas.getContext("2d");
    if (!context) throw new Error("浏览器没有可用的 2D Canvas");
    context.scale(scale, scale);
    return { canvas, context, width, height, scale };
  }

  async function rasterizeElement(element, rect, options = {}) {
    const document = element.ownerDocument;
    const view = document.defaultView;
    const rootRect = { left: rect.left ?? 0, top: rect.top ?? 0, right: rect.right ?? rect.width, bottom: rect.bottom ?? rect.height, width: rect.width, height: rect.height, __root: element };
    const { canvas, context, width, height, scale } = rasterCanvas(document, rect, options);
    const rootStyle = view.getComputedStyle(element);
    const rootFilter = String(rootStyle.filter || "none");
    if (rootFilter !== "none") {
      const layer = document.createElement("canvas");
      layer.width = canvas.width; layer.height = canvas.height;
      const layerContext = layer.getContext("2d");
      layerContext.scale(scale, scale);
      await paintCanvasElement(layerContext, element, rootRect, view, options);
      context.save();
      context.filter = rootFilter;
      context.drawImage(layer, 0, 0, width, height);
      context.restore();
    } else await paintCanvasElement(context, element, rootRect, view, options);
    if (!width || !height) throw new Error("局部区域尺寸无效");
    return canvas.toDataURL("image/png");
  }

  async function rasterizeBackground(document, style, rect, options = {}) {
    const { canvas, context } = rasterCanvas(document, rect, options);
    drawCanvasBox(context, style, { x: 0, y: 0, width: rect.width, height: rect.height });
    return canvas.toDataURL("image/png");
  }

  function localizedFallbackRoots(page, rootRect, view) {
    const roots = new Map();
    for (const element of page.querySelectorAll("*")) {
      if (SKIP_TAGS.has(element.tagName)) continue;
      const style = view.getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      if (style.display === "none" || style.visibility === "hidden" || cssOpacity(style) <= .005 || !intersecting(rect, rootRect)) continue;
      const reason = rasterFallbackReason(element, style);
      if (!reason) continue;
      if (Array.from(roots.keys()).some((ancestor) => ancestor.contains(element))) continue;
      for (const descendant of Array.from(roots.keys()).filter((candidate) => element.contains(candidate))) roots.delete(descendant);
      roots.set(element, reason);
    }
    return roots;
  }

  function pageCandidates(document) {
    for (const selector of PAGE_SELECTORS) {
      const matches = Array.from(document.querySelectorAll(selector));
      if (matches.length >= 2 || (matches.length === 1 && /omnidoc-page/.test(selector))) return matches;
    }
    const sections = Array.from(document.body?.children || []).filter((element) => element.tagName === "SECTION");
    return sections.length >= 2 ? sections : [document.body];
  }

  function pageSize(element, fallbackWidth, fallbackHeight) {
    const declared = String(element?.getAttribute?.("data-page-size") || "").match(/(\d+(?:\.\d+)?)\s*[x×]\s*(\d+(?:\.\d+)?)/i);
    const rect = element?.getBoundingClientRect?.() || {};
    return {
      width: declared ? number(declared[1], fallbackWidth) : number(rect.width, element?.scrollWidth || fallbackWidth),
      height: declared ? number(declared[2], fallbackHeight) : number(rect.height, element?.scrollHeight || fallbackHeight),
    };
  }

  function semanticName(element, fallback = "") {
    return String(
      element?.getAttribute?.("data-unippt-role")
      || element?.getAttribute?.("aria-label")
      || element?.className
      || fallback,
    ).trim();
  }

  function pageMetadata(page, pageIndex) {
    const notesElement = page?.querySelector?.("template[data-unippt-notes], [data-unippt-notes][hidden]");
    const notes = notesElement?.tagName === "TEMPLATE"
      ? String(notesElement.content?.textContent || "").trim()
      : String(notesElement?.textContent || "").trim();
    const titleElement = page?.querySelector?.('[data-unippt-role="cover-title"], [data-unippt-role="slide-title"], [data-unippt-role="closing-title"], h1');
    return {
      title: String(page?.getAttribute?.("data-title") || titleElement?.textContent || `HTML 页面 ${pageIndex + 1}`).trim(),
      role: String(page?.getAttribute?.("data-unippt-role") || "").trim(),
      notes,
    };
  }

  async function measurePage(page, pageIndex, options = {}) {
    const document = page.ownerDocument;
    const view = document.defaultView;
    const measured = pageSize(page, options.width || DEFAULT_WIDTH, options.height || DEFAULT_HEIGHT);
    let rootRect = page.getBoundingClientRect();
    if (rootRect.width < 1 || rootRect.height < 1) {
      rootRect = { left: 0, top: 0, right: measured.width, bottom: measured.height, width: measured.width, height: measured.height };
    }
    const pageStyle = view.getComputedStyle(page);
    const elements = [];
    const unsupported = [];
    const fallbacks = [];
    let order = 0;
    const pageFill = background(pageStyle);
    const bodyStyle = view.getComputedStyle(document.body);
    const pageBackground = visibleColor(pageStyle.backgroundColor, visibleColor(bodyStyle.backgroundColor, "#FFFFFF"));
    const pageRect = { x: 0, y: 0, width: rootRect.width, height: rootRect.height };
    if (/radial-gradient\(/i.test(pageFill)) {
      try {
        elements.push({
          type: "image", name: "page radial-gradient fidelity layer",
          asset: await rasterizeBackground(document, pageStyle, pageRect, options), fallback: true, pageBackground: true,
          rect: pageRect, zIndex: -100000, order: order++, opacity: 1,
        });
        fallbacks.push({ tag: "background", reason: "radial-gradient", mode: "background-raster", rect: pageRect });
      } catch (error) {
        elements.push({
          type: "box", name: "page-background", fill: pageFill, gradient: null,
          stroke: "transparent", strokeWidth: 0, strokeDash: null, radius: 0, shadow: null, rotation: 0,
          rect: pageRect, zIndex: -100000, order: order++, opacity: 1,
        });
        unsupported.push({ tag: "background", reason: "radial-gradient-raster-failed", message: error?.message || String(error) });
      }
    } else if (/linear-gradient\(/i.test(pageFill)) {
      elements.push({
        type: "box", name: "page-background", fill: pageFill, gradient: linearGradient(pageFill),
        stroke: "transparent", strokeWidth: 0, strokeDash: null, radius: 0, shadow: null, rotation: 0,
        rect: pageRect, zIndex: -100000, order: order++, opacity: 1,
      });
    }
    const fallbackRoots = localizedFallbackRoots(page, rootRect, view);
    for (const element of [page, ...page.querySelectorAll("*")]) {
      if (SKIP_TAGS.has(element.tagName)) continue;
      if (element !== page && Array.from(fallbackRoots.keys()).some((root) => root !== element && root.contains(element))) continue;
      const style = view.getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      if (style.display === "none" || style.visibility === "hidden" || cssOpacity(style) <= .005 || !intersecting(rect, rootRect)) continue;
      const zIndex = Number.isFinite(Number(style.zIndex)) ? Number(style.zIndex) : 0;
      const base = { rect: rectangle(rect, rootRect), zIndex, order: order++, opacity: cssOpacity(style) };
      const fallbackReason = fallbackRoots.get(element);
      if (fallbackReason) {
        try {
          if (["backdrop-filter", "mix-blend-mode", "css-mask", "3d-transform"].includes(fallbackReason)) {
            throw new Error(`${fallbackReason} 依赖页面上下文，不能安全隔离渲染`);
          }
          if (fallbackReason === "css-filter" && /url\(/i.test(String(style.filter || ""))) {
            throw new Error("引用式 SVG/CSS filter 不能安全隔离渲染");
          }
          let asset;
          let fallbackBase = base;
          if (element.tagName === "IFRAME") {
            const nested = element.contentDocument;
            if (!nested?.body) throw new Error("跨域或沙箱 iframe 不可读取");
            await settleDocument(nested, Math.min(2000, number(options.timeoutMs, 8000)));
            asset = await rasterizeElement(nested.body, { width: rect.width, height: rect.height }, options);
          } else if (["OBJECT", "EMBED"].includes(element.tagName)) {
            throw new Error("插件文档不可由浏览器安全读取");
          } else if (element.tagName === "VIDEO") {
            throw new Error("视频没有 poster 静态帧");
          } else {
            const captureRect = rasterCaptureRect(rect, style, rootRect);
            fallbackBase = { ...base, rect: rectangle(captureRect, rootRect) };
            asset = await rasterizeElement(element, captureRect, options);
          }
          elements.push({
            type: "image", asset, fallback: true,
            name: `${element.tagName.toLowerCase()} ${fallbackReason} fidelity layer`, ...fallbackBase,
          });
          fallbacks.push({ tag: element.tagName.toLowerCase(), reason: fallbackReason, mode: "localized-raster", rect: fallbackBase.rect });
          continue;
        } catch (error) {
          fallbackRoots.delete(element);
          unsupported.push({
            tag: element.tagName.toLowerCase(), reason: `${fallbackReason}-raster-failed`, rect: base.rect,
            message: error?.message || String(error),
          });
          if (["IFRAME", "OBJECT", "EMBED", "VIDEO"].includes(element.tagName)) continue;
        }
      }
      const asset = elementImage(element);
      if (asset) {
        elements.push({ type: "image", asset, name: semanticName(element, element.getAttribute("alt") || element.tagName.toLowerCase()), ...base });
      } else if (element !== page && hasVisualBox(style) && !["SVG", "CANVAS", "IMG", "VIDEO"].includes(element.tagName)) {
        const outline = resolvedBorder(style, pageBackground);
        const fill = background(style);
        if (/radial-gradient\(/i.test(fill)) {
          try {
            elements.push({
              type: "image", asset: await rasterizeBackground(document, style, base.rect, options), fallback: true,
              name: `${element.tagName.toLowerCase()} radial-gradient fidelity layer`, ...base,
            });
            fallbacks.push({ tag: element.tagName.toLowerCase(), reason: "radial-gradient", mode: "background-raster", rect: base.rect });
          } catch (error) {
            unsupported.push({ tag: element.tagName.toLowerCase(), reason: "radial-gradient-raster-failed", rect: base.rect, message: error?.message || String(error) });
          }
        } else {
          elements.push({
            type: "box", name: semanticName(element, element.tagName.toLowerCase()),
            fill, gradient: linearGradient(fill),
            stroke: outline.stroke,
            strokeWidth: outline.width, strokeDash: outline.dash,
            radius: number(style.borderTopLeftRadius), shadow: boxShadow(style), rotation: computedRotation(style.transform),
            ...base, opacity: visualBoxOpacity(style),
          });
        }
      }
      for (const run of directTextRuns(element, rootRect, view)) {
        const textStyle = run.style;
        const fonts = officeFontProfile(textStyle, run.text);
        const richRuns = (run.richRuns || [{ text: run.text, style: textStyle }]).map((fragment) => {
          const fragmentStyle = fragment.style || textStyle;
          const fragmentFonts = officeFontProfile(fragmentStyle, fragment.text);
          return {
            text: fragment.text,
            fontFamily: fragmentFonts.cssFamily, nativeFontFamily: fragmentFonts.nativeFontFamily,
            nativeFonts: fragmentFonts.nativeFonts, fontSize: number(fragmentStyle.fontSize, number(textStyle.fontSize, 20)),
            color: visibleColor(fragmentStyle.color, "#172033"), bold: number(fragmentStyle.fontWeight, 400) >= 600,
            italic: String(fragmentStyle.fontStyle) === "italic",
            underline: /underline/i.test(String(fragmentStyle.textDecorationLine || fragmentStyle.textDecoration || "")),
            strikethrough: /line-through/i.test(String(fragmentStyle.textDecorationLine || fragmentStyle.textDecoration || "")),
          };
        });
        elements.push({
          type: "text", text: run.text, name: semanticName(element, element.tagName.toLowerCase()),
          rect: rectangle(run.rect, rootRect), zIndex: zIndex + 1, order: order++, opacity: cssOpacity(textStyle) * colorAlpha(textStyle.color, 1),
          fontFamily: fonts.cssFamily, nativeFontFamily: fonts.nativeFontFamily, nativeFonts: fonts.nativeFonts,
          fontSize: number(textStyle.fontSize, 20),
          color: visibleColor(textStyle.color, "#172033"), bold: number(textStyle.fontWeight, 400) >= 600,
          italic: String(textStyle.fontStyle) === "italic", align: textAlign(textStyle.textAlign),
          lineHeight: number(textStyle.lineHeight, 0), letterSpacing: number(textStyle.letterSpacing, 0),
          writingMode: String(textStyle.writingMode || "horizontal-tb"), wordWrap: run.wordWrap !== false,
          rotation: computedRotation(textStyle.transform), runs: richRuns,
        });
      }
      const before = view.getComputedStyle(element, "::before");
      const after = view.getComputedStyle(element, "::after");
      for (const pseudo of [before, after]) {
        const content = String(pseudo?.content || "").replace(/^['"]|['"]$/g, "");
        if (content && content !== "none" && content !== "normal" && !/^url\(/.test(content)) {
          unsupported.push({ tag: "pseudo", reason: "generated-content", text: content, rect: base.rect });
        }
      }
    }
    const scripted = document.querySelectorAll("script").length;
    if (scripted) unsupported.push({ tag: "script", reason: "scripts-disabled", count: scripted });
    return {
      index: pageIndex,
      width: Math.max(1, measured.width || rootRect.width), height: Math.max(1, measured.height || rootRect.height),
      background: pageBackground,
      ...pageMetadata(page, pageIndex),
      elements, unsupported, fallbacks,
    };
  }

  function frame(rect, sourceWidth, sourceHeight, width, height, rotation = 0) {
    return {
      x: rect.x / sourceWidth * width, y: rect.y / sourceHeight * height,
      width: Math.max(1, rect.width / sourceWidth * width), height: Math.max(1, rect.height / sourceHeight * height),
      rotation: number(rotation),
    };
  }

  function compileMeasuredPages(pages, options = {}) {
    const width = Math.max(1, number(options.width, DEFAULT_WIDTH));
    const height = Math.max(1, number(options.height, DEFAULT_HEIGHT));
    const report = {
      pageCount: pages.length, nativeText: 0, nativeShapes: 0, preservedImages: 0, preservedBackgrounds: 0,
      localizedFallbacks: 0, externalImages: 0, unsupported: [], fallbacks: [], editableObjectCount: 0,
    };
    const slides = pages.map((page, pageIndex) => {
      const objects = [];
      const sourceWidth = Math.max(1, number(page.width, width));
      const sourceHeight = Math.max(1, number(page.height, height));
      const ordered = Array.from(page.elements || []).sort((first, second) => first.zIndex - second.zIndex || first.order - second.order || (first.type === "text" ? 1 : -1));
      const pageBackgroundLayer = ordered.find((item) => item.type === "image" && item.pageBackground);
      if (pageBackgroundLayer) report.preservedBackgrounds += 1;
      for (const item of ordered) {
        if (item === pageBackgroundLayer) continue;
        const objectFrame = frame(item.rect, sourceWidth, sourceHeight, width, height, item.rotation);
        if (item.type === "box") {
          objects.push({
            kind: "shape", name: `HTML 形状 · ${String(item.name || "box").slice(0, 48)}`,
            semanticRole: String(item.name || "").trim(),
            frame: objectFrame, geometry: item.radius > 1 ? "roundRect" : "rect",
            style: { fill: item.fill || "transparent", gradient: item.gradient || null, stroke: item.stroke || "transparent", strokeWidth: number(item.strokeWidth), strokeDash: item.strokeDash || null, opacity: item.opacity, shadow: item.shadow || null },
          });
          report.nativeShapes += 1;
        } else if (item.type === "image") {
          objects.push({
            kind: "image", name: `${item.fallback ? "HTML 保真片段" : "HTML 图像"} · ${String(item.name || "image").slice(0, 48)}`,
            semanticRole: String(item.name || "").trim(),
            frame: objectFrame, asset: item.asset,
            style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: item.opacity },
          });
          report.preservedImages += 1;
          if (!/^data:image\//i.test(String(item.asset || ""))) report.externalImages += 1;
        } else if (item.type === "text") {
          const scale = Math.min(width / sourceWidth, height / sourceHeight);
          const scaledFontSize = Math.max(1, item.fontSize * scale);
          const lineSpacing = item.lineHeight > 0
            ? clamp(item.lineHeight / Math.max(1, item.fontSize), .5, 5)
            : 1.2;
          const nativeFonts = item.nativeFonts || {};
          const richRuns = Array.isArray(item.runs) && item.runs.length ? item.runs : [{
            text: item.text, fontFamily: item.fontFamily, nativeFontFamily: item.nativeFontFamily || null,
            nativeFonts, fontSize: item.fontSize, color: item.color, bold: Boolean(item.bold),
            italic: Boolean(item.italic), underline: false, strikethrough: false,
          }];
          objects.push({
            kind: "text", name: `HTML 文本 · ${String(item.name || "text").slice(0, 48)}`, text: item.text,
            semanticRole: String(item.name || "").trim(),
            frame: objectFrame,
            textStyle: {
              fontFamily: item.fontFamily, nativeFontFamily: item.nativeFontFamily || null,
              nativeFonts, fontSize: scaledFontSize, color: item.color,
              bold: Boolean(item.bold), italic: Boolean(item.italic), align: item.align || "left",
            },
            textParagraphs: [{
              align: item.align || "left", level: 0, bullet: null, numbering: null,
              lineSpacing, spaceBefore: 0, spaceAfter: 0,
              runs: richRuns.map((run) => ({
                text: run.text, fontFamily: run.fontFamily || item.fontFamily,
                nativeFontFamily: run.nativeFontFamily || item.nativeFontFamily || null,
                nativeFonts: run.nativeFonts || nativeFonts,
                fontSize: Math.max(1, number(run.fontSize, item.fontSize) * scale), color: run.color || item.color,
                bold: Boolean(run.bold), italic: Boolean(run.italic), underline: Boolean(run.underline),
                strikethrough: Boolean(run.strikethrough), baseline: "normal",
                hyperlinks: { click: null, hover: null },
              })),
            }],
            textFrame: {
              marginLeft: 0, marginRight: 0, marginTop: 0, marginBottom: 0, verticalAlign: "top",
              verticalType: /vertical/.test(item.writingMode || "") ? "vert" : "horz",
              wordWrap: item.wordWrap !== false, autoSize: "textToFitShape",
            },
            style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: item.opacity },
          });
          report.nativeText += 1;
        }
      }
      const pageUnsupported = (page.unsupported || []).map((issue) => ({ pageIndex, ...issue }));
      const pageFallbacks = (page.fallbacks || []).map((issue) => ({ pageIndex, ...issue }));
      report.unsupported.push(...pageUnsupported);
      report.fallbacks.push(...pageFallbacks);
      report.localizedFallbacks += pageFallbacks.length;
      const authoredNotes = String(page.notes || "").trim();
      const importNotes = `[HTML Import]\nsource=${String(options.sourceName || "自由 HTML")}\nnativeObjects=${objects.length}\nlocalizedFallbacks=${pageFallbacks.length}\nunsupported=${pageUnsupported.length}\n[/HTML Import]`;
      return {
        name: String(options.slideNames?.[pageIndex] || page.title || `HTML 页面 ${pageIndex + 1}`),
        background: visibleColor(page.background, "#ffffff"),
        backgroundAsset: pageBackgroundLayer?.asset || null,
        notes: [authoredNotes, importNotes].filter(Boolean).join("\n\n"),
        objects,
        transition: null,
        semantic: {
          role: String(page.role || (pageIndex === 0 ? "cover" : pageIndex === pages.length - 1 ? "closing" : "content")),
          summary: String(page.title || `HTML 页面 ${pageIndex + 1}`),
        },
      };
    });
    report.editableObjectCount = report.nativeText + report.nativeShapes;
    report.fidelityRisk = report.unsupported.length
      ? (report.unsupported.length > pages.length ? "high" : "medium")
      : report.externalImages || report.localizedFallbacks ? "medium" : "low";
    report.editabilityRatio = report.nativeText + report.nativeShapes + report.preservedImages
      ? Math.round((report.nativeText + report.nativeShapes) / (report.nativeText + report.nativeShapes + report.preservedImages) * 1000) / 10
      : 0;
    return { title: String(options.title || options.sourceName || "自由 HTML"), width, height, slides, report };
  }

  function waitForFrame(frame, timeoutMs) {
    return new Promise((resolve, reject) => {
      const timer = global.setTimeout(() => reject(new Error("自由 HTML 沙箱加载超时")), timeoutMs);
      frame.addEventListener("load", () => { global.clearTimeout(timer); resolve(); }, { once: true });
    });
  }

  async function settleDocument(document, timeoutMs = 3500) {
    try { await Promise.race([document.fonts?.ready || Promise.resolve(), new Promise((resolve) => global.setTimeout(resolve, timeoutMs))]); } catch (_) {}
    const images = Array.from(document.images || []).filter((image) => !image.complete);
    await Promise.race([
      Promise.all(images.map((image) => new Promise((resolve) => {
        image.addEventListener("load", resolve, { once: true });
        image.addEventListener("error", resolve, { once: true });
      }))),
      new Promise((resolve) => global.setTimeout(resolve, timeoutMs)),
    ]);
    await new Promise((resolve) => global.requestAnimationFrame(() => global.requestAnimationFrame(resolve)));
  }

  async function blobDataUrl(blob) {
    const bytes = new Uint8Array(await blob.arrayBuffer());
    let binary = "";
    for (let offset = 0; offset < bytes.length; offset += 0x8000) {
      binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
    }
    return `data:${blob.type || "application/octet-stream"};base64,${global.btoa(binary)}`;
  }

  async function inlineExternalImages(pages, options = {}) {
    if (typeof global.fetch !== "function") return;
    const maximum = Math.max(1024, number(options.maximumEmbeddedImageBytes, 10 * 1024 * 1024));
    for (const page of pages) {
      for (const item of page.elements || []) {
        if (item.type !== "image" || /^data:image\//i.test(String(item.asset || ""))) continue;
        if (options.restrictNetwork) {
          page.unsupported.push({ tag: "img", reason: "network-disabled", source: String(item.asset || "").slice(0, 240) });
          continue;
        }
        try {
          const response = await global.fetch(new URL(item.asset, global.location.href), { credentials: "same-origin" });
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          const blob = await response.blob();
          if (!/^image\//i.test(blob.type) || blob.size > maximum) throw new Error("图片类型或大小不适合嵌入");
          item.asset = await blobDataUrl(blob);
        } catch (error) {
          page.unsupported.push({ tag: "img", reason: "external-image-not-embedded", source: String(item.asset || "").slice(0, 240), message: error?.message || String(error) });
        }
      }
    }
  }

  function injectFreezeCss(html, options = {}) {
    const css = "<style data-unippt-html-freeze>*{animation:none!important;transition:none!important;caret-color:transparent!important}html,body{margin:0!important}</style>";
    const csp = options.restrictNetwork
      ? `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data: blob:; style-src 'unsafe-inline'; font-src data:; media-src 'none'; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'">`
      : "";
    const guard = `${csp}${css}`;
    if (/<head\b[^>]*>/i.test(html)) return html.replace(/<head\b[^>]*>/i, (head) => `${head}${guard}`);
    if (/<html\b[^>]*>/i.test(html)) return html.replace(/<html\b[^>]*>/i, (root) => `${root}<head>${guard}</head>`);
    return `<head>${guard}</head>${html}`;
  }

  async function convert(html, options = {}) {
    if (!global.document?.createElement) throw new Error("自由 HTML 转 PPT 需要浏览器布局引擎");
    const source = String(html || "").trim();
    if (!source) throw new Error("自由 HTML 源码为空");
    const frame = global.document.createElement("iframe");
    frame.setAttribute("sandbox", "allow-same-origin");
    frame.setAttribute("aria-hidden", "true");
    frame.style.cssText = `position:fixed;left:-20000px;top:0;width:${Math.max(320, number(options.viewportWidth, DEFAULT_WIDTH))}px;height:${Math.max(180, number(options.viewportHeight, DEFAULT_HEIGHT))}px;border:0;visibility:hidden;pointer-events:none`;
    global.document.body.append(frame);
    try {
      const loaded = waitForFrame(frame, number(options.timeoutMs, 8000));
      frame.srcdoc = injectFreezeCss(source, options);
      await loaded;
      const document = frame.contentDocument;
      if (!document?.body) throw new Error("无法读取自由 HTML 沙箱");
      await settleDocument(document);
      const candidates = pageCandidates(document).filter(Boolean);
      const snapshots = [];
      const originals = candidates.map((page) => page.getAttribute("style"));
      for (let index = 0; index < candidates.length; index += 1) {
        if (candidates.length > 1) {
          candidates.forEach((candidate, candidateIndex) => {
            if (candidateIndex !== index) candidate.style.setProperty("display", "none", "important");
          });
          const current = candidates[index];
          current.style.setProperty("display", String(current.dataset.display || "block"), "important");
          current.style.setProperty("visibility", "visible", "important");
          current.style.setProperty("opacity", "1", "important");
          current.style.setProperty("transform", "none", "important");
          await new Promise((resolve) => global.requestAnimationFrame(resolve));
        }
        snapshots.push(await measurePage(candidates[index], index, options));
        candidates.forEach((candidate, candidateIndex) => {
          const original = originals[candidateIndex];
          if (original == null) candidate.removeAttribute("style"); else candidate.setAttribute("style", original);
        });
      }
      await inlineExternalImages(snapshots, options);
      const title = String(options.title || document.title || options.sourceName || "自由 HTML");
      return compileMeasuredPages(snapshots, { ...options, title });
    } finally {
      frame.remove();
    }
  }

  global.UniPptHtmlToPpt = Object.freeze({
    convert, compileMeasuredPages, pageCandidates,
    __test: Object.freeze({ colorAlpha, visibleColor, compositeColor, resolvedBorder, cssFontFamilies, officeFontProfile, officeTextRect, coalesceTextFragments, distinctTextLineCount, directTextRuns }),
  });
})(globalThis);
