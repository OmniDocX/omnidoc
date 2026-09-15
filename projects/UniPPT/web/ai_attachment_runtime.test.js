"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "ai_attachment_runtime.js"), "utf8");

function runtime(extra = {}) {
  const context = { console, encodeURIComponent, ...extra };
  context.globalThis = context;
  vm.runInNewContext(source, context, { filename: "ai_attachment_runtime.js" });
  return context.UniPptAiAttachments;
}

function jsonResponse(value, status = 200) {
  return { ok: status >= 200 && status < 300, status, text: async () => JSON.stringify(value) };
}

test("native OCR mode never runs inpainting and permits visual-only recovery", async () => {
  const calls = [];
  const ai = runtime();
  const detection = await ai.detectImageText({ kind: "image", name: "figure.png", dataUrl: "data:image/png;base64,AAAA", width: 100, height: 100 }, { detectOnly: true, fetch: async (url) => { calls.push(url); return jsonResponse({ regions: [] }); } });
  assert.equal(detection.regions.length, 0);
  assert.deepEqual(calls, ["/api/ai/ocr"]);
});

test("AI attachments recognize PPTX, UDOC, lossless HTML and images", () => {
  const ai = runtime();
  assert.equal(ai.documentRoute({ name: "deck.pptx" }).endpoint, "/api/import-pptx");
  assert.equal(ai.documentRoute({ name: "deck.udoc" }).endpoint, "/api/import-udoc");
  assert.equal(ai.documentRoute({ name: "deck.html" }).endpoint, "/api/import-html");
  assert.equal(ai.isImageFile({ name: "photo.webp", type: "" }), true);
  assert.match(ai.ACCEPT, /\.pptx/);
  assert.match(ai.ACCEPT, /\.udoc/);
  assert.match(ai.ACCEPT, /image\/\*/);
});

test("PPTX attachment is parsed without replacing the active document", async () => {
  const calls = [];
  const ai = runtime();
  const file = { name: "source.pptx", size: 4096 };
  const attachment = await ai.prepareFile(file, { fetch: async (url, options) => {
    calls.push({ url, options });
    return jsonResponse({ title: "Source", width: 13.33, height: 7.5, slides: [{ id: "s1", name: "封面", objects: [{ id: "o1", type: "text", text: "多模态演示" }] }] });
  } });
  assert.equal(calls[0].url, "/api/import-pptx");
  assert.equal(calls[0].options.body, file);
  assert.equal(attachment.kind, "document");
  assert.equal(attachment.slideCount, 1);
  assert.match(attachment.text, /多模态演示/);
});

test("generic HTML falls back to a source-preserving free HTML attachment", async () => {
  const ai = runtime();
  const sourceHtml = '<!doctype html><section class="omnidoc-page"><h1>自由布局</h1></section>';
  const file = { name: "free.html", size: sourceHtml.length, text: async () => sourceHtml };
  const attachment = await ai.prepareFile(file, { fetch: async () => jsonResponse({ error: "not lossless" }, 422) });
  assert.equal(attachment.kind, "html");
  assert.equal(attachment.format, "FREE HTML");
  assert.equal(attachment.html, sourceHtml);
  assert.match(attachment.text, /源码保留在本机/);
  assert.doesNotMatch(attachment.text, /<section/);
  const payload = ai.messagePayload("转换为可编辑 PPT", [attachment]);
  const serialized = JSON.stringify(payload);
  assert.match(serialized, /free\.html/);
  assert.doesNotMatch(serialized, /自由布局|<section/);
});

test("recognized HTML keeps its exact source local while exposing only a compact summary to the model", async () => {
  const ai = runtime();
  const sourceHtml = '<!doctype html><html><body><section class="omnidoc-page"><h1>商业机密页面</h1></section></body></html>';
  const file = { name: "recognized.html", size: sourceHtml.length, text: async () => sourceHtml };
  const attachment = await ai.prepareFile(file, { fetch: async () => jsonResponse({ title: "摘要标题", slides: [{ name: "第 1 页" }] }) });
  assert.equal(attachment.kind, "html");
  assert.equal(attachment.html, sourceHtml);
  assert.match(attachment.text, /摘要标题/);
  assert.doesNotMatch(attachment.text, /商业机密页面|<!doctype html>/i);
  const payload = ai.messagePayload("导入", [attachment]);
  assert.doesNotMatch(JSON.stringify(payload), /商业机密页面|<!doctype html>/i);
});

test("image attachments become OpenAI-compatible multimodal content", () => {
  const ai = runtime();
  const payload = ai.messagePayload("比较版式", [
    { kind: "image", name: "reference.png", size: 100, dataUrl: "data:image/png;base64,AAAA" },
    { kind: "document", name: "source.udoc", format: "UDOC", slideCount: 2, size: 200, text: "两页结构" },
  ]);
  assert.equal(payload.content[0].type, "text");
  assert.match(payload.content[0].text, /source\.udoc/);
  assert.match(payload.content[0].text, /图片附件：reference\.png/);
  assert.equal(payload.content[1].type, "image_url");
  assert.equal(payload.content[1].image_url.url, "data:image/png;base64,AAAA");
  assert.equal(payload.content[1].image_url.detail, "auto");
});

test("image reconstruction preflights through the dedicated OCR coordinate endpoint", async () => {
  const calls = [];
  const ai = runtime();
  const detection = await ai.detectImageText({
    kind: "image", name: "poster.png", width: 1600, height: 900,
    dataUrl: "data:image/png;base64,AAAA",
  }, { fetch: async (url, options) => {
    calls.push({ url, options });
    return jsonResponse({
      ok: true, model: "qwen3.5-ocr", image: { width: 1600, height: 900 },
      regions: [{ text: "智启未来", location: [80, 100, 720, 100, 720, 260, 80, 260] }],
    });
  } });
  assert.equal(calls[0].url, "/api/ai/ocr");
  assert.equal(calls[0].options.method, "POST");
  assert.equal(JSON.parse(calls[0].options.body).attachmentName, "poster.png");
  assert.equal(detection.regions[0].text, "智启未来");
  assert.equal(detection.regions[0].location.length, 8);
});

test("image reconstruction refuses a missing neural cleaned background instead of producing overlay blocks", async () => {
  const ai = runtime();
  await assert.rejects(
    ai.detectImageText({
      kind: "image", name: "poster.png", width: 1600, height: 900,
      dataUrl: "data:image/png;base64,AAAA",
    }, {
      requireNeuralBackground: true,
      fetch: async () => jsonResponse({
        ok: true, model: "qwen3.5-ocr", image: { width: 1600, height: 900 },
        regions: [{ text: "智启未来", location: [80, 100, 720, 100, 720, 260, 80, 260] }],
      }),
    }),
    /已阻止生成低质量色块遮罩/,
  );
});

test("contrast glyph mask selects text pixels without deleting the OCR rectangle background", () => {
  const ai = runtime();
  const width = 40, height = 20;
  const pixels = new Uint8ClampedArray(width * height * 4);
  for (let index = 0; index < width * height; index += 1) pixels[index * 4 + 3] = 255;
  for (let y = 8; y < 13; y += 1) {
    for (let x = 10; x < 20; x += 1) {
      const offset = (y * width + x) * 4;
      pixels[offset] = 255; pixels[offset + 1] = 255; pixels[offset + 2] = 255;
    }
  }
  const mask = ai.buildTextMaskFromPixels(pixels, width, height, [
    { text: "TEST", location: [5, 5, 35, 5, 35, 15, 5, 15] },
  ]);
  const textOffset = (9 * width + 12) * 4;
  const backgroundOffset = (9 * width + 30) * 4;
  assert.equal(mask.pixels[textOffset], 255);
  assert.equal(mask.pixels[backgroundOffset], 0);
  assert.equal(mask.maskedPixelCount, 50);
  assert.equal(mask.foregroundColors[0], "#ffffff");
});

test("contrast mask never emits clustered rich-text runs", () => {
  const ai = runtime();
  const width = 48, height = 20;
  const pixels = new Uint8ClampedArray(width * height * 4);
  for (let index = 0; index < width * height; index += 1) {
    pixels[index * 4] = 255;
    pixels[index * 4 + 1] = 255;
    pixels[index * 4 + 2] = 255;
    pixels[index * 4 + 3] = 255;
  }
  for (let y = 8; y < 13; y += 1) {
    for (let x = 8; x < 18; x += 1) {
      const offset = (y * width + x) * 4;
      pixels[offset] = 30; pixels[offset + 1] = 30; pixels[offset + 2] = 30;
    }
    for (let x = 24; x < 38; x += 1) {
      const offset = (y * width + x) * 4;
      pixels[offset] = 15; pixels[offset + 1] = 170; pixels[offset + 2] = 105;
    }
  }
  const mask = ai.buildTextMaskFromPixels(pixels, width, height, [
    { text: "black and green", location: [4, 4, 43, 4, 43, 16, 4, 16] },
  ]);
  assert.equal(mask.foregroundColors.length, 1);
  assert.equal(Object.hasOwn(mask, "foregroundRuns"), false);
});

test("browser reconstruction is wired to the authenticated server-side inpaint proxy", () => {
  assert.match(source, /\"\/api\/ai\/inpaint\"/);
  assert.match(source, /directional-interpolation-v1-fallback/);
});
