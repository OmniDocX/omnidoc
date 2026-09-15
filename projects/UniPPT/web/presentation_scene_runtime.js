(function installUniPptPresentationScene(global) {
  "use strict";

  const STYLE_ID = "unippt-presentation-scene-runtime-style";
  const autoTextFitCache = new Map();

  function number(value, fallback = 0) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
  }

  function clamp(value, minimum, maximum) {
    return Math.max(minimum, Math.min(maximum, value));
  }

  function element(documentValue, tag, className, text) {
    const node = documentValue.createElement(tag);
    if (className) node.className = className;
    if (text != null) node.textContent = text;
    return node;
  }

  function installStyles(documentValue) {
    if (!documentValue?.head || documentValue.getElementById(STYLE_ID)) return;
    const style = documentValue.createElement("style");
    style.id = STYLE_ID;
    style.textContent = `
.unippt-shared-scene .scene-object{position:absolute;display:flex;box-sizing:border-box;white-space:pre-wrap;overflow:hidden;transform-origin:center;line-height:1.18;border:var(--object-border,0px) var(--object-border-style,solid) var(--object-stroke,transparent);background:var(--object-fill,transparent);color:var(--text-color,#172033);font-family:var(--font-family,"Aptos",sans-serif);font-size:var(--font-size,24px);font-weight:var(--font-weight,400);font-style:var(--font-style,normal);text-align:var(--text-align,left);opacity:var(--object-opacity,1)}
.unippt-shared-scene .scene-object.kind-group{display:block;padding:0!important;overflow:visible;border:0!important;background:transparent!important;pointer-events:none}
.unippt-shared-scene .scene-object.kind-group>.scene-object{pointer-events:auto}
.unippt-shared-scene .scene-object.kind-image{padding:0!important;background:transparent!important}
.unippt-shared-scene .scene-object.kind-image>img{position:absolute;width:100%;height:100%;max-width:none;max-height:none;object-fit:fill;pointer-events:none}
.unippt-shared-scene .scene-object>.shape-fill-image{position:absolute;width:auto;height:auto;max-width:none;max-height:none;object-fit:fill;pointer-events:none;z-index:-1}
.unippt-shared-scene .scene-object.has-linear-geometry,.unippt-shared-scene .scene-object.has-custom-geometry{padding:0!important;overflow:visible!important;background:transparent!important;border:0!important}
.unippt-shared-scene .linear-geometry-svg,.unippt-shared-scene .custom-geometry-svg{position:absolute;inset:0;width:100%;height:100%;overflow:visible;pointer-events:none}
.unippt-shared-scene .scene-object.has-custom-geometry>.object-content,.unippt-shared-scene .scene-object.has-custom-geometry>.rich-text{position:relative;z-index:1}
.unippt-shared-scene .scene-object.kind-table,.unippt-shared-scene .scene-object.kind-chart{padding:0!important;align-items:stretch!important;background:transparent}
.unippt-shared-scene .scene-object.kind-chart>.native-chart-svg{width:100%;height:100%;pointer-events:none}
.unippt-shared-scene .ppt-table-grid{width:100%;height:100%;display:grid;min-width:0;min-height:0;overflow:hidden}
.unippt-shared-scene .ppt-table-cell{box-sizing:border-box;min-width:0;min-height:0;overflow:hidden;display:flex;line-height:1.15;white-space:normal}
.unippt-shared-scene .object-content{width:100%;min-width:0}
.unippt-shared-scene [data-text-flow="vertical-rl-upright"]>.object-content{writing-mode:vertical-rl;text-orientation:upright;width:100%;height:100%}
.unippt-shared-scene [data-text-flow="vertical-rl-mixed"]>.object-content{writing-mode:vertical-rl;text-orientation:mixed;width:100%;height:100%}
.unippt-shared-scene [data-text-flow="vertical-lr-mixed"]>.object-content{writing-mode:vertical-lr;text-orientation:mixed;width:100%;height:100%}
.unippt-shared-scene [data-text-flow="vertical-lr-upright"]>.object-content{writing-mode:vertical-lr;text-orientation:upright;width:100%;height:100%}
.unippt-shared-scene .ppt-paragraph{position:relative;min-height:1em;margin:0;white-space:pre-wrap}
.unippt-shared-scene .ppt-bullet{display:inline-block;min-width:1.15em;margin-left:-1.15em}
.unippt-shared-scene .ppt-run{white-space:inherit}
.unippt-shared-scene .scene-object.kind-math{justify-content:center;overflow:visible}
.unippt-shared-scene .scene-object.kind-math .katex-display{margin:0}
.unippt-shared-scene .scene-object>video.native-media,.unippt-shared-scene .scene-object>video.media{position:absolute;inset:0;z-index:1;width:100%;height:100%;object-fit:cover;pointer-events:none}
.unippt-shared-scene .scene-object>audio.native-media,.unippt-shared-scene .scene-object>audio.media{display:none}
.unippt-shared-scene .dynamic-content-object{padding:0!important;background:#fff!important}
.unippt-shared-scene .dynamic-content-frame{display:block;width:100%;height:100%;border:0;background:#fff}
`;
    documentValue.head.append(style);
  }

  function orderedLayerObjects(slide) {
    const output = [];
    let z = 0;
    for (const [layer, objects] of [
      ["master", slide?.masterObjects],
      ["layout", slide?.layoutObjects],
      ["slide", slide?.objects],
    ]) {
      for (const object of objects || []) output.push({ layer, object, z: z++ });
    }
    return output;
  }

  function applySlideBackground(node, slide) {
    node.style.background = slide?.background || "#ffffff";
    node.style.backgroundImage = slide?.backgroundAsset
      ? `url(${JSON.stringify(slide.backgroundAsset)})`
      : "none";
    node.style.backgroundSize = "100% 100%";
    node.style.backgroundPosition = "center";
    node.style.backgroundRepeat = "no-repeat";
  }

  function isDistributedAlign(value) {
    return value === "distributed" || value === "thaiDistributed";
  }

  function cssTextAlign(value) {
    return isDistributedAlign(value) || value === "justifyLow" ? "justify" : value || "left";
  }

  function textFlowMode(verticalType) {
    switch (String(verticalType || "horz")) {
      case "eaVert": return "vertical-rl-upright";
      case "vert":
      case "wordArtVert": return "vertical-rl-mixed";
      case "vert270":
      case "wordArtVertRtl": return "vertical-lr-mixed";
      case "mongolianVert": return "vertical-lr-upright";
      default: return "horizontal";
    }
  }

  function dashCss(value) {
    if (/dot/i.test(value || "")) return "dotted";
    if (/dash/i.test(value || "")) return "dashed";
    return "solid";
  }

  function cssColorWithOpacity(color, opacity) {
    const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color || "");
    if (!match) return color;
    const rgb = match.slice(1).map((value) => parseInt(value, 16));
    return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${clamp(number(opacity, 1), 0, 1)})`;
  }

  function shadowCss(shadow) {
    if (!shadow) return "none";
    const color = cssColorWithOpacity(shadow.color || "#000000", shadow.opacity ?? 0.35);
    return `${shadow.inset ? "inset " : ""}${number(shadow.offsetX)}px ${number(shadow.offsetY)}px ${Math.max(0, number(shadow.blur))}px ${color}`;
  }

  function objectTransform(object) {
    const frame = object?.frame || {};
    return `rotate(${number(frame.rotation)}deg) scaleX(${object?.flipH ? -1 : 1}) scaleY(${object?.flipV ? -1 : 1})`;
  }

  function textGradientCss(gradient) {
    if (!gradient?.stops?.length) return null;
    const stops = gradient.stops.map((stop) =>
      `${cssColorWithOpacity(stop.color || "#000000", stop.opacity ?? 1)} ${clamp(number(stop.position), 0, 1) * 100}%`
    );
    return `linear-gradient(${number(gradient.angle)}deg, ${stops.join(", ")})`;
  }

  function applyTextGradient(node, gradient) {
    const fill = textGradientCss(gradient);
    if (!fill) return;
    node.style.backgroundImage = fill;
    node.style.backgroundClip = "text";
    node.style.webkitBackgroundClip = "text";
    node.style.color = "transparent";
    node.style.webkitTextFillColor = "transparent";
  }

  function applyObjectStyle(node, object, z) {
    const frame = object?.frame || {};
    const style = object?.style || {};
    const textStyle = object?.textStyle || {};
    const textFrame = object?.textFrame || {};
    const transform = objectTransform(object);
    const textAlign = textStyle.align || "left";
    const radius = /round|ellipse|arc|cloud/i.test(object?.geometry || "")
      ? (/ellipse/i.test(object?.geometry || "") ? "50%" : "16px")
      : "0";
    const tiledImageFill = Boolean(object?.shapeFillAsset && /\brepeat\b/i.test(style.fill || ""));

    Object.assign(node.style, {
      zIndex: String(z + 1),
      left: `${number(frame.x)}px`,
      top: `${number(frame.y)}px`,
      width: `${Math.max(1, number(frame.width, 1))}px`,
      height: `${Math.max(1, number(frame.height, 1))}px`,
      boxSizing: "border-box",
      minWidth: "0",
      minHeight: "0",
      transform,
      justifyContent: textAlign === "center" ? "center" : textAlign === "right" ? "flex-end" : "flex-start",
      alignItems: textFrame.verticalAlign === "top" ? "flex-start" : textFrame.verticalAlign === "bottom" ? "flex-end" : "center",
      padding: `${textFrame.marginTop ?? 5}px ${textFrame.marginRight ?? 8}px ${textFrame.marginBottom ?? 5}px ${textFrame.marginLeft ?? 8}px`,
      whiteSpace: textFrame.wordWrap === false ? "pre" : "pre-wrap",
      boxShadow: shadowCss(style.shadow),
      background: object?.shapeFillAsset && !tiledImageFill ? "transparent" : style.fill || "transparent",
      border: `${number(style.strokeWidth)}px ${dashCss(style.strokeDash)} ${style.stroke || "transparent"}`,
      opacity: String(style.opacity ?? 1),
      color: textStyle.color || "#172033",
      fontFamily: textStyle.fontFamily || textStyle.nativeFontFamily || "Aptos, sans-serif",
      fontSize: `${number(textStyle.fontSize, 24)}px`,
      fontWeight: textStyle.bold ? "700" : "400",
      fontStyle: textStyle.italic ? "italic" : "normal",
      textAlign: cssTextAlign(textAlign),
      borderRadius: radius,
    });
    if (tiledImageFill && !object.customGeometry) {
      node.style.backgroundImage = `url(${JSON.stringify(object.shapeFillAsset)})`;
      node.style.backgroundRepeat = "repeat";
    }
    node.style.textAlignLast = isDistributedAlign(textAlign) ? "justify" : "auto";
    node.dataset.id = object?.id || "";
    node.dataset.base = transform;
    node.dataset.animationBase = transform;
    node.dataset.textAutoSize = textFrame.autoSize || "none";
    node.dataset.textFlow = textFlowMode(textFrame.verticalType);
    node.dataset.textWrap = textFrame.wordWrap === false ? "none" : "wrap";
    node.style.setProperty("--object-fill", node.style.background || "transparent");
    node.style.setProperty("--object-stroke", style.stroke || "transparent");
    node.style.setProperty("--object-border", `${number(style.strokeWidth)}px`);
    node.style.setProperty("--object-border-style", dashCss(style.strokeDash));
    node.style.setProperty("--object-opacity", String(style.opacity ?? 1));
    node.style.setProperty("--text-color", textStyle.color || "#172033");
    node.style.setProperty("--font-family", textStyle.fontFamily || textStyle.nativeFontFamily || "Aptos, sans-serif");
    node.style.setProperty("--font-size", `${number(textStyle.fontSize, 24)}px`);
    node.style.setProperty("--font-weight", textStyle.bold ? "700" : "400");
    node.style.setProperty("--font-style", textStyle.italic ? "italic" : "normal");
    node.style.setProperty("--text-align", cssTextAlign(textAlign));
    node.style.setProperty("--shape-radius", radius);
  }

  function imageCropMetrics(crop, width = 100, height = 100) {
    const left = clamp(number(crop?.left), 0, 0.9999);
    const top = clamp(number(crop?.top), 0, 0.9999);
    const right = clamp(number(crop?.right), 0, 0.9999);
    const bottom = clamp(number(crop?.bottom), 0, 0.9999);
    const visibleWidth = Math.max(0.0001, 1 - Math.min(0.9999, left + right));
    const visibleHeight = Math.max(0.0001, 1 - Math.min(0.9999, top + bottom));
    return {
      x: -left / visibleWidth * width,
      y: -top / visibleHeight * height,
      width: width / visibleWidth,
      height: height / visibleHeight,
    };
  }

  function applyImageCrop(image, crop) {
    const metrics = imageCropMetrics(crop);
    Object.assign(image.style, {
      left: `${metrics.x}%`, top: `${metrics.y}%`,
      width: `${metrics.width}%`, height: `${metrics.height}%`,
    });
  }

  function imageFillMetrics(crop, fillRect, width = 100, height = 100) {
    const left = number(fillRect?.left);
    const top = number(fillRect?.top);
    const right = number(fillRect?.right);
    const bottom = number(fillRect?.bottom);
    const destination = {
      x: left * width,
      y: top * height,
      width: Math.max(0.0001, (1 - left - right) * width),
      height: Math.max(0.0001, (1 - top - bottom) * height),
    };
    const source = imageCropMetrics(crop, destination.width, destination.height);
    return {
      x: destination.x + source.x,
      y: destination.y + source.y,
      width: source.width,
      height: source.height,
    };
  }

  function applyImageEffects(host, image, effects, documentValue) {
    const radius = Math.max(0, number(effects?.softEdgeRadius));
    if (radius > 0) {
      const width = Math.max(1, number(host.style.width?.replace("px", ""), host.offsetWidth || 1));
      const height = Math.max(1, number(host.style.height?.replace("px", ""), host.offsetHeight || 1));
      const x = Math.min(50, radius / width * 100);
      const y = Math.min(50, radius / height * 100);
      const mask = `linear-gradient(to right, transparent 0%, #000 ${x}%, #000 ${100 - x}%, transparent 100%), linear-gradient(to bottom, transparent 0%, #000 ${y}%, #000 ${100 - y}%, transparent 100%)`;
      Object.assign(image.style, {
        maskImage: mask, maskComposite: "intersect",
        webkitMaskImage: mask, webkitMaskComposite: "source-in",
      });
    }
    const color = (value) => /^#[0-9a-f]{6}$/i.test(value || "")
      ? [1, 3, 5].map((offset) => parseInt(value.slice(offset, offset + 2), 16) / 255)
      : null;
    const shadow = color(effects?.duotone?.shadowColor);
    const highlight = color(effects?.duotone?.highlightColor);
    if (!shadow || !highlight) return;
    const namespace = "http://www.w3.org/2000/svg";
    const svg = documentValue.createElementNS(namespace, "svg");
    const filter = documentValue.createElementNS(namespace, "filter");
    const grayscale = documentValue.createElementNS(namespace, "feColorMatrix");
    const transfer = documentValue.createElementNS(namespace, "feComponentTransfer");
    const id = `unippt-duotone-${String(host.dataset.id || "image").replace(/[^a-z0-9_-]/gi, "-")}-${Math.random().toString(36).slice(2)}`;
    filter.id = id;
    filter.setAttribute("color-interpolation-filters", "sRGB");
    grayscale.setAttribute("type", "saturate");
    grayscale.setAttribute("values", "0");
    for (let index = 0; index < 3; index += 1) {
      const component = documentValue.createElementNS(namespace, `feFunc${"RGB"[index]}`);
      component.setAttribute("type", "table");
      component.setAttribute("tableValues", `${shadow[index]} ${highlight[index]}`);
      transfer.append(component);
    }
    filter.append(grayscale, transfer);
    const defs = documentValue.createElementNS(namespace, "defs");
    defs.append(filter);
    svg.append(defs);
    Object.assign(svg.style, { position: "absolute", width: "0", height: "0", overflow: "hidden" });
    svg.setAttribute("aria-hidden", "true");
    host.append(svg);
    image.style.filter = `url(#${id})`;
  }

  function renderShapeFillImage(node, object, documentValue) {
    if (!object?.shapeFillAsset || object.customGeometry || object.kind === "image" || /\brepeat\b/i.test(object.style?.fill || "")) return false;
    const frame = object.frame || {};
    const metrics = imageFillMetrics(
      object.imageCrop,
      object.imageFillRect,
      Math.max(1, number(frame.width, 1)),
      Math.max(1, number(frame.height, 1)),
    );
    const image = element(documentValue, "img", "shape-fill-image");
    image.src = object.shapeFillAsset;
    image.alt = "";
    Object.assign(image.style, {
      left: `${metrics.x}px`, top: `${metrics.y}px`,
      width: `${metrics.width}px`, height: `${metrics.height}px`,
    });
    node.prepend(image);
    return true;
  }

  function isLinearGeometry(object) {
    const frame = object?.frame || {};
    const style = object?.style || {};
    const width = Math.abs(number(frame.width));
    const height = Math.abs(number(frame.height));
    const strokeWidth = number(style.strokeWidth);
    const stroke = String(style.stroke || "").trim().toLowerCase();
    const fill = String(style.fill || "").trim().toLowerCase();
    const hasVisibleStroke = strokeWidth > 0 && stroke && stroke !== "transparent" && stroke !== "none";
    const hasNoFill = !fill || fill === "transparent" || fill === "none";
    const hasText = Boolean(String(object?.text || "").trim()) || (object?.textParagraphs || []).some(
      (paragraph) => (paragraph.runs || []).some((run) => Boolean(String(run.text || "").trim())),
    );
    const degenerateFrame = width <= 0.5 || height <= 0.5;
    const linePreset = /(?:^|[^a-z])(?:line|straightconnector)/i.test(String(object?.geometry || ""));
    return Boolean(hasVisibleStroke && !hasText && hasNoFill && (degenerateFrame || linePreset));
  }

  function renderLinearGeometry(node, object, documentValue) {
    if (!isLinearGeometry(object)) return false;
    const namespace = "http://www.w3.org/2000/svg";
    const svg = documentValue.createElementNS(namespace, "svg");
    const line = documentValue.createElementNS(namespace, "line");
    const frame = object.frame || {};
    const horizontal = Math.abs(number(frame.height)) <= 0.5;
    const vertical = Math.abs(number(frame.width)) <= 0.5;
    svg.classList.add("linear-geometry-svg");
    svg.setAttribute("viewBox", "0 0 1000 1000");
    svg.setAttribute("preserveAspectRatio", "none");
    svg.setAttribute("aria-hidden", "true");
    line.setAttribute("x1", vertical ? "500" : "0");
    line.setAttribute("y1", horizontal ? "500" : "0");
    line.setAttribute("x2", vertical ? "500" : "1000");
    line.setAttribute("y2", horizontal ? "500" : "1000");
    line.setAttribute("stroke", object.style?.stroke || "transparent");
    line.setAttribute("stroke-width", String(Math.max(0.25, number(object.style?.strokeWidth, 1))));
    line.setAttribute("stroke-linecap", "butt");
    line.setAttribute("vector-effect", "non-scaling-stroke");
    line.setAttribute("stroke-dasharray", /dot/i.test(object.style?.strokeDash || "") ? "1 2" : /dash/i.test(object.style?.strokeDash || "") ? "6 4" : "none");
    svg.append(line);
    node.classList.add("has-linear-geometry");
    Object.assign(node.style, { background: "transparent", border: "0", padding: "0", overflow: "visible" });
    node.append(svg);
    return true;
  }

  function renderCustomGeometry(node, object, documentValue) {
    const geometry = object?.customGeometry;
    const width = number(geometry?.width);
    const height = number(geometry?.height);
    if (!geometry?.pathData || width < 0 || height < 0 || !(width > 0 || height > 0)) return false;
    const namespace = "http://www.w3.org/2000/svg";
    const svg = documentValue.createElementNS(namespace, "svg");
    const path = documentValue.createElementNS(namespace, "path");
    svg.classList.add("custom-geometry-svg");
    svg.setAttribute("viewBox", `${width > 0 ? 0 : -0.5} ${height > 0 ? 0 : -0.5} ${Math.max(1, width)} ${Math.max(1, height)}`);
    svg.setAttribute("preserveAspectRatio", "none");
    svg.setAttribute("aria-hidden", "true");
    path.setAttribute("d", geometry.pathData);
    if (object.shapeFillAsset) {
      const defs = documentValue.createElementNS(namespace, "defs");
      const pattern = documentValue.createElementNS(namespace, "pattern");
      const image = documentValue.createElementNS(namespace, "image");
      const id = `shape-fill-${String(object.id || "shape").replace(/[^a-z0-9_-]/gi, "-")}-${Math.random().toString(36).slice(2)}`;
      pattern.id = id;
      pattern.setAttribute("patternUnits", "userSpaceOnUse");
      pattern.setAttribute("width", String(Math.max(1, width)));
      pattern.setAttribute("height", String(Math.max(1, height)));
      const crop = imageFillMetrics(object.imageCrop, object.imageFillRect, Math.max(1, width), Math.max(1, height));
      image.setAttribute("href", object.shapeFillAsset);
      image.setAttribute("x", String(crop.x));
      image.setAttribute("y", String(crop.y));
      image.setAttribute("width", String(crop.width));
      image.setAttribute("height", String(crop.height));
      image.setAttribute("preserveAspectRatio", "none");
      pattern.append(image);
      defs.append(pattern);
      svg.append(defs);
      path.setAttribute("fill", `url(#${id})`);
    } else {
      path.setAttribute("fill", object.style?.fill || "transparent");
    }
    path.setAttribute("fill-rule", "evenodd");
    path.setAttribute("stroke", object.style?.stroke || "transparent");
    path.setAttribute("stroke-width", String(Math.max(0, number(object.style?.strokeWidth))));
    path.setAttribute("stroke-linecap", String(object.name || "").startsWith("墨迹") ? "round" : "butt");
    path.setAttribute("stroke-linejoin", String(object.name || "").startsWith("墨迹") ? "round" : "miter");
    path.setAttribute("stroke-dasharray", /dot/i.test(object.style?.strokeDash || "") ? "1 2" : /dash/i.test(object.style?.strokeDash || "") ? "6 4" : "none");
    path.setAttribute("vector-effect", "non-scaling-stroke");
    svg.append(path);
    node.classList.add("has-custom-geometry");
    Object.assign(node.style, { background: "transparent", border: "0", padding: "0", overflow: "visible" });
    node.append(svg);
    return true;
  }

  function bindHyperlinks(node, hyperlinks, options, object) {
    const click = hyperlinks?.click;
    const hover = hyperlinks?.hover;
    if (!click && !hover) return;
    node.dataset.hyperlink = "true";
    node.style.cursor = "pointer";
    const tooltip = click?.tooltip || hover?.tooltip;
    if (tooltip) node.title = tooltip;
    const activate = (link, event) => {
      const handled = options.onHyperlink?.(link, event, node, object);
      if (handled) {
        event.preventDefault?.();
        event.stopPropagation?.();
      }
    };
    if (click) node.addEventListener("click", (event) => activate(click, event));
    if (hover && options.hoverHyperlinks !== false) {
      node.addEventListener("pointerenter", (event) => {
        if (node.dataset.hoverLinkActivated === "1") return;
        const result = options.onHyperlink?.(hover, event, node, object);
        if (result) {
          node.dataset.hoverLinkActivated = "1";
          event.stopPropagation?.();
        }
      });
    }
  }

  function renderRichText(node, object, options, documentValue) {
    const objectTextStyle = object.textStyle || {};
    const flow = element(documentValue, "div", "object-content rich-text content");
    for (const paragraph of object.textParagraphs || []) {
      const line = element(documentValue, "div", "ppt-paragraph p");
      const paragraphSize = Math.max(1, ...(paragraph.runs || []).map((run) => number(run.fontSize, number(objectTextStyle.fontSize, 24))));
      const align = paragraph.align || objectTextStyle.align || "left";
      Object.assign(line.style, {
        whiteSpace: object.textFrame?.wordWrap === false ? "pre" : "pre-wrap",
        fontSize: `${paragraphSize}px`,
        textAlign: cssTextAlign(align),
        textAlignLast: isDistributedAlign(align) ? "justify" : "auto",
        paddingLeft: `${Math.max(0, number(paragraph.level)) * 24}px`,
        lineHeight: String(paragraph.lineSpacing || 1.18),
      });
      if (paragraph.spaceBefore != null) line.style.marginTop = `${paragraph.spaceBefore}px`;
      if (paragraph.spaceAfter != null) line.style.marginBottom = `${paragraph.spaceAfter}px`;
      if (paragraph.bullet) line.append(element(documentValue, "span", "ppt-bullet bullet", paragraph.bullet));
      for (const run of paragraph.runs || []) {
        const span = element(documentValue, "span", "ppt-run run", run.text || "");
        Object.assign(span.style, {
          fontFamily: run.fontFamily || run.nativeFontFamily || objectTextStyle.fontFamily || objectTextStyle.nativeFontFamily || "Aptos, sans-serif",
          fontSize: `${number(run.fontSize, number(objectTextStyle.fontSize, 24))}px`,
          color: run.color || objectTextStyle.color || "#172033",
          fontWeight: (run.bold ?? objectTextStyle.bold) ? "700" : "400",
          fontStyle: (run.italic ?? objectTextStyle.italic) ? "italic" : "normal",
          textDecoration: `${run.underline ? "underline" : ""}${run.strikethrough ? " line-through" : ""}`.trim() || "none",
          textDecorationStyle: /wavy/i.test(run.underlineStyle || "") ? "wavy" : /dbl/i.test(run.underlineStyle || "") ? "double" : "solid",
          verticalAlign: Number.isFinite(run.baselineOffset) ? `${run.baselineOffset * number(run.fontSize, number(objectTextStyle.fontSize,24)) / 100}px` : run.baseline === "super" ? "super" : run.baseline === "sub" ? "sub" : "baseline",
          opacity: String(run.alpha ?? 1),
        });
        applyTextGradient(span, run.gradient);
        bindHyperlinks(span, run.hyperlinks, options, run);
        line.append(span);
      }
      flow.append(line);
    }
    node.append(flow);
  }

  function applyTableCellBorders(node, borders) {
    for (const [side, cssName] of [["left", "borderLeft"], ["right", "borderRight"], ["top", "borderTop"], ["bottom", "borderBottom"]]) {
      const border = borders?.[side];
      node.style[cssName] = border
        ? `${Math.max(0, number(border.width))}px ${dashCss(border.dash)} ${border.color || "transparent"}`
        : "0 solid transparent";
    }
  }

  function renderTable(node, object, options, documentValue) {
    const table = object.table;
    const grid = element(documentValue, "div", "ppt-table-grid table-grid");
    const columns = table.columns?.length
      ? table.columns
      : Array.from({ length: Math.max(1, ...table.rows.map((row) => row.cells?.length || 0)) }, () => 1);
    grid.style.gridTemplateColumns = columns.map((value) => `minmax(0,${Math.max(1, number(value, 1))}fr)`).join(" ");
    grid.style.gridTemplateRows = table.rows.map((row) => `minmax(0,${Math.max(1, number(row.height, 1))}fr)`).join(" ");
    table.rows.forEach((row, rowIndex) => {
      (row.cells || []).forEach((cell, columnIndex) => {
        if (cell.hMerge || cell.vMerge) return;
        const cellNode = element(documentValue, "div", "ppt-table-cell table-cell");
        const textStyle = cell.textStyle || object.textStyle || {};
        const textFrame = cell.textFrame || {};
        cellNode.style.gridColumn = `${columnIndex + 1} / span ${Math.max(1, number(cell.gridSpan, 1))}`;
        cellNode.style.gridRow = `${rowIndex + 1} / span ${Math.max(1, number(cell.rowSpan, 1))}`;
        cellNode.dataset.textFlow = textFlowMode(textFrame.verticalType);
        Object.assign(cellNode.style, {
          background: cell.fill || "transparent",
          color: textStyle.color || "#172033",
          fontFamily: textStyle.fontFamily || textStyle.nativeFontFamily || "Aptos, sans-serif",
          fontSize: `${number(textStyle.fontSize, 18)}px`,
          fontWeight: textStyle.bold ? "700" : "400",
          fontStyle: textStyle.italic ? "italic" : "normal",
          textAlign: cssTextAlign(textStyle.align || "left"),
          padding: `${textFrame.marginTop ?? 5}px ${textFrame.marginRight ?? 8}px ${textFrame.marginBottom ?? 5}px ${textFrame.marginLeft ?? 8}px`,
          alignItems: textFrame.verticalAlign === "top" ? "flex-start" : textFrame.verticalAlign === "bottom" ? "flex-end" : "center",
        });
        applyTableCellBorders(cellNode, cell.borders || {});
        if (cell.textParagraphs?.length) renderRichText(cellNode, cell, options, documentValue);
        else cellNode.append(element(documentValue, "div", "object-content", cell.text || ""));
        grid.append(cellNode);
      });
    });
    node.style.padding = "0";
    node.append(grid);
  }

  function renderMath(node, formula, options) {
    const source = formula?.latex || "Office 公式";
    const katex = options.katex || global.katex;
    if (katex?.render && formula?.latex) katex.render(source, node, { displayMode: Boolean(formula.display), throwOnError: false });
    else node.textContent = source;
  }

  function placeholderLabel(object) {
    return ({ chart: "图表", table: "表格", smartArt: "SmartArt", ole: "OLE 嵌入对象", unknown: object?.name || "未识别对象", connector: "" })[object?.kind] || "";
  }

  function dynamicContentForObject(object, options) {
    const objects = options.dynamicObjects || {};
    return objects[object?.id] || Object.values(objects).find((entry) => entry?.objectName === object?.name) || null;
  }

  function renderDynamicContent(node, object, options, documentValue) {
    const dynamic = dynamicContentForObject(object, options);
    if (!dynamic) return false;
    node.classList.add("dynamic-content-object");
    const frame = element(documentValue, "iframe", "dynamic-content-frame");
    frame.title = object.name || "动态内容";
    frame.setAttribute("sandbox", dynamic.allowScripts ? "allow-scripts allow-forms allow-popups" : "");
    frame.srcdoc = dynamic.kind === "svg"
      ? `<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;width:100%;height:100%;overflow:hidden}svg{display:block;width:100%;height:100%}</style></head><body>${dynamic.source || ""}</body></html>`
      : String(dynamic.source || "");
    node.append(frame);
    return true;
  }

  function renderObject(object, container, options = {}, z = 0) {
    if (!object || !container) return null;
    const documentValue = options.document || container.ownerDocument || global.document;
    if (!documentValue) return null;
    if (object.kind === "group" && !object.children?.length) {
      return renderObject({ ...object, kind: "placeholder", text: options.emptyGroupLabel || "组合对象" }, container, options, z);
    }
    const kind = object.kind || "unknown";
    const node = element(documentValue, "div", `scene-object o kind-${kind}${kind === "group" ? " group" : ""}${kind === "math" ? " math formula" : ""}`);
    applyObjectStyle(node, object, z);
    bindHyperlinks(node, object.hyperlinks, options, object);
    if (kind === "group") {
      (object.children || []).forEach((child, index) => renderObject(child, node, options, index));
      container.append(node);
      return node;
    }

    const hasGeometry = renderCustomGeometry(node, object, documentValue) || renderLinearGeometry(node, object, documentValue);
    if (renderDynamicContent(node, object, options, documentValue)) {
      // Portable interactive content runs in a non-same-origin sandbox.
    } else if (kind === "image" && object.asset) {
      const image = element(documentValue, "img");
      image.src = object.asset;
      image.alt = object.name || "";
      applyImageCrop(image, object.imageCrop);
      node.append(image);
      applyImageEffects(node, image, object.imageEffects, documentValue);
    } else if (kind === "table" && object.table?.rows?.length) {
      renderTable(node, object, options, documentValue);
    } else if (kind === "chart" && object.chart) {
      const chartRuntime = options.chartRuntime || global.UniPPTChartRuntime;
      if (!chartRuntime?.render?.(node, object.chart)) node.append(element(documentValue, "div", "object-content", object.chart.title || object.name || "Chart"));
    } else if (kind === "math") {
      renderMath(node, object.formula, options);
    } else if (object.textParagraphs?.length) {
      renderRichText(node, object, options, documentValue);
    } else if (!hasGeometry || object.text || placeholderLabel(object)) {
      node.append(element(documentValue, "div", "object-content", object.text || placeholderLabel(object)));
    }
    renderShapeFillImage(node, object, documentValue);
    const attachMedia = options.attachMedia || global.UniPptMedia?.attach;
    attachMedia?.(node, object, false, options.mediaContext || null);
    container.append(node);
    return node;
  }

  function fitTextToShape(node) {
    const flow = node.querySelector?.(":scope > .rich-text");
    if (!flow) return 1;
    const runs = [...flow.querySelectorAll(".ppt-run")];
    const paragraphs = [...flow.querySelectorAll(".ppt-paragraph")];
    if (!runs.length || !paragraphs.length) return 1;
    for (const run of runs) run.dataset.baseFontSize ||= String(parseFloat(run.style.fontSize) || 1);
    for (const paragraph of paragraphs) paragraph.dataset.baseFontSize ||= String(parseFloat(paragraph.style.fontSize) || 1);
    const computed = global.getComputedStyle?.(node) || { paddingLeft: "0", paddingRight: "0", paddingTop: "0", paddingBottom: "0" };
    const width = Math.max(1, node.clientWidth - number(parseFloat(computed.paddingLeft)) - number(parseFloat(computed.paddingRight)));
    const height = Math.max(1, node.clientHeight - number(parseFloat(computed.paddingTop)) - number(parseFloat(computed.paddingBottom)));
    const cacheKey = [
      Math.round(width * 10), Math.round(height * 10), node.dataset.id || "", flow.textContent || "",
      runs.map((run) => `${run.dataset.baseFontSize}:${run.style.fontFamily}`).join("|"),
    ].join(";");
    const applyScale = (scale) => {
      for (const run of runs) run.style.fontSize = `${Math.max(0.5, number(run.dataset.baseFontSize, 1) * scale)}px`;
      for (const paragraph of paragraphs) paragraph.style.fontSize = `${Math.max(0.5, number(paragraph.dataset.baseFontSize, 1) * scale)}px`;
    };
    const fits = () => flow.scrollWidth <= width + 0.5 && flow.scrollHeight <= height + 0.5;
    const cached = autoTextFitCache.get(cacheKey);
    if (Number.isFinite(cached)) {
      applyScale(cached);
      if (fits()) return cached;
    }
    applyScale(1);
    if (fits()) {
      autoTextFitCache.set(cacheKey, 1);
      return 1;
    }
    let low = 0.12;
    let high = 1;
    for (let index = 0; index < 12; index += 1) {
      const middle = (low + high) / 2;
      applyScale(middle);
      if (fits()) low = middle;
      else high = middle;
    }
    const result = Math.max(0.1, low * 0.985);
    applyScale(result);
    autoTextFitCache.set(cacheKey, result);
    return result;
  }

  function fitAutoTextInContainer(container) {
    if (!container?.querySelectorAll || !container.isConnected) return;
    for (const node of container.querySelectorAll('[data-text-auto-size="textToFitShape"]')) fitTextToShape(node);
    for (const node of container.querySelectorAll('[data-text-wrap="none"][data-text-flow^="vertical-"]')) fitVerticalNoWrapText(node);
  }

  function fitVerticalNoWrapText(node) {
    const flow = node.querySelector?.(":scope > .rich-text");
    if (!flow) return;
    const availableHeight = Math.max(1, flow.clientHeight);
    for (const paragraph of flow.querySelectorAll(":scope > .ppt-paragraph")) {
      paragraph.style.letterSpacing = "";
      if (paragraph.scrollHeight <= availableHeight + 0.5) continue;
      const computed = global.getComputedStyle?.(paragraph) || { fontSize: "1" };
      const fontSize = Math.max(1, number(parseFloat(computed.fontSize), 1));
      let fitting = -fontSize * 0.5;
      let overflowing = 0;
      paragraph.style.letterSpacing = `${fitting}px`;
      if (paragraph.scrollHeight > availableHeight + 0.5) continue;
      for (let index = 0; index < 12; index += 1) {
        const middle = (fitting + overflowing) / 2;
        paragraph.style.letterSpacing = `${middle}px`;
        if (paragraph.scrollHeight <= availableHeight + 0.5) fitting = middle;
        else overflowing = middle;
      }
      paragraph.style.letterSpacing = `${fitting - 0.05}px`;
    }
  }

  function scheduleAutoFit(container, options) {
    if (options.autoFit === false) return;
    const fit = () => fitAutoTextInContainer(container);
    if (typeof global.queueMicrotask === "function") global.queueMicrotask(fit);
    else Promise.resolve().then(fit);
    const fontsReady = container.ownerDocument?.fonts?.ready;
    if (fontsReady?.then) fontsReady.then(fit, () => {});
  }

  function renderSlide(slide, container, options = {}) {
    if (!slide || !container) return { nodes: [], layers: [] };
    const documentValue = options.document || container.ownerDocument || global.document;
    installStyles(documentValue);
    container.classList?.add("unippt-shared-scene");
    if (options.clear !== false) container.replaceChildren();
    if (options.deckWidth != null) container.style.width = `${Math.max(1, number(options.deckWidth, 1))}px`;
    if (options.deckHeight != null) container.style.height = `${Math.max(1, number(options.deckHeight, 1))}px`;
    if (options.applyBackground !== false) applySlideBackground(container, slide);
    const layers = orderedLayerObjects(slide);
    const nodes = layers.map(({ object, z }) => renderObject(object, container, { ...options, document: documentValue }, z)).filter(Boolean);
    scheduleAutoFit(container, options);
    return { nodes, layers };
  }

  global.UniPptPresentationScene = Object.freeze({
    renderSlide,
    renderObject,
    orderedLayerObjects,
    applySlideBackground,
    applyObjectStyle,
    fitAutoTextInContainer,
    fitTextToShape,
    fitVerticalNoWrapText,
    isLinearGeometry,
    imageCropMetrics,
    imageFillMetrics,
    textFlowMode,
  });
})(typeof globalThis !== "undefined" ? globalThis : window);
