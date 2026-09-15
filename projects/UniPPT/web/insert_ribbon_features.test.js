"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");

const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const source = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");

test("every visible Insert ribbon command has a stable interactive target", () => {
  for (const id of [
    "addText", "addImage", "captureScreen", "addTable", "addChart", "addSmartArt",
    "addFormula", "addSymbol", "addVideo", "addAudio",
  ]) {
    assert.match(html, new RegExp(`<button id="${id}"`), `${id} must be a real button`);
    assert.match(source, new RegExp(`\\$\\("#${id}"\\)\\.onclick\\s*=`), `${id} must be bound`);
  }
  assert.match(html, /id="videoInput"[^>]+accept="[^"]*\.mp4/);
  assert.match(html, /id="audioInput"[^>]+accept="[^"]*\.mp3/);
  assert.match(html, /id="imageInput"[^>]+accept="[^"]*\.svg[^"]*\.emf/);
});

test("insertions populate editable scene data instead of raster-only placeholders", () => {
  assert.match(source, /object\.table\s*=\s*\{[\s\S]*rows:[\s\S]*cells:/);
  assert.match(source, /const chart\s*=\s*\{[\s\S]*categories,[\s\S]*series,/);
  assert.match(source, /object\.chart\s*=\s*chart/);
  assert.match(source, /object\.media\s*=\s*\{[\s\S]*asset:\s*reader\.result/);
  assert.match(source, /editTableCell\(event, object\.id, rowIndex, columnIndex\)/);
  assert.match(source, /buildSmartArtObjects\(layout, labels\)/);
  assert.doesNotMatch(source, /SmartArt[^\n]{0,120}canvas\.toDataURL/);
});

test("new media keeps native bytes and browser playback bytes independently", () => {
  assert.match(source, /asset:\s*reader\.result,[\s\S]*mimeType:[\s\S]*playbackAsset:\s*reader\.result/);
  assert.match(source, /sourcePartName:\s*null/);
});

test("SVG and EMF insertion uses the lossless vector bridge", () => {
  assert.match(source, /fetch\("\/api\/vector\/emf-to-svg"/);
  assert.match(source, /data:image\/svg\+xml;base64/);
  assert.match(source, /原始 EMF 字节随投影无损绑定并可原样回写/);
  assert.match(source, /生成携带原始 SVG 的无损 EMF/);
});
