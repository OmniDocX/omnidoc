"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

function runtime() {
  const context = vm.createContext({ console });
  context.globalThis = context;
  vm.runInContext(fs.readFileSync(path.join(__dirname, "html_to_ppt_runtime.js"), "utf8"), context, { filename: "html_to_ppt_runtime.js" });
  return context.UniPptHtmlToPpt;
}

test("measured free HTML becomes editable native slide objects with a fidelity report", () => {
  const api = runtime();
  const result = api.compileMeasuredPages([{
    width: 1600, height: 900, background: "rgb(10, 17, 40)",
    elements: [
      { type: "box", name: "hero", rect: { x: 80, y: 90, width: 1440, height: 300 }, fill: "#10233d", stroke: "#00ddf5", strokeWidth: 2, radius: 18, opacity: 1, zIndex: 0, order: 0 },
      { type: "text", name: "h1", text: "AI 重塑企业", rect: { x: 120, y: 140, width: 900, height: 100 }, fontFamily: "Noto Sans SC", fontSize: 64, color: "#f8f9fa", bold: true, align: "left", opacity: 1, zIndex: 1, order: 1 },
      { type: "image", name: "visual", asset: "data:image/png;base64,AAAA", rect: { x: 1120, y: 440, width: 320, height: 300 }, opacity: 1, zIndex: 2, order: 2 },
    ],
    unsupported: [{ tag: "script", reason: "scripts-disabled", count: 1 }],
  }], { title: "HTML 战略稿", sourceName: "strategy.html", width: 1280, height: 720 });

  assert.equal(result.slides.length, 1);
  assert.equal(result.slides[0].background, "#0A1128");
  assert.deepEqual(JSON.parse(JSON.stringify(result.slides[0].objects.map((object) => object.kind))), ["shape", "text", "image"]);
  assert.equal(result.slides[0].objects[0].geometry, "roundRect");
  assert.equal(result.slides[0].objects[1].textStyle.fontSize, 51.2);
  assert.equal(result.slides[0].objects[1].frame.x, 96);
  assert.equal(result.slides[0].objects[1].textFrame.autoSize, "textToFitShape");
  assert.equal(result.report.nativeText, 1);
  assert.equal(result.report.nativeShapes, 1);
  assert.equal(result.report.preservedImages, 1);
  assert.equal(result.report.editabilityRatio, 66.7);
  assert.equal(result.report.fidelityRisk, "medium");
  assert.match(result.slides[0].notes, /unsupported=1/);
});

test("browser virtual fonts become explicit Office script slots", () => {
  const fonts = runtime().__test.officeFontProfile({
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Arial, sans-serif',
  }, "AI重塑生产力革命");

  assert.equal(fonts.cssFamily, '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Arial, sans-serif');
  assert.equal(fonts.nativeFontFamily, "Segoe UI");
  assert.deepEqual(JSON.parse(JSON.stringify(fonts.nativeFonts)), {
    latin: "Segoe UI", eastAsia: "Microsoft YaHei", complexScript: "Segoe UI",
    symbol: "Segoe UI Symbol", languageId: "zh-CN",
  });
});

test("a CJK-first CSS family remains first for Latin glyph metrics and preserves script language", () => {
  const api = runtime().__test;
  const chinese = api.officeFontProfile({ fontFamily: '"Microsoft YaHei", sans-serif' }, "AI生产力");
  assert.equal(chinese.nativeFonts.latin, "Microsoft YaHei");
  assert.equal(chinese.nativeFonts.eastAsia, "Microsoft YaHei");
  assert.equal(chinese.nativeFonts.languageId, "zh-CN");

  const japanese = api.officeFontProfile({ fontFamily: '"Noto Sans JP", sans-serif' }, "AIカタカナ");
  assert.equal(japanese.nativeFonts.latin, "Noto Sans JP");
  assert.equal(japanese.nativeFonts.eastAsia, "Noto Sans JP");
  assert.equal(japanese.nativeFonts.languageId, "ja-JP");

  const korean = api.officeFontProfile({ fontFamily: '"Microsoft YaHei", sans-serif' }, "AI한글");
  assert.equal(korean.nativeFonts.languageId, "ko-KR");
});

test("Office text frames use the available CSS content width plus a metric safety margin", () => {
  const rect = runtime().__test.officeTextRect(
    { left: 100, top: 100, right: 318, bottom: 156, width: 218, height: 56 },
    { left: 100, top: 90, right: 720, bottom: 210, width: 620, height: 120 },
    { fontSize: "56px", textAlign: "left", writingMode: "horizontal-tb", paddingRight: "16px" },
    { left: 0, top: 0, right: 1280, bottom: 720, width: 1280, height: 720 },
    false,
  );

  assert.equal(rect.left, 100);
  assert.equal(rect.right, 704);
  assert.equal(rect.width, 604);
  assert.ok(rect.height > 56);
});

test("compiled single-line text disables PowerPoint wrapping and carries native font slots", () => {
  const result = runtime().compileMeasuredPages([{
    width: 1280, height: 720, background: "#fff", unsupported: [], fallbacks: [],
    elements: [{
      type: "text", name: "cover-title", text: "智启未来", rect: { x: 100, y: 100, width: 600, height: 64 },
      fontFamily: '-apple-system, "Segoe UI", sans-serif', nativeFontFamily: "Segoe UI",
      nativeFonts: { latin: "Segoe UI", eastAsia: "Microsoft YaHei", complexScript: "Segoe UI", symbol: "Segoe UI Symbol", languageId: "zh-CN" },
      fontSize: 56, color: "#fff", opacity: 1, zIndex: 1, order: 1, wordWrap: false,
    }],
  }], {});
  const object = result.slides[0].objects[0];
  assert.equal(object.textFrame.wordWrap, false);
  assert.equal(object.textFrame.autoSize, "textToFitShape");
  assert.equal(object.textStyle.nativeFontFamily, "Segoe UI");
  assert.equal(object.textStyle.nativeFonts.eastAsia, "Microsoft YaHei");
  assert.equal(object.textParagraphs[0].lineSpacing, 1.2);
  assert.equal(object.textParagraphs[0].runs[0].nativeFonts.eastAsia, "Microsoft YaHei");
});

test("centered inline rich text is measured once at its block container instead of as overlapping full-width boxes", () => {
  const api = runtime();
  const document = {
    createRange() {
      let selected;
      return {
        selectNodeContents(node) { selected = node; },
        getBoundingClientRect() { return selected.rect; },
        getClientRects() { return [selected.rect]; },
        detach() {},
      };
    },
  };
  const blockStyle = { display: "block", visibility: "visible", opacity: "1", textAlign: "center", writingMode: "horizontal-tb", fontSize: "32px", paddingLeft: "20px", paddingRight: "20px", color: "#111", fontWeight: "400", fontStyle: "normal" };
  const inlineStyle = (color, weight, family = "Segoe UI") => ({ display: "inline", visibility: "visible", opacity: "1", color, fontWeight: weight, fontStyle: "normal", fontSize: "32px", fontFamily: family });
  const blockRect = { left: 100, top: 80, right: 500, bottom: 150, width: 400, height: 70 };
  const block = { nodeType: 1, tagName: "P", childNodes: [], parentElement: null, ownerDocument: document, style: blockStyle, getBoundingClientRect: () => blockRect };
  const span = (text, rect, style) => {
    const textNode = { nodeType: 3, nodeValue: text, rect };
    const element = { nodeType: 1, tagName: "SPAN", childNodes: [textNode], parentElement: block, ownerDocument: document, style, getBoundingClientRect: () => rect };
    return element;
  };
  block.childNodes = [
    span("AI ", { left: 205, top: 96, right: 250, bottom: 132, width: 45, height: 36 }, inlineStyle("#00B7FF", "700")),
    span("重塑", { left: 250, top: 96, right: 315, bottom: 132, width: 65, height: 36 }, inlineStyle("#FFFFFF", "400", "Microsoft YaHei")),
    span("未来", { left: 315, top: 96, right: 380, bottom: 132, width: 65, height: 36 }, inlineStyle("#FFB000", "600", "Microsoft YaHei")),
  ];
  const view = { getComputedStyle: (element) => element.style };
  const root = { left: 0, top: 0, right: 1280, bottom: 720, width: 1280, height: 720 };

  const measured = api.__test.directTextRuns(block, root, view);
  assert.equal(measured.length, 1);
  assert.equal(measured[0].text, "AI 重塑未来");
  assert.equal(measured[0].rect.left, 120);
  assert.equal(measured[0].rect.right, 480);
  assert.equal(measured[0].richRuns.length, 3);
  assert.equal(api.__test.directTextRuns(block.childNodes[0], root, view).length, 0);
});

test("compiled HTML rich text stays one textbox and preserves mixed inline formatting", () => {
  const result = runtime().compileMeasuredPages([{
    width: 1280, height: 720, background: "#081426", unsupported: [], fallbacks: [],
    elements: [{
      type: "text", name: "mixed-title", text: "AI 重塑未来", rect: { x: 120, y: 96, width: 360, height: 48 },
      fontFamily: "Segoe UI", nativeFontFamily: "Segoe UI", nativeFonts: { latin: "Segoe UI", eastAsia: "Microsoft YaHei" },
      fontSize: 32, color: "#FFFFFF", align: "center", opacity: 1, zIndex: 1, order: 1, wordWrap: false,
      runs: [
        { text: "AI ", fontFamily: "Segoe UI", nativeFontFamily: "Segoe UI", nativeFonts: { latin: "Segoe UI", eastAsia: "Segoe UI" }, fontSize: 32, color: "#00B7FF", bold: true, italic: false },
        { text: "重塑", fontFamily: "Microsoft YaHei", nativeFontFamily: "Segoe UI", nativeFonts: { latin: "Segoe UI", eastAsia: "Microsoft YaHei" }, fontSize: 32, color: "#FFFFFF", bold: false, italic: false },
        { text: "未来", fontFamily: "Microsoft YaHei", nativeFontFamily: "Segoe UI", nativeFonts: { latin: "Segoe UI", eastAsia: "Microsoft YaHei" }, fontSize: 36, color: "#FFB000", bold: true, italic: true },
      ],
    }],
  }], {});

  assert.equal(result.slides[0].objects.length, 1);
  const object = result.slides[0].objects[0];
  assert.equal(object.kind, "text");
  assert.equal(object.text, "AI 重塑未来");
  assert.equal(object.textStyle.align, "center");
  assert.equal(object.textParagraphs[0].runs.length, 3);
  assert.equal(object.textParagraphs[0].runs[0].color, "#00B7FF");
  assert.equal(object.textParagraphs[0].runs[0].bold, true);
  assert.equal(object.textParagraphs[0].runs[1].nativeFonts.eastAsia, "Microsoft YaHei");
  assert.equal(object.textParagraphs[0].runs[2].fontSize, 36);
  assert.equal(object.textParagraphs[0].runs[2].italic, true);
});

test("CSS white-space collapses source newlines but preserves pre-line and explicit breaks", () => {
  const api = runtime().__test;
  const normal = api.coalesceTextFragments([
    { text: "第一行\n    只是源码换行", style: { whiteSpace: "normal" } },
  ]);
  assert.equal(normal.map((run) => run.text).join(""), "第一行 只是源码换行");

  const preLine = api.coalesceTextFragments([
    { text: "第一行\n    第二行", style: { whiteSpace: "pre-line" } },
  ]);
  assert.equal(preLine.map((run) => run.text).join(""), "第一行\n第二行");

  const hardBreak = api.coalesceTextFragments([
    { text: "第一行", style: { whiteSpace: "normal" } },
    { text: "\n", hardBreak: true, style: { whiteSpace: "normal" } },
    { text: "第二行", style: { whiteSpace: "normal" } },
  ]);
  assert.equal(hardBreak.map((run) => run.text).join(""), "第一行\n第二行");
});

test("vertical HTML text keeps vertical writing direction in the PPT text frame", () => {
  const result = runtime().compileMeasuredPages([{
    width: 1280, height: 720, background: "#fff", unsupported: [],
    elements: [{
      type: "text", name: "vertical", text: "战略", rect: { x: 100, y: 80, width: 60, height: 200 },
      fontFamily: "Microsoft YaHei", fontSize: 32, color: "#111", align: "center", writingMode: "vertical-rl",
      opacity: 1, zIndex: 0, order: 0,
    }],
  }], {});
  assert.equal(result.slides[0].objects[0].textFrame.verticalType, "vert");
  assert.equal(result.report.fidelityRisk, "low");
});

test("authored page semantics and source notes survive the editable HTML compilation", () => {
  const result = runtime().compileMeasuredPages([{
    width: 1280, height: 720, background: "#071426", unsupported: [], fallbacks: [],
    title: "AI 改变的不是工具，而是工作流", role: "comparison",
    notes: "先解释左右对照。\n[Sources]\n- https://example.test/report\n[/Sources]",
    elements: [
      { type: "box", name: "comparison-panel", rect: { x: 60, y: 150, width: 1160, height: 460 }, fill: "#10233d", stroke: "transparent", strokeWidth: 0, radius: 24, opacity: 1, zIndex: 0, order: 0 },
      { type: "text", name: "slide-title", text: "AI 改变的不是工具，而是工作流", rect: { x: 70, y: 60, width: 1080, height: 64 }, fontFamily: "Microsoft YaHei", fontSize: 42, color: "#fff", opacity: 1, zIndex: 1, order: 1 },
    ],
  }], { title: "AI 演讲", sourceName: "ai-deck.html" });

  assert.equal(result.slides[0].name, "AI 改变的不是工具，而是工作流");
  assert.equal(result.slides[0].semantic.role, "comparison");
  assert.equal(result.slides[0].semantic.summary, "AI 改变的不是工具，而是工作流");
  assert.match(result.slides[0].notes, /\[Sources\][\s\S]*example\.test\/report[\s\S]*\[\/Sources\]/);
  assert.match(result.slides[0].notes, /\[HTML Import\]/);
  assert.match(result.slides[0].objects[1].name, /slide-title/);
  assert.equal(result.slides[0].objects[1].semanticRole, "slide-title");
});

test("an unmarked final HTML page falls back to the closing semantic role", () => {
  const page = (title) => ({ width: 1280, height: 720, background: "#fff", title, role: "", notes: "", unsupported: [], fallbacks: [], elements: [] });
  const result = runtime().compileMeasuredPages([page("开始"), page("结束")], {});
  assert.equal(result.slides[0].semantic.role, "cover");
  assert.equal(result.slides[1].semantic.role, "closing");
});

test("bounded local raster fallbacks stay movable and are reported separately from hard failures", () => {
  const result = runtime().compileMeasuredPages([{
    width: 1280, height: 720, background: "#fff", unsupported: [],
    fallbacks: [{ tag: "div", reason: "css-filter", mode: "localized-raster", rect: { x: 100, y: 80, width: 320, height: 180 } }],
    elements: [
      { type: "image", fallback: true, name: "filtered-card", asset: "data:image/png;base64,AAAA", rect: { x: 100, y: 80, width: 320, height: 180 }, opacity: 1, zIndex: 0, order: 0 },
      { type: "text", name: "editable-neighbor", text: "仍然可编辑", rect: { x: 500, y: 100, width: 260, height: 50 }, fontFamily: "Microsoft YaHei", fontSize: 28, color: "#111", opacity: 1, zIndex: 1, order: 1 },
    ],
  }], {});
  assert.deepEqual(JSON.parse(JSON.stringify(result.slides[0].objects.map((object) => object.kind))), ["image", "text"]);
  assert.match(result.slides[0].objects[0].name, /^HTML 保真片段/);
  assert.equal(result.report.localizedFallbacks, 1);
  assert.equal(result.report.fallbacks[0].reason, "css-filter");
  assert.equal(result.report.unsupported.length, 0);
  assert.equal(result.report.fidelityRisk, "medium");
  assert.match(result.slides[0].notes, /localizedFallbacks=1/);
});

test("CSS alpha colors remain translucent instead of becoming opaque white", () => {
  const colors = runtime().__test;
  assert.equal(colors.visibleColor("#ffffff0b"), "#FFFFFF");
  assert.ok(Math.abs(colors.colorAlpha("#ffffff0b") - 11 / 255) < 1e-9);
  assert.ok(Math.abs(colors.colorAlpha("rgba(255, 255, 255, 0.12)") - 0.12) < 1e-9);
  assert.ok(Math.abs(colors.colorAlpha("color(srgb 0.9 0.7 0.1 / 22%)") - 0.22) < 1e-9);
  assert.equal(colors.colorAlpha("transparent"), 0);
  assert.equal(colors.compositeColor("#ffffff20", "#0b1020"), "#2A2E3C");
  assert.equal(colors.resolvedBorder({
    borderTopWidth: "0px", borderRightWidth: "0px", borderBottomWidth: "0px", borderLeftWidth: "0px",
    borderTopStyle: "none", borderTopColor: "rgb(232, 238, 252)",
  }, "#0b1020").stroke, "transparent");
  assert.equal(colors.resolvedBorder({
    borderTopWidth: "1px", borderRightWidth: "1px", borderBottomWidth: "1px", borderLeftWidth: "1px",
    borderTopStyle: "solid", borderTopColor: "#ffffff20",
  }, "#0b1020").stroke, "#2A2E3C");
});
