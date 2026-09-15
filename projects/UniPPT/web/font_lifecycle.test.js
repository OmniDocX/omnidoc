"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const fonts = require("./font_catalog.js");

const deck = {
  slides: [{
    masterObjects: [{
      textStyle: { nativeFontFamily: "母版原生体", fontFamily: "Fallback" },
      textParagraphs: [{ runs: [{
        text: "M",
        nativeFontFamily: "Master Run",
        nativeFonts: {
          latin: "Master Latin",
          eastAsia: "Master East Asia",
          complexScript: "Master Complex",
          symbol: "Master Symbol",
          languageId: "zh-CN",
        },
      }] }],
    }],
    layoutObjects: [{
      textStyle: { fontFamily: "'Layout Exact', Arial, sans-serif" },
    }],
    objects: [{
      kind: "group",
      children: [{
        textStyle: { nativeFontFamily: "Child Font" },
        table: { rows: [{ cells: [{
          textStyle: { nativeFontFamily: "Table Font" },
          textParagraphs: [{ runs: [{ text: "T", fontFamily: "'Table Run', serif" }] }],
        }] }] },
        chart: {
          title: { textStyle: { nativeFontFamily: "Chart Title" } },
          axes: [{ labels: { fontFamily: "'Chart Labels', sans-serif" } }],
          unrelated: { css: "font-family: Should Not Be Parsed" },
        },
      }],
    }],
  }],
};

assert.deepEqual(fonts.collectDeckFontFamilies(deck), [
  "母版原生体",
  "Master Run",
  "Master Latin",
  "Master East Asia",
  "Master Complex",
  "Master Symbol",
  "Layout Exact",
  "Child Font",
  "Table Font",
  "Table Run",
  "Chart Title",
  "Chart Labels",
]);
assert.equal(fonts.primaryFamily("'造字工房力黑（非商用）常规体', 'Microsoft YaHei'"), "造字工房力黑（非商用）常规体");

const source = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
assert.match(html, /font:\s*\{[\s\S]*?\{\{asset:\/font_catalog\.js\}\}[\s\S]*?\{\{asset:\/font_runtime\.js\}\}/, "verified font runtime must have a dedicated deferred feature");
assert.match(html, /render:\s*\{[\s\S]*?dependsOn:\s*\["font"\]/, "render helpers must reuse the font feature");
assert.match(html, /markFirstPaint\(\)[\s\S]*?loadFeature\("font"\)[\s\S]*?loadFeature\("render"\)/, "font discovery must start only after first paint and before heavier rendering work");
assert.match(source, /function installEmbeddedFonts\(deck\)[\s\S]*const generation = \+\+fontInstallGeneration/, "font installs must own a generation");
assert.match(source, /scheduleFontPreparation[\s\S]*generation !== fontInstallGeneration[\s\S]*deck !== state\.deck/, "stale document font loads must not repaint the active deck");
assert.match(source, /prepareDeckFonts\(deck\)/, "fingerprinted fonts must use the verified local-first runtime");
assert.match(source, /function openDocumentPicker\(\)[\s\S]*authorizeLocalFonts[\s\S]*fileInput/, "Open must request Local Font Access inside the trusted gesture");
assert.match(source, /function scheduleFontPreparation[\s\S]*requestIdleCallback/, "font hashing and fallback must stay on the post-paint idle path");
assert.match(source, /document\.fonts\.addEventListener\("loadingdone"[\s\S]*scheduleFontMetricRefresh/, "late system font loads must invalidate metrics");
assert.match(source, /function scheduleFontMetricRefresh[\s\S]*renderSlideList\(\)[\s\S]*renderCanvas\(\)[\s\S]*renderPresentation[\s\S]*scheduleFitSlide/, "font settlement must refresh thumbnail, canvas, slideshow, and fit");
assert.match(source, /function refreshFontCatalog[\s\S]*collectDeckFontFamilies[\s\S]*dataset\.documentFont/, "actual and embedded document fonts must populate the ribbon");
assert.match(source, /function syncTextRibbon[\s\S]*run\?\.nativeFontFamily[\s\S]*style\.nativeFontFamily/, "the ribbon must echo the exact native run/style family");
assert.match(source, /paragraphAlign[\s\S]*alignLeft[\s\S]*aria-pressed/, "the ribbon must echo the selected object paragraph alignment");
assert.doesNotMatch(source, /font\.css|\.css\)\.join/, "document-provided raw font CSS must never be injected");

console.log("font lifecycle tests passed");
