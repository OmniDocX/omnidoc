(function (global) {
  "use strict";

  const ACCEPT = ".pptx,.udoc,.html,.htm,image/*,.txt,.md,.json,.csv,.xml,.yaml,.yml";
  const IMAGE_LIMIT = 12 * 1024 * 1024;
  const DOCUMENT_LIMIT = 128 * 1024 * 1024;
  const HTML_LIMIT = 16 * 1024 * 1024;
  const TEXT_LIMIT = 4 * 1024 * 1024;
  const SUMMARY_LIMIT = 140000;
  const OMIT_VALUE = /^data(?:Uri|Url)?$|^bytes$|^blob$|^binary$|^native(?:Xml|Bytes|Package)?$|^sourcePptx$/i;

  function extension(name) {
    return String(name || "").toLowerCase().match(/\.([^.]+)$/)?.[1] || "";
  }

  function isImageFile(file) {
    return /^image\//i.test(file?.type || "") || /^(png|jpe?g|webp|gif|bmp|svg|avif)$/i.test(extension(file?.name));
  }

  function documentRoute(file) {
    const ext = extension(file?.name);
    if (ext === "pptx") return {
      endpoint: "/api/import-pptx",
      format: "PPTX",
      contentType: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    };
    if (ext === "udoc") return { endpoint: "/api/import-udoc", format: "UDOC", contentType: "application/vnd.unidoc" };
    if (ext === "html" || ext === "htm") return { endpoint: "/api/import-html", format: "HTML", contentType: "text/html" };
    return null;
  }

  function readableSize(bytes) {
    const value = Math.max(0, Number(bytes) || 0);
    if (value < 1024) return `${value} B`;
    if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
    return `${(value / 1024 / 1024).toFixed(1)} MiB`;
  }

  function clippedString(value, limit = 12000) {
    const text = String(value || "");
    return text.length <= limit ? text : `${text.slice(0, limit)}…[已截断 ${text.length - limit} 字符]`;
  }

  function compactValue(value, depth = 0, key = "") {
    if (OMIT_VALUE.test(key)) return `[${key} 二进制内容已省略]`;
    if (value == null || typeof value === "number" || typeof value === "boolean") return value;
    if (typeof value === "string") return clippedString(value);
    if (depth >= 5) return Array.isArray(value) ? `[${value.length} 项]` : "[嵌套结构已省略]";
    if (Array.isArray(value)) {
      const limited = value.slice(0, 120).map((item) => compactValue(item, depth + 1, key));
      if (value.length > limited.length) limited.push(`[另有 ${value.length - limited.length} 项]`);
      return limited;
    }
    if (typeof value === "object") {
      const output = {};
      for (const [childKey, childValue] of Object.entries(value)) {
        if (/^(extensions|sourceImportId|sourcePartName|sourceRelationshipId)$/i.test(childKey)) continue;
        output[childKey] = compactValue(childValue, depth + 1, childKey);
      }
      return output;
    }
    return String(value);
  }

  function compactObject(object, layer) {
    const output = { layer };
    const keys = [
      "id", "type", "name", "text", "altText", "description", "x", "y", "width", "height",
      "rotation", "opacity", "shapeType", "style", "richText", "table", "chart", "media",
      "hyperlink", "children",
    ];
    for (const key of keys) {
      if (object?.[key] !== undefined && object[key] !== null && object[key] !== "") {
        output[key] = compactValue(object[key], 0, key);
      }
    }
    return output;
  }

  function summarizeDeck(deck, file) {
    const slides = (deck?.slides || []).map((slide, index) => ({
      index: index + 1,
      id: slide.id,
      name: slide.name,
      notes: clippedString(slide.notes || "", 16000),
      transition: compactValue(slide.transition, 0, "transition"),
      animations: compactValue([...(slide.inheritedAnimations || []), ...(slide.animations || [])], 0, "animations"),
      objects: [
        ...(slide.masterObjects || []).map((object) => compactObject(object, "master")),
        ...(slide.layoutObjects || []).map((object) => compactObject(object, "layout")),
        ...(slide.objects || []).map((object) => compactObject(object, "slide")),
      ],
    }));
    const summary = {
      attachment: file?.name || "presentation",
      title: deck?.title || "",
      format: deck?.format || "",
      slideCount: slides.length,
      canvas: { width: deck?.width, height: deck?.height },
      fonts: (deck?.fonts || []).map((font) => ({ family: font.family, weight: font.weight, style: font.style })),
      slides,
    };
    const text = JSON.stringify(summary, null, 2);
    if (text.length <= SUMMARY_LIMIT) return text;
    return `${text.slice(0, SUMMARY_LIMIT)}\n…[附件结构过长，已截断 ${text.length - SUMMARY_LIMIT} 字符；可指定页码继续分析]`;
  }

  function readDataUrl(file) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result || ""));
      reader.onerror = () => reject(reader.error || new Error("图片附件读取失败"));
      reader.readAsDataURL(file);
    });
  }

  async function imageDimensions(file, dataUrl) {
    try {
      if (typeof global.createImageBitmap === "function") {
        const bitmap = await global.createImageBitmap(file);
        const dimensions = { width: bitmap.width, height: bitmap.height };
        bitmap.close?.();
        return dimensions;
      }
      if (typeof global.Image === "function") {
        return await new Promise((resolve) => {
          const image = new global.Image();
          image.onload = () => resolve({ width: image.naturalWidth, height: image.naturalHeight });
          image.onerror = () => resolve({ width: 0, height: 0 });
          image.src = dataUrl;
        });
      }
    } catch (_) {}
    return { width: 0, height: 0 };
  }

  async function readJson(response) {
    const text = await response.text();
    if (!response.ok) {
      try { throw new Error(JSON.parse(text)?.error || text || `HTTP ${response.status}`); }
      catch (error) { if (error instanceof SyntaxError) throw new Error(text || `HTTP ${response.status}`); throw error; }
    }
    try { return JSON.parse(text); }
    catch (_) { throw new Error(`附件解析服务返回了无效 JSON（${text.length} 字节）`); }
  }

  function median(values) {
    const sorted = values.slice().sort((a, b) => a - b);
    return sorted.length ? sorted[Math.floor(sorted.length / 2)] : 0;
  }

  function rgbHex(red, green, blue) {
    return `#${[red, green, blue].map((value) => Math.max(0, Math.min(255, Math.round(value))).toString(16).padStart(2, "0")).join("")}`;
  }

  function clamp(value, minimum, maximum) {
    return Math.max(minimum, Math.min(maximum, value));
  }

  function regionPolygon(region, width, height) {
    const location = Array.isArray(region?.location) ? region.location.map(Number) : [];
    if (location.length !== 8 || location.some((value) => !Number.isFinite(value))) return null;
    const points = [0, 2, 4, 6].map((offset) => ({
      x: clamp(location[offset], 0, width - 1),
      y: clamp(location[offset + 1], 0, height - 1),
    }));
    const topLength = Math.max(1, Math.hypot(points[1].x - points[0].x, points[1].y - points[0].y));
    const leftLength = Math.max(1, Math.hypot(points[3].x - points[0].x, points[3].y - points[0].y));
    const ux = (points[1].x - points[0].x) / topLength;
    const uy = (points[1].y - points[0].y) / topLength;
    const vx = (points[3].x - points[0].x) / leftLength;
    const vy = (points[3].y - points[0].y) / leftLength;
    const shortEdge = Math.max(1, Math.min(topLength, leftLength));
    const along = clamp(shortEdge * .16, 2, Math.max(2, Math.min(width, height) * .018));
    const across = clamp(shortEdge * .30, 3, Math.max(3, Math.min(width, height) * .026));
    const expanded = [
      { x: points[0].x - ux * along - vx * across, y: points[0].y - uy * along - vy * across },
      { x: points[1].x + ux * along - vx * across, y: points[1].y + uy * along - vy * across },
      { x: points[2].x + ux * along + vx * across, y: points[2].y + uy * along + vy * across },
      { x: points[3].x - ux * along + vx * across, y: points[3].y - uy * along + vy * across },
    ].map((point) => ({ x: clamp(point.x, 0, width - 1), y: clamp(point.y, 0, height - 1) }));
    return { points, expanded, shortEdge };
  }

  function polygonBounds(points, width, height) {
    const xs = points.map((point) => point.x);
    const ys = points.map((point) => point.y);
    return {
      left: clamp(Math.floor(Math.min(...xs)), 0, width - 1),
      right: clamp(Math.ceil(Math.max(...xs)), 0, width - 1),
      top: clamp(Math.floor(Math.min(...ys)), 0, height - 1),
      bottom: clamp(Math.ceil(Math.max(...ys)), 0, height - 1),
    };
  }

  function pointInPolygon(x, y, points) {
    let inside = false;
    for (let current = 0, previous = points.length - 1; current < points.length; previous = current++) {
      const a = points[current], b = points[previous];
      const crosses = (a.y > y) !== (b.y > y)
        && x < (b.x - a.x) * (y - a.y) / ((b.y - a.y) || Number.EPSILON) + a.x;
      if (crosses) inside = !inside;
    }
    return inside;
  }

  function pointSegmentDistance(x, y, start, end) {
    const dx = end.x - start.x, dy = end.y - start.y;
    const lengthSquared = dx * dx + dy * dy;
    const amount = lengthSquared ? clamp(((x - start.x) * dx + (y - start.y) * dy) / lengthSquared, 0, 1) : 0;
    return Math.hypot(x - (start.x + dx * amount), y - (start.y + dy * amount));
  }

  function pixelColor(pixels, width, height, mask, x, y, radius = 2) {
    const colors = [[], [], []];
    for (let py = Math.max(0, y - radius); py <= Math.min(height - 1, y + radius); py += 1) {
      for (let px = Math.max(0, x - radius); px <= Math.min(width - 1, x + radius); px += 1) {
        const index = py * width + px;
        if (mask[index]) continue;
        const offset = index * 4;
        if (pixels[offset + 3] < 220) continue;
        colors[0].push(pixels[offset]); colors[1].push(pixels[offset + 1]); colors[2].push(pixels[offset + 2]);
      }
    }
    return colors[0].length ? colors.map(median) : null;
  }

  function colorDistance(first, second) {
    if (!first || !second) return Number.POSITIVE_INFINITY;
    return Math.hypot(first[0] - second[0], first[1] - second[1], first[2] - second[2]);
  }

  function interpolateColor(first, second, amount) {
    if (!first) return second;
    if (!second) return first;
    return first.map((value, index) => value + (second[index] - value) * amount);
  }

  function removeTextRegionsFromPixels(inputPixels, width, height, regions) {
    const source = new Uint8ClampedArray(inputPixels || []);
    const output = new Uint8ClampedArray(source);
    if (source.length !== width * height * 4) return { pixels: output, regions, cleaning: { strategy: "none", regionCount: 0, complexRegionCount: 0 } };
    const plans = (regions || []).map((region, index) => {
      const geometry = regionPolygon(region, width, height);
      return geometry ? { region, index, ...geometry, bounds: polygonBounds(geometry.expanded, width, height) } : null;
    }).filter(Boolean);
    if (!plans.length) return { pixels: output, regions, cleaning: { strategy: "none", regionCount: 0, complexRegionCount: 0 } };

    // One union mask prevents a nearby line of text from becoming the sample
    // source for another line. This is the most common cause of faint glyph
    // echoes when several OCR boxes sit close together.
    const unionMask = new Uint8Array(width * height);
    for (const plan of plans) {
      for (let y = plan.bounds.top; y <= plan.bounds.bottom; y += 1) {
        for (let x = plan.bounds.left; x <= plan.bounds.right; x += 1) {
          if (pointInPolygon(x + .5, y + .5, plan.expanded)) unionMask[y * width + x] = 1;
        }
      }
    }

    let complexRegionCount = 0;
    const decorated = Array.from(regions || []);
    for (const plan of plans) {
      const rows = new Map(), columns = new Map(), boundaryColors = [];
      for (let y = plan.bounds.top; y <= plan.bounds.bottom; y += 1) {
        let first = null, last = null;
        for (let x = plan.bounds.left; x <= plan.bounds.right; x += 1) {
          if (!pointInPolygon(x + .5, y + .5, plan.expanded)) continue;
          if (first == null) first = x;
          last = x;
        }
        if (first == null) continue;
        let left = first - 1, right = last + 1;
        while (left >= 0 && unionMask[y * width + left]) left -= 1;
        while (right < width && unionMask[y * width + right]) right += 1;
        const leftColor = left >= 0 ? pixelColor(source, width, height, unionMask, left, y) : null;
        const rightColor = right < width ? pixelColor(source, width, height, unionMask, right, y) : null;
        if (leftColor) boundaryColors.push(leftColor);
        if (rightColor) boundaryColors.push(rightColor);
        rows.set(y, { first, last, leftColor, rightColor, difference: colorDistance(leftColor, rightColor) });
      }
      for (let x = plan.bounds.left; x <= plan.bounds.right; x += 1) {
        let first = null, last = null;
        for (let y = plan.bounds.top; y <= plan.bounds.bottom; y += 1) {
          if (!pointInPolygon(x + .5, y + .5, plan.expanded)) continue;
          if (first == null) first = y;
          last = y;
        }
        if (first == null) continue;
        let top = first - 1, bottom = last + 1;
        while (top >= 0 && unionMask[top * width + x]) top -= 1;
        while (bottom < height && unionMask[bottom * width + x]) bottom += 1;
        const topColor = top >= 0 ? pixelColor(source, width, height, unionMask, x, top) : null;
        const bottomColor = bottom < height ? pixelColor(source, width, height, unionMask, x, bottom) : null;
        if (topColor) boundaryColors.push(topColor);
        if (bottomColor) boundaryColors.push(bottomColor);
        columns.set(x, { first, last, topColor, bottomColor, difference: colorDistance(topColor, bottomColor) });
      }

      const channels = [0, 1, 2].map((channel) => boundaryColors.map((color) => color[channel]));
      const background = channels[0].length ? channels.map(median) : [11, 16, 32];
      const lumas = boundaryColors.map((color) => color[0] * .299 + color[1] * .587 + color[2] * .114);
      const meanLuma = lumas.length ? lumas.reduce((sum, value) => sum + value, 0) / lumas.length : 0;
      const deviation = lumas.length ? Math.sqrt(lumas.reduce((sum, value) => sum + (value - meanLuma) ** 2, 0) / lumas.length) : 0;
      const repairMode = deviation < 9 ? "flat" : deviation < 28 ? "smooth-gradient" : "complex";
      if (repairMode === "complex") complexRegionCount += 1;

      for (let y = plan.bounds.top; y <= plan.bounds.bottom; y += 1) {
        const row = rows.get(y);
        if (!row) continue;
        for (let x = row.first; x <= row.last; x += 1) {
          if (!pointInPolygon(x + .5, y + .5, plan.expanded)) continue;
          const column = columns.get(x);
          const horizontal = interpolateColor(row.leftColor, row.rightColor, (x - row.first) / Math.max(1, row.last - row.first));
          const vertical = column
            ? interpolateColor(column.topColor, column.bottomColor, (y - column.first) / Math.max(1, column.last - column.first))
            : null;
          const horizontalWeight = horizontal ? 1 / (18 + (Number.isFinite(row.difference) ? row.difference : 255)) : 0;
          const verticalWeight = vertical ? 1 / (18 + (Number.isFinite(column?.difference) ? column.difference : 255)) : 0;
          const candidate = horizontalWeight + verticalWeight
            ? [0, 1, 2].map((channel) => ((horizontal?.[channel] || 0) * horizontalWeight + (vertical?.[channel] || 0) * verticalWeight) / (horizontalWeight + verticalWeight))
            : background;
          const edgeDistance = Math.min(...plan.expanded.map((point, index) => pointSegmentDistance(x + .5, y + .5, point, plan.expanded[(index + 1) % plan.expanded.length])));
          const opacity = clamp(edgeDistance / 2.25, 0, 1);
          const offset = (y * width + x) * 4;
          for (let channel = 0; channel < 3; channel += 1) output[offset + channel] = source[offset + channel] * (1 - opacity) + candidate[channel] * opacity;
        }
      }

      const luminance = background[0] * .299 + background[1] * .587 + background[2] * .114;
      decorated[plan.index] = {
        ...plan.region,
        maskLocation: plan.expanded.flatMap((point) => [Math.round(point.x * 100) / 100, Math.round(point.y * 100) / 100]),
        background: rgbHex(...background),
        foreground: luminance < 150 ? "#ffffff" : "#172033",
        backgroundComplexity: Math.round(deviation * 100) / 100,
        repairMode,
      };
    }
    return {
      pixels: output,
      regions: decorated,
      cleaning: { strategy: "directional-interpolation-v1", regionCount: plans.length, complexRegionCount },
    };
  }

  function percentile(values, amount) {
    if (!values.length) return 0;
    const sorted = values.slice().sort((first, second) => first - second);
    const index = clamp(Math.round((sorted.length - 1) * amount), 0, sorted.length - 1);
    return sorted[index];
  }

  function buildTextMaskFromPixels(inputPixels, width, height, regions) {
    const source = new Uint8ClampedArray(inputPixels || []);
    const pixels = new Uint8ClampedArray(width * height * 4);
    for (let offset = 3; offset < pixels.length; offset += 4) pixels[offset] = 255;
    if (source.length !== width * height * 4) {
      return { pixels, maskedPixelCount: 0, regionCount: 0, foregroundColors: [] };
    }
    let maskedPixelCount = 0;
    let regionCount = 0;
    const foregroundColors = [];
    for (let regionIndex = 0; regionIndex < (regions || []).length; regionIndex += 1) {
      const geometry = regionPolygon(regions[regionIndex], width, height);
      if (!geometry) continue;
      const rawBounds = polygonBounds(geometry.points, width, height);
      const bounds = {
        left: Math.max(0, rawBounds.left - 8),
        right: Math.min(width - 1, rawBounds.right + 8),
        top: Math.max(0, rawBounds.top - 8),
        bottom: Math.min(height - 1, rawBounds.bottom + 8),
      };
      const support = [], background = [];
      for (let y = bounds.top; y <= bounds.bottom; y += 1) {
        for (let x = bounds.left; x <= bounds.right; x += 1) {
          const offset = (y * width + x) * 4;
          if (source[offset + 3] < 220) continue;
          const inside = pointInPolygon(x + .5, y + .5, geometry.points);
          if (inside) {
            support.push({ x, y, offset });
            continue;
          }
          const edgeDistance = Math.min(...geometry.points.map((point, index) =>
            pointSegmentDistance(x + .5, y + .5, point, geometry.points[(index + 1) % geometry.points.length])));
          if (edgeDistance <= 7) background.push([source[offset], source[offset + 1], source[offset + 2]]);
        }
      }
      if (!support.length) continue;
      if (!background.length) {
        for (const item of support) {
          if (item.x === rawBounds.left || item.x === rawBounds.right || item.y === rawBounds.top || item.y === rawBounds.bottom) {
            background.push([source[item.offset], source[item.offset + 1], source[item.offset + 2]]);
          }
        }
      }
      const backgroundColor = background.length
        ? [0, 1, 2].map((channel) => median(background.map((color) => color[channel])))
        : [0, 0, 0];
      const distances = support.map((item) => colorDistance(
        [source[item.offset], source[item.offset + 1], source[item.offset + 2]],
        backgroundColor,
      ));
      const threshold = Math.max(22, percentile(distances, .62) * .48);
      let selected = support.filter((_, index) => distances[index] >= threshold);
      const unreliableSegmentation = selected.length < Math.max(12, Math.round(support.length * .008));
      if (unreliableSegmentation) selected = support;
      const selectedDistances = selected.map((item) => colorDistance(
        [source[item.offset], source[item.offset + 1], source[item.offset + 2]],
        backgroundColor,
      ));
      const coreThreshold = percentile(selectedDistances, .85);
      const core = selected.filter((_, index) => selectedDistances[index] >= coreThreshold);
      const foregroundPixels = core.length ? core : selected;
      const foreground = [0, 1, 2].map((channel) => median(foregroundPixels.map((item) => source[item.offset + channel])));
      foregroundColors[regionIndex] = rgbHex(...foreground);
      for (const item of selected) {
        const offset = (item.y * width + item.x) * 4;
        if (pixels[offset] === 0) maskedPixelCount += 1;
        pixels[offset] = 255;
        pixels[offset + 1] = 255;
        pixels[offset + 2] = 255;
      }
      regionCount += 1;
    }
    return { pixels, maskedPixelCount, regionCount, foregroundColors };
  }

  async function decorateOcrRegions(attachment, detection, options = {}) {
    const regions = Array.isArray(detection?.regions) ? detection.regions : [];
    if (!regions.length || typeof global.Image !== "function" || !global.document?.createElement) return detection;
    try {
      const image = await new Promise((resolve, reject) => {
        const node = new global.Image();
        node.onload = () => resolve(node);
        node.onerror = reject;
        node.src = attachment.dataUrl;
      });
      const canvas = global.document.createElement("canvas");
      canvas.width = image.naturalWidth || attachment.width;
      canvas.height = image.naturalHeight || attachment.height;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      context.drawImage(image, 0, 0, canvas.width, canvas.height);
      const imageData = context.getImageData(0, 0, canvas.width, canvas.height);
      const repaired = removeTextRegionsFromPixels(imageData.data, canvas.width, canvas.height, regions);
      const mask = buildTextMaskFromPixels(imageData.data, canvas.width, canvas.height, regions);
      const decoratedRegions = repaired.regions.map((region, index) => ({
        ...region,
        foreground: mask.foregroundColors[index] || region.foreground,
      }));
      if (!mask.maskedPixelCount) throw new Error("OCR 文字蒙版为空");
      const maskCanvas = global.document.createElement("canvas");
      maskCanvas.width = canvas.width;
      maskCanvas.height = canvas.height;
      const maskContext = maskCanvas.getContext("2d");
      const maskImageData = maskContext.createImageData
        ? maskContext.createImageData(canvas.width, canvas.height)
        : context.getImageData(0, 0, canvas.width, canvas.height);
      maskImageData.data.set(mask.pixels);
      maskContext.putImageData(maskImageData, 0, 0);
      const maskDataUrl = maskCanvas.toDataURL("image/png");
      try {
        const response = await (options.fetch || global.fetch)("/api/ai/inpaint", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            image: attachment.dataUrl,
            mask: maskDataUrl,
            maskDilate: 6,
            featherRadius: 0,
            contextMargin: 128,
            roiMode: "auto",
          }),
          signal: options.signal,
        });
        const cleaned = await readJson(response);
        if (!/^data:image\//i.test(cleaned?.image || "") || cleaned.image.length > 24 * 1024 * 1024) {
          throw new Error("clean-image 未返回有效图片");
        }
        return {
          ...detection,
          regions: decoratedRegions,
          cleanedImage: cleaned.image,
          cleaning: { ...(cleaned.cleaning || {}), model: cleaned.model, maskedPixelCount: mask.maskedPixelCount },
        };
      } catch (remoteError) {
        imageData.data.set(repaired.pixels);
        context.putImageData?.(imageData, 0, 0);
        let cleanedImage = "";
        if (typeof canvas.toDataURL === "function") {
          cleanedImage = canvas.toDataURL("image/webp", .96);
          if (!/^data:image\//i.test(cleanedImage) || cleanedImage.length > 18 * 1024 * 1024) cleanedImage = "";
        }
        return {
          ...detection,
          regions: decoratedRegions,
          cleanedImage: cleanedImage || undefined,
          cleaning: {
            ...repaired.cleaning,
            strategy: "directional-interpolation-v1-fallback",
            maskedPixelCount: mask.maskedPixelCount,
            fallbackReason: remoteError?.message || String(remoteError),
          },
        };
      }
    } catch (error) {
      return {
        ...detection,
        cleaning: {
          strategy: "mask-analysis-failed",
          fallbackReason: error?.message || String(error),
        },
      };
    }
  }

  async function detectImageText(attachment, options = {}) {
    if (attachment?.kind !== "image" || !attachment.dataUrl) throw new Error("OCR 需要有效的图片附件");
    if (!attachment.width || !attachment.height) throw new Error("OCR 需要图片原始尺寸");
    const controller = typeof global.AbortController === "function" ? new global.AbortController() : null;
    const abort = () => controller?.abort(options.signal?.reason);
    if (options.signal?.aborted) abort();
    else options.signal?.addEventListener?.("abort", abort, { once: true });
    const timer = controller && typeof global.setTimeout === "function" ? global.setTimeout(abort, options.timeoutMs || 120000) : null;
    try {
    const response = await (options.fetch || global.fetch)("/api/ai/ocr", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        attachmentName: attachment.name,
        image: attachment.dataUrl,
        width: attachment.width,
        height: attachment.height,
      }),
      signal: controller?.signal || options.signal,
    });
    const detection = await readJson(response);
    if (options.detectOnly) return { ...detection, regions: Array.isArray(detection.regions) ? detection.regions : [] };
    if (!Array.isArray(detection.regions) || !detection.regions.length) throw new Error("OCR 没有检测到文字坐标");
    const decorated = await decorateOcrRegions(attachment, detection, options);
    if (options.requireNeuralBackground) {
      const strategy = String(decorated?.cleaning?.strategy || "");
      const hasBackground = /^data:image\//i.test(String(decorated?.cleanedImage || ""));
      if (!hasBackground || !strategy.startsWith("big-lama")) {
        const reason = String(decorated?.cleaning?.fallbackReason || "clean-image 未返回神经擦字背景");
        throw new Error(`AI 擦字失败，已阻止生成低质量色块遮罩：${reason}`);
      }
    }
    return decorated;
    } finally {
      if (timer != null) global.clearTimeout(timer);
      options.signal?.removeEventListener?.("abort", abort);
    }
  }

  async function prepareFile(file, options = {}) {
    if (!file?.name) throw new Error("附件没有文件名");
    if (isImageFile(file)) {
      if (file.size > IMAGE_LIMIT) throw new Error(`${file.name} 超过 12 MiB 图片上限`);
      const dataUrl = await readDataUrl(file);
      const dimensions = await imageDimensions(file, dataUrl);
      return {
        kind: "image", name: file.name, size: file.size, mimeType: file.type || `image/${extension(file.name)}`,
        dataUrl, ...dimensions,
      };
    }
    const route = documentRoute(file);
    if (route) {
      const fileLimit = route.format === "HTML" ? HTML_LIMIT : DOCUMENT_LIMIT;
      if (file.size > fileLimit) throw new Error(`${file.name} 超过 ${route.format === "HTML" ? 16 : 128} MiB 文稿上限`);
      const freeHtmlSource = route.format === "HTML" && typeof file.text === "function" ? await file.text() : "";
      if (route.format === "HTML") {
        let deck = null;
        try {
          const response = await (options.fetch || global.fetch)(route.endpoint, {
            method: "POST",
            headers: { "Content-Type": route.contentType, "X-UniPPT-Filename": encodeURIComponent(file.name) },
            body: file,
          });
          deck = await readJson(response);
        } catch (_) { /* generic free HTML may not be a UniPPT lossless package */ }
        return {
          kind: "html", name: file.name, size: file.size, format: "FREE HTML", html: freeHtmlSource,
          slideCount: deck?.slides?.length || 0,
          text: deck
            ? summarizeDeck(deck, file)
            : "【自由 HTML 源码保留在本机；明确要求转换时由浏览器布局编译器读取，不发送到模型】",
        };
      }
      try {
        const response = await (options.fetch || global.fetch)(route.endpoint, {
          method: "POST",
          headers: { "Content-Type": route.contentType, "X-UniPPT-Filename": encodeURIComponent(file.name) },
          body: file,
        });
        const deck = await readJson(response);
        return {
          kind: "document", name: file.name, size: file.size, format: route.format,
          slideCount: deck?.slides?.length || 0, text: summarizeDeck(deck, file),
        };
      } catch (error) { throw error; }
    }
    if (!/\.(txt|md|json|csv|xml|ya?ml)$/i.test(file.name)) throw new Error(`${file.name} 不是支持的附件类型`);
    if (file.size > TEXT_LIMIT) throw new Error(`${file.name} 超过 4 MiB 文本上限`);
    return { kind: "text", name: file.name, size: file.size, format: extension(file.name).toUpperCase(), text: clippedString(await file.text(), SUMMARY_LIMIT) };
  }

  async function prepareFiles(files, options = {}) {
    const prepared = [];
    for (const file of Array.from(files || []).slice(0, 8)) prepared.push(await prepareFile(file, options));
    return prepared;
  }

  function messagePayload(prompt, attachments) {
    const items = Array.from(attachments || []);
    const instruction = String(prompt || "").trim() || "请分析所附附件，概括内容并指出可以如何用于当前演示文稿。";
    const images = items.filter((item) => item.kind === "image");
    const imageText = images.map((item) => {
      const dimensions = item.width && item.height ? `；原始尺寸 ${item.width}×${item.height}px` : "";
      return `【图片附件：${item.name}${dimensions}；请以该文件名引用图片】`;
    }).join("\n");
    const documentText = items.filter((item) => item.kind !== "image").map((item) => {
      const meta = [item.format, item.slideCount ? `${item.slideCount} 页` : "", readableSize(item.size)].filter(Boolean).join(" · ");
      if (item.kind === "html") return `【附件：${item.name}${meta ? `（${meta}）` : ""}；完整 HTML 源码仅保留在本机附件运行时，不发送到模型】`;
      return `【附件：${item.name}${meta ? `（${meta}）` : ""}】\n${item.text || ""}`;
    }).join("\n\n");
    const text = [instruction, imageText, documentText].filter(Boolean).join("\n\n");
    const content = images.length
      ? [{ type: "text", text }, ...images.map((item) => ({ type: "image_url", image_url: { url: item.dataUrl, detail: "auto" } }))]
      : text;
    const imageNames = images.map((item) => item.name).join("、");
    return {
      content,
      displayText: instruction,
      conversationText: `${text}${imageNames ? `\n\n【本轮图片附件：${imageNames}；图片已作为视觉输入发送】` : ""}`,
    };
  }

  global.UniPptAiAttachments = Object.freeze({
    ACCEPT, IMAGE_LIMIT, DOCUMENT_LIMIT, HTML_LIMIT, TEXT_LIMIT,
    extension, isImageFile, documentRoute, readableSize, summarizeDeck,
    prepareFile, prepareFiles, messagePayload, detectImageText, decorateOcrRegions,
    regionPolygon, removeTextRegionsFromPixels, buildTextMaskFromPixels,
  });
})(globalThis);
