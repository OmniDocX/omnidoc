"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const runtimeSource = fs.readFileSync(path.join(__dirname, "document_cache.js"), "utf8");
const context = {
  console,
  ArrayBuffer,
  Blob,
  Intl,
};
context.globalThis = context;
vm.runInNewContext(runtimeSource, context);
const runtime = context.UniPptDocumentCache;

const media = `data:image/png;base64,${"A".repeat(1024 * 1024)}`;
const deck = {
  title: "9.pptx",
  width: 1280,
  height: 720,
  sourceImportId: "source-cache-9",
  slides: [{ id: "slide-1", objects: [{ id: "shape-1", asset: media }] }],
};
const cache = runtime.create();
runtime.reset(cache, deck, { sourceFile: { name: "9.pptx" }, sourceKind: "pptx" });
assert.equal(cache.sourceFile.name, "9.pptx");
assert.equal(cache.sourceFile.size, 0);
assert.equal(cache.sourceFile.lastModified, 0);

const structure = runtime.structure(cache, deck);
assert.deepEqual(Object.keys(structure).slice(0, 2), ["app", "unidoc_type"], "portable identity must be first and adjacent");
assert.equal(structure.app, "UniPPT");
assert.equal(structure.unidoc_type, "pptx");
assert.equal(structure.document.app, "UniPPT");
assert.equal(structure.document.unidoc_type, "pptx");
assert.equal(structure.document.deck.sourceImportId, null, "runtime cache handles must not leak into the portable document");
assert.equal(deck.sourceImportId, "source-cache-9", "building the structure must not mutate the live scene");
assert.equal(structure.document.deck.slides[0], deck.slides[0], "the instant viewer must share parsed nested scene data");
assert.equal(structure.manifest.assets, "assets/index.json");
assert.equal(structure.manifest.opcPackage, "pptx/package.json");
assert.equal(structure.manifest.presentation, undefined);
assert.ok(structure.manifest.features.includes("opc-parts-v1"));
assert.equal(structure.relationships.relationships[0].type, "native-opc-package");
assert.equal(structure.relationships.relationships[0].target, "pptx/package.json");
assert.equal(structure.relationships.relationships[1].type, "asset-index");
assert.equal(structure.assets.format, "unippt-asset-index");

const assetId = "a".repeat(64);
const cachedAsset = `/api/cache/source-cache-9/asset/${assetId}`;
const cachedDeck = {
  ...deck,
  slides: [{ id: "slide-1", objects: [{ id: "shape-1", asset: cachedAsset, style: { fill: `url(\"${cachedAsset}\")` } }] }],
};
const cachedStructure = runtime.structure(runtime.reset(runtime.create(), cachedDeck), cachedDeck);
assert.equal(cachedStructure.assets.uniqueAssets, 1);
assert.equal(cachedStructure.assets.references, 2);
assert.equal(cachedStructure.assets.entries[0].path, `blobs/sha256/${assetId}`);

const summary = runtime.stringSummary(media);
assert.match(summary, /data URI · image\/png/);
assert.match(summary, /MiB/);
assert.doesNotMatch(summary, /AAAAAA/, "large media summaries must not materialize their payload");

const firstBody = runtime.exportBody(cache, deck);
assert.equal(runtime.exportBody(cache, deck), firstBody, "UDOC/HTML/PPTX must reuse one serialized scene per revision");
const firstEnvelope = JSON.parse(firstBody);
assert.equal(firstEnvelope.cache_id, "source-cache-9");
assert.equal(firstEnvelope.cache_revision, 0);
assert.equal(firstEnvelope.deck, undefined, "an unchanged import must export by cache id without retransmitting the scene");

deck.title = "9-edited";
runtime.markChanged(cache);
const secondBody = runtime.exportBody(cache, deck);
assert.notEqual(secondBody, firstBody);
assert.equal(JSON.parse(secondBody).cache_revision, 1);
assert.equal(JSON.parse(secondBody).deck.slides[0].objects[0].asset.length, media.length);
assert.equal(runtime.structure(cache, deck).manifest.basename, "9-edited");
assert.equal(runtime.markSynced(cache, 0), false, "a stale response must not sync the current revision");
assert.equal(runtime.markSynced(cache, 1), true);
const thirdBody = runtime.exportBody(cache, deck);
assert.equal(JSON.parse(thirdBody).cache_revision, 1);
assert.equal(JSON.parse(thirdBody).deck, undefined, "a synced edited revision must reuse the server cache");
runtime.markChanged(cache);
assert.ok(JSON.parse(runtime.exportBody(cache, deck)).deck, "the next edit must sync exactly once again");
assert.equal(runtime.markSynced(cache, 1), false, "a delayed r1 response must not mark r2 as synced");
assert.equal(cache.syncedRevision, 1);

const history = runtime.cloneForHistory(deck);
assert.notEqual(history, deck);
assert.notEqual(history.slides, deck.slides);
assert.notEqual(history.slides[0].objects[0], deck.slides[0].objects[0]);
history.slides[0].objects[0].id = "history-only";
assert.equal(deck.slides[0].objects[0].id, "shape-1");

const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const app = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const style = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");
assert.match(html, /\.udoc 块结构（实时）/);
assert.match(html, /app: UniPPT[\s\S]*unidoc_type: pptx/);
assert.match(html, /document_cache\.js[\s\S]*app\.js/, "cache runtime must load before the editor");
assert.doesNotMatch(app, /fetch\("\/api\/udoc-json"/, "opening the structure viewer must be local and instantaneous");
assert.match(app, /body:\s*exportRequest\.body/g, "all server exports must reuse one revision-bound payload");
assert.match(app, /markExportRevisionSynced\(response, exportRequest\.revision\)/g, "successful exports must only sync the revision that was sent");
assert.match(app, /responseRevision === sentRevision[\s\S]*cache\.revision === sentRevision/, "delayed responses must not sync newer edits");
assert.match(app, /api\\\/cache\\\/\[\^\/\]\+\\\/asset/, "server-cached embedded font URLs must remain installable");
assert.match(app, /function mutate[\s\S]*markDeckChanged\(\)/, "edits must invalidate the real-time structure and export cache");
assert.doesNotMatch(app, /state\.history\.push\(JSON\.stringify/, "undo history must not repeatedly serialize base64 media");
assert.match(style, /udoc-tree-large-chunk/);

console.log("document cache and real-time UDOC structure tests passed");
