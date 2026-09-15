(() => {
  "use strict";

  const FONT_FIELD = /^(?:nativeFontFamily|fontFamily|typeface|fontFace|fontName|latinTypeface|eastAsianTypeface)$/i;

  function primaryFamily(value) {
    const first = String(value || "").split(",")[0].trim();
    return first.replace(/^["']|["']$/g, "").trim();
  }

  function addFamily(families, value) {
    const family = primaryFamily(value);
    if (!family || /^(?:inherit|initial|unset|sans-serif|serif|monospace)$/i.test(family)) return;
    const key = family.toLocaleLowerCase();
    if (!families.has(key)) families.set(key, family);
  }

  function scanNativeFonts(families, slots) {
    if (!slots || typeof slots !== "object") return;
    for (const key of ["latin", "eastAsia", "complexScript", "symbol"]) {
      addFamily(families, slots[key]);
    }
  }

  function scanTextStyle(families, style) {
    if (!style || typeof style !== "object") return;
    addFamily(families, style.nativeFontFamily || style.fontFamily);
    scanNativeFonts(families, style.nativeFonts);
  }

  function scanParagraphs(families, paragraphs) {
    for (const paragraph of paragraphs || []) {
      scanTextStyle(families, paragraph?.textStyle);
      for (const run of paragraph?.runs || []) {
        addFamily(families, run?.nativeFontFamily || run?.fontFamily);
        scanNativeFonts(families, run?.nativeFonts);
      }
    }
  }

  // Chart text models differ between producers. Traverse only the chart model
  // and collect explicitly named font fields; assets and arbitrary CSS never
  // enter this path.
  function scanChartFontFields(families, value, seen = new Set()) {
    if (!value || typeof value !== "object" || seen.has(value)) return;
    seen.add(value);
    if (Array.isArray(value)) {
      for (const item of value) scanChartFontFields(families, item, seen);
      return;
    }
    for (const [key, child] of Object.entries(value)) {
      if (FONT_FIELD.test(key)) addFamily(families, child);
      else if (child && typeof child === "object") scanChartFontFields(families, child, seen);
    }
  }

  function scanObject(families, object) {
    if (!object || typeof object !== "object") return;
    scanTextStyle(families, object.textStyle);
    scanParagraphs(families, object.textParagraphs);
    for (const row of object.table?.rows || []) {
      for (const cell of row?.cells || []) {
        scanTextStyle(families, cell?.textStyle);
        scanParagraphs(families, cell?.textParagraphs);
      }
    }
    scanChartFontFields(families, object.chart);
    for (const child of object.children || []) scanObject(families, child);
  }

  function collectDeckFontFamilies(deck) {
    const families = new Map();
    for (const slide of deck?.slides || []) {
      for (const layer of [slide.masterObjects, slide.layoutObjects, slide.objects]) {
        for (const object of layer || []) scanObject(families, object);
      }
    }
    return [...families.values()];
  }

  const api = { primaryFamily, collectDeckFontFamilies };
  if (typeof module === "object" && module.exports) module.exports = api;
  globalThis.UniPptFonts = api;
})();
