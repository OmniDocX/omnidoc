"use strict";

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "font_runtime.js"), "utf8");

function bytes(text) {
  const value = Buffer.from(text, "utf8");
  return value.buffer.slice(value.byteOffset, value.byteOffset + value.byteLength);
}

function digest(value, algorithm) {
  return crypto.createHash(algorithm).update(Buffer.from(value)).digest("hex");
}

function font(family, value, sourceUrl = "/api/cache/pptx-" + "a".repeat(64) + "/asset/" + "b".repeat(64)) {
  return {
    family,
    weight: 400,
    style: "normal",
    dataUri: sourceUrl,
    mimeType: "font/ttf",
    format: "truetype",
    sha256: digest(value, "sha256"),
    md5: digest(value, "md5"),
    bytes: value.byteLength,
  };
}

function runtimeHarness() {
  const installed = [];
  const deleted = [];
  const requests = [];
  let localRecords = [];
  let networkBytes = bytes("unset");
  let queryCount = 0;

  class MockFontFace {
    constructor(family, value, descriptors) {
      this.family = family;
      this.value = value;
      this.descriptors = descriptors;
    }
    async load() { return this; }
  }

  const mocks = {
    secureContext: true,
    hasTransientActivation: true,
    queryLocalFonts() { queryCount++; return Promise.resolve(localRecords); },
    digestSha256: async (value) => digest(value, "sha256"),
    FontFace: MockFontFace,
    fontSet: {
      add(face) { installed.push(face); },
      delete(face) { deleted.push(face); return true; },
    },
    baseUrl: "https://127.0.0.1/",
    async fetch(url, options) {
      requests.push({ url: String(url), options: { ...options } });
      return { ok: true, status: 200, arrayBuffer: async () => networkBytes.slice(0) };
    },
  };
  const context = {
    __UNIPPT_FONT_RUNTIME_MOCKS__: mocks,
    UniPptFonts: { collectDeckFontFamilies: (deck) => deck.requestedFamilies || [] },
    console: { warn() {}, error() {}, log() {} },
    ArrayBuffer, Uint8Array, Uint32Array, DataView, URL, Date, Math,
    Promise, setTimeout, clearTimeout,
  };
  vm.runInNewContext(source, context, { filename: "font_runtime.js" });
  return {
    api: context.UniPptFontRuntime,
    testApi: context.__UniPptFontRuntimeTest,
    mocks,
    installed,
    deleted,
    requests,
    setLocalRecords(value) { localRecords = value; },
    setNetworkBytes(value) { networkBytes = value; },
    queryCount() { return queryCount; },
  };
}

test("MD5 compatibility identity matches canonical vectors", async () => {
  const harness = runtimeHarness();
  assert.equal(await harness.testApi.md5(bytes("")), "d41d8cd98f00b204e9800998ecf8427e");
  assert.equal(await harness.testApi.md5(bytes("abc")), "900150983cd24fb0d6963f7d28e17f72");
  assert.equal(await harness.testApi.md5(bytes("1234567890".repeat(8))), "57edf4a22be3c955ac49da2e2107b67a");
});

test("an exact local SHA-256 and MD5 match installs without a font GET", async () => {
  const harness = runtimeHarness();
  const localBytes = bytes("exact-local-font");
  harness.setLocalRecords([{
    family: "Exact Face",
    fullName: "Exact Face",
    postscriptName: "ExactFace-Regular",
    blob: async () => new Blob([localBytes]),
  }]);
  assert.equal(await harness.api.authorizeLocalFonts(), true);
  const result = await harness.api.prepareDeckFonts({
    requestedFamilies: ["Exact Face"],
    fonts: [font("Exact Face", localBytes)],
  });
  assert.deepEqual({ local: result.localInstalled, network: result.networkInstalled }, { local: 1, network: 0 });
  assert.equal(harness.requests.length, 0);
  assert.equal(harness.installed[0].family, "Exact Face");
  assert.equal(harness.queryCount(), 1);
});

test("a same-name mismatched local face falls back to a verified content-addressed GET", async () => {
  const harness = runtimeHarness();
  const expected = bytes("published-font-v2");
  harness.setNetworkBytes(expected);
  harness.setLocalRecords([{
    family: "Fallback Face",
    fullName: "Fallback Face",
    blob: async () => new Blob([bytes("wrong-local-version")]),
  }]);
  await harness.api.authorizeLocalFonts();
  const result = await harness.api.prepareDeckFonts({
    requestedFamilies: ["Fallback Face"],
    fonts: [font("Fallback Face", expected)],
  });
  assert.equal(result.localInstalled, 0);
  assert.equal(result.networkInstalled, 1);
  assert.equal(harness.requests.length, 1);
  assert.match(harness.requests[0].url, new RegExp(`fontSha256=${digest(expected, "sha256")}`));
  assert.equal(harness.requests[0].options.method, "GET");
  assert.equal(Object.hasOwn(harness.requests[0].options, "body"), false);
});

test("corrupted fallback bytes and unused document faces are never installed", async () => {
  const harness = runtimeHarness();
  const expected = bytes("expected-font");
  harness.setNetworkBytes(bytes("tampered-font"));
  const result = await harness.api.prepareDeckFonts({
    requestedFamilies: ["Used Face"],
    fonts: [font("Used Face", expected), font("Unused Face", bytes("unused"))],
  });
  assert.equal(result.matchedFiles, 1);
  assert.equal(result.installed, 0);
  assert.equal(harness.installed.length, 0);
  assert.equal(harness.requests.length, 1);
  assert.deepEqual(Object.keys(harness.testApi.state()).sort(),
    ["installedFaceCount", "localPermission", "localRecordCount"].sort());
});

test("missing activation and SecurityError remain retryable", async () => {
  const harness = runtimeHarness();
  harness.mocks.hasTransientActivation = false;
  assert.equal(await harness.api.authorizeLocalFonts(), false);
  assert.equal(harness.queryCount(), 0);
  assert.equal(harness.testApi.state().localPermission, "needs-gesture");

  harness.mocks.hasTransientActivation = true;
  harness.mocks.queryLocalFonts = () => {
    throw Object.assign(new Error("activation required"), { name: "SecurityError" });
  };
  assert.equal(await harness.api.authorizeLocalFonts(), false);
  assert.equal(harness.testApi.state().localPermission, "needs-gesture");
  harness.mocks.queryLocalFonts = () => Promise.resolve([]);
  assert.equal(await harness.api.authorizeLocalFonts(), true);
});
