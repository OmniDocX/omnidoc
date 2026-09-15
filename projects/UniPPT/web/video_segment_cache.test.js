"use strict";

const assert = require("node:assert/strict");
const path = require("node:path");
const test = require("node:test");
const { pathToFileURL } = require("node:url");

const asset = (character) => character.repeat(64);
const profile = { name: "quality" };

function manifest() {
  const ids = { a: asset("a"), b: asset("b"), c: asset("c"), d: asset("d") };
  return {
    runtimeHash: "runtime-1",
    assetIndex: Object.values(ids).map((id) => ({ id, mimeType: "image/png" })),
    deck: {
      width: 1280,
      height: 720,
      fonts: [
        { family: "Page A", dataUri: `unippt-asset:${ids.a}` },
        { family: "Page B", dataUri: `unippt-asset:${ids.b}` },
        { family: "Unused", dataUri: `unippt-asset:${ids.d}` },
      ],
      extensions: {
        "org.unippt.dynamic": {
          objects: {
            dynamicA: { objectName: "Dynamic A", source: "<b>A</b>" },
            unused: { objectName: "Unused", source: "<b>unused</b>" },
          },
        },
      },
      slides: [
        { id: "s0", objects: [{ id: "dynamicA", name: "Dynamic A", textStyle: { fontFamily: "Page A, sans-serif" } }] },
        { id: "s1", objects: [{ id: "b", asset: `unippt-asset:${ids.b}`, textStyle: { fontFamily: "Page B" } }] },
        { id: "s2", objects: [{ id: "c", asset: `unippt-asset:${ids.c}`, textStyle: { fontFamily: "Arial" } }] },
      ],
    },
  };
}

test("video segment cache v6 only hashes the current/previous slide dependency closure and encoder", async () => {
  const moduleUrl = pathToFileURL(path.join(__dirname, "..", "tools", "html-video-renderer", "segment_cache.mjs")).href;
  const { VIDEO_SEGMENT_CACHE_SCHEMA, segmentCacheDependencies, segmentCacheKey } = await import(moduleUrl);
  assert.equal(VIDEO_SEGMENT_CACHE_SCHEMA, "unippt-video-segment-v6");

  const original = manifest();
  const dependency = segmentCacheDependencies(original, 0);
  assert.deepEqual(dependency.dynamic.map(([id]) => id), ["dynamicA"]);
  assert.deepEqual(dependency.fonts.map((font) => font.family), ["Page A"]);
  assert.deepEqual(dependency.assets.map((entry) => entry.id), [asset("a")]);

  const keys = original.deck.slides.map((_, index) => segmentCacheKey(original, index, profile, 1920, 1080, 30));
  const editedFirst = structuredClone(original);
  editedFirst.deck.slides[0].objects[0].name = "Dynamic A edited";
  const firstKeys = editedFirst.deck.slides.map((_, index) => segmentCacheKey(editedFirst, index, profile, 1920, 1080, 30));
  assert.notEqual(firstKeys[0], keys[0], "editing slide 0 invalidates its segment");
  assert.notEqual(firstKeys[1], keys[1], "editing slide 0 invalidates the next transition segment");
  assert.equal(firstKeys[2], keys[2], "editing slide 0 does not invalidate unrelated later segments");

  const unrelatedAsset = structuredClone(original);
  unrelatedAsset.assetIndex.find((entry) => entry.id === asset("d")).mimeType = "application/octet-stream";
  assert.equal(
    segmentCacheKey(unrelatedAsset, 2, profile, 1920, 1080, 30),
    keys[2],
    "an unreferenced asset cannot evict a cached segment",
  );

  const hardware = { ...profile, encoder: "h264_qsv" };
  assert.notEqual(
    segmentCacheKey(original, 0, hardware, 1920, 1080, 30),
    keys[0],
    "hardware and software segments cannot share an incompatible cache entry",
  );
});
