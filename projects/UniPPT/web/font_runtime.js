(() => {
  "use strict";

  const HASH_256 = /^[a-f0-9]{64}$/;
  const HASH_MD5 = /^[a-f0-9]{32}$/;
  const MD5_SHIFTS = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
  ];
  const MD5_CONSTANTS = Array.from({ length: 64 }, (_, index) =>
    Math.floor(Math.abs(Math.sin(index + 1)) * 0x100000000) >>> 0);
  const state = {
    localQueryPromise: null,
    localQueryState: "idle",
    localFonts: [],
    installedFaces: [],
    requestEpoch: 0,
  };

  function deps() {
    return globalThis.__UNIPPT_FONT_RUNTIME_MOCKS__ || {};
  }

  function normalizeName(value) {
    return String(value || "").trim().replace(/^["']|["']$/g, "")
      .replace(/\s+/g, " ").toLocaleLowerCase();
  }

  function normalizeHash(value, pattern) {
    const hash = String(value || "").trim().toLowerCase();
    return pattern.test(hash) ? hash : "";
  }

  function asBytes(data) {
    if (data instanceof ArrayBuffer) return new Uint8Array(data);
    if (ArrayBuffer.isView(data)) return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    return null;
  }

  // MD5 is only the secondary byte-version identity used by the shared
  // UniDoc protocol. SHA-256 below remains the mandatory integrity boundary.
  async function md5(data) {
    if (typeof deps().digestMd5 === "function") {
      return normalizeHash(await deps().digestMd5(data), HASH_MD5);
    }
    let source = asBytes(data);
    if (!source && data?.arrayBuffer) source = new Uint8Array(await data.arrayBuffer());
    if (!source) return "";
    const length = source.byteLength;
    const paddedLength = Math.ceil((length + 9) / 64) * 64;
    const padded = new Uint8Array(paddedLength);
    padded.set(source);
    padded[length] = 0x80;
    const view = new DataView(padded.buffer);
    view.setUint32(paddedLength - 8, (length << 3) >>> 0, true);
    view.setUint32(paddedLength - 4, Math.floor(length / 0x20000000) >>> 0, true);
    const words = new Uint32Array(16);
    let a0 = 0x67452301, b0 = 0xefcdab89, c0 = 0x98badcfe, d0 = 0x10325476;
    let sliceStarted = Date.now();
    for (let offset = 0; offset < paddedLength; offset += 64) {
      for (let index = 0; index < 16; index++) words[index] = view.getUint32(offset + index * 4, true);
      let a = a0, b = b0, c = c0, d = d0;
      for (let index = 0; index < 64; index++) {
        let f, wordIndex;
        if (index < 16) { f = (b & c) | (~b & d); wordIndex = index; }
        else if (index < 32) { f = (d & b) | (~d & c); wordIndex = (5 * index + 1) % 16; }
        else if (index < 48) { f = b ^ c ^ d; wordIndex = (3 * index + 5) % 16; }
        else { f = c ^ (b | ~d); wordIndex = (7 * index) % 16; }
        const sum = (a + f + MD5_CONSTANTS[index] + words[wordIndex]) >>> 0;
        const shift = MD5_SHIFTS[index];
        const previousD = d;
        d = c;
        c = b;
        b = (b + ((sum << shift) | (sum >>> (32 - shift)))) >>> 0;
        a = previousD;
      }
      a0 = (a0 + a) >>> 0;
      b0 = (b0 + b) >>> 0;
      c0 = (c0 + c) >>> 0;
      d0 = (d0 + d) >>> 0;
      if (offset && offset % (64 * 4096) === 0 && Date.now() - sliceStarted >= 12) {
        await new Promise((resolve) => setTimeout(resolve, 0));
        sliceStarted = Date.now();
      }
    }
    return [a0, b0, c0, d0].map((word) => [0, 8, 16, 24]
      .map((shift) => ((word >>> shift) & 0xff).toString(16).padStart(2, "0")).join("")).join("");
  }

  async function sha256(data) {
    if (typeof deps().digestSha256 === "function") {
      return normalizeHash(await deps().digestSha256(data), HASH_256);
    }
    const subtle = deps().cryptoSubtle || globalThis.crypto?.subtle;
    let bytes = asBytes(data);
    if (!bytes && data?.arrayBuffer) bytes = new Uint8Array(await data.arrayBuffer());
    if (!bytes || !subtle?.digest) return "";
    const input = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
    const digest = await subtle.digest("SHA-256", input);
    return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
  }

  function isSecureContextForFonts() {
    if (typeof deps().secureContext === "boolean") return deps().secureContext;
    return globalThis.isSecureContext === true;
  }

  function hasTransientActivation() {
    if (typeof deps().hasTransientActivation === "boolean") return deps().hasTransientActivation;
    return !globalThis.navigator?.userActivation || globalThis.navigator.userActivation.isActive === true;
  }

  // This must be invoked synchronously by the Open command. Local font names,
  // handles, blobs and computed hashes never enter a request or public state.
  function authorizeLocalFonts() {
    const query = deps().queryLocalFonts || globalThis.queryLocalFonts;
    if (!isSecureContextForFonts() || typeof query !== "function") return Promise.resolve(false);
    if (state.localQueryPromise) return state.localQueryPromise;
    if (!hasTransientActivation()) {
      state.localQueryState = "needs-gesture";
      return Promise.resolve(false);
    }
    state.localQueryState = "pending";
    let result;
    try { result = query.call(globalThis); }
    catch (error) { result = Promise.reject(error); }
    state.localQueryPromise = Promise.resolve(result).then((records) => {
      state.localFonts = Array.from(records || []);
      state.localQueryState = "granted";
      return true;
    }).catch((error) => {
      state.localFonts = [];
      if (error?.name === "SecurityError") {
        state.localQueryState = "needs-gesture";
        state.localQueryPromise = null;
      } else {
        state.localQueryState = "denied";
      }
      return false;
    });
    return state.localQueryPromise;
  }

  function trustedFontSource(value) {
    const source = String(value || "");
    if (/^data:(?:font\/|application\/)/i.test(source)) return source;
    if (/^\/api\/cache\/pptx-[a-f0-9]{64}\/asset\/[a-f0-9]{64}$/i.test(source)) return source;
    return "";
  }

  function verifiedNetworkUrl(source, expectedSha256) {
    if (source.startsWith("data:")) return source;
    const base = deps().baseUrl || globalThis.location?.href || "http://127.0.0.1/";
    const url = new URL(source, base);
    url.searchParams.set("fontSha256", expectedSha256);
    return url.href;
  }

  function normalizeFace(font) {
    const sha = normalizeHash(font?.sha256, HASH_256);
    const source = trustedFontSource(font?.dataUri);
    const format = String(font?.format || "truetype").toLowerCase();
    const mime = String(font?.mimeType || "font/ttf").toLowerCase();
    if (!font?.family || !sha || !source || format === "embedded-opentype"
      || format === "collection" || mime === "font/collection") return null;
    const names = [font.family, font.postscriptName].map(normalizeName).filter(Boolean);
    return {
      family: String(font.family),
      names: new Set(names),
      weight: String(Math.max(1, Math.min(1000, Number(font.weight) || 400))),
      style: font.style === "italic" ? "italic" : "normal",
      source,
      sha256: sha,
      md5: normalizeHash(font.md5, HASH_MD5),
      bytes: Math.max(0, Number(font.bytes) || 0),
    };
  }

  function requestedFamilyKeys(deck) {
    const collected = globalThis.UniPptFonts?.collectDeckFontFamilies?.(deck) || [];
    return new Set(collected.map(normalizeName).filter(Boolean));
  }

  function groupFaces(deck) {
    const requested = requestedFamilyKeys(deck);
    const groups = new Map();
    for (const font of deck?.fonts || []) {
      const face = normalizeFace(font);
      if (!face || !requested.has(normalizeName(face.family))) continue;
      let group = groups.get(face.sha256);
      if (!group) {
        group = { sha256: face.sha256, md5: face.md5, source: face.source,
          bytes: face.bytes, names: new Set(), faces: [], invalid: false };
        groups.set(face.sha256, group);
      } else if ((group.md5 && face.md5 && group.md5 !== face.md5)
        || group.source !== face.source || (group.bytes && face.bytes && group.bytes !== face.bytes)) {
        group.invalid = true;
      }
      for (const name of face.names) group.names.add(name);
      group.faces.push(face);
    }
    return groups;
  }

  function localRecordNames(record) {
    return [record?.family, record?.fullName, record?.postscriptName]
      .map(normalizeName).filter(Boolean);
  }

  async function installGroup(group, bytes, epoch) {
    if (epoch !== state.requestEpoch || group.invalid) return 0;
    const FontFaceCtor = deps().FontFace || globalThis.FontFace;
    const fontSet = deps().fontSet || globalThis.document?.fonts;
    if (typeof FontFaceCtor !== "function" || !fontSet?.add) return 0;
    let installed = 0;
    for (const face of group.faces) {
      try {
        const copy = bytes.slice ? bytes.slice(0) : bytes;
        const loaded = await new FontFaceCtor(face.family, copy, {
          weight: face.weight, style: face.style, display: "swap",
        }).load();
        if (epoch !== state.requestEpoch) return installed;
        fontSet.add(loaded);
        state.installedFaces.push(loaded);
        installed++;
      } catch (_) { /* the verified network/system fallback remains available */ }
    }
    return installed;
  }

  async function installLocalGroups(groups, epoch, resolved) {
    if (state.localQueryPromise) await state.localQueryPromise;
    if (epoch !== state.requestEpoch || !state.localFonts.length) return 0;
    let installed = 0;
    const candidates = state.localFonts.filter((record) => {
      const names = localRecordNames(record);
      return [...groups.values()].some((group) => names.some((name) => group.names.has(name)));
    });
    for (const record of candidates) {
      if (epoch !== state.requestEpoch || typeof record?.blob !== "function") break;
      try {
        const bytes = await (await record.blob()).arrayBuffer();
        const hash = await sha256(bytes);
        const group = groups.get(hash);
        if (!group || resolved.has(hash) || group.invalid) continue;
        if (group.bytes && bytes.byteLength !== group.bytes) continue;
        if (group.md5 && await md5(bytes) !== group.md5) continue;
        installed += await installGroup(group, bytes, epoch);
        resolved.add(hash);
      } catch (_) { /* unreadable or mismatched local face falls back to GET */ }
    }
    return installed;
  }

  async function installNetworkGroups(groups, epoch, resolved) {
    const fetchFn = deps().fetch || globalThis.fetch;
    if (typeof fetchFn !== "function") return 0;
    const counts = await Promise.all([...groups.values()].map(async (group) => {
      if (epoch !== state.requestEpoch || resolved.has(group.sha256) || group.invalid) return 0;
      try {
        const response = await fetchFn(verifiedNetworkUrl(group.source, group.sha256), {
          method: "GET", credentials: "same-origin", cache: "force-cache",
        });
        if (!response?.ok) throw new Error(`HTTP ${response?.status || 0}`);
        const bytes = await response.arrayBuffer();
        if (group.bytes && bytes.byteLength !== group.bytes) throw new Error("字体长度校验失败");
        if (await sha256(bytes) !== group.sha256) throw new Error("SHA-256 校验失败");
        if (group.md5 && await md5(bytes) !== group.md5) throw new Error("MD5 校验失败");
        const count = await installGroup(group, bytes, epoch);
        resolved.add(group.sha256);
        return count;
      } catch (error) {
        console.warn("[fonts] verified fallback failed:", error?.message || error);
        return 0;
      }
    }));
    return counts.reduce((sum, value) => sum + value, 0);
  }

  function clearInstalledFaces() {
    const fontSet = deps().fontSet || globalThis.document?.fonts;
    for (const face of state.installedFaces) {
      try { fontSet?.delete?.(face); } catch (_) { /* best effort */ }
    }
    state.installedFaces = [];
  }

  function cancel() {
    state.requestEpoch++;
    clearInstalledFaces();
  }

  async function prepareDeckFonts(deck) {
    cancel();
    const epoch = state.requestEpoch;
    const groups = groupFaces(deck);
    if (!groups.size) return { matchedFiles: 0, installed: 0, localInstalled: 0, networkInstalled: 0 };
    const resolved = new Set();
    const localInstalled = await installLocalGroups(groups, epoch, resolved);
    const networkInstalled = await installNetworkGroups(groups, epoch, resolved);
    const report = { matchedFiles: groups.size, installed: localInstalled + networkInstalled,
      localInstalled, networkInstalled };
    if (epoch === state.requestEpoch && typeof globalThis.CustomEvent === "function") {
      globalThis.dispatchEvent?.(new globalThis.CustomEvent("unippt:verified-fonts-ready", { detail: report }));
    }
    return report;
  }

  const api = Object.freeze({ authorizeLocalFonts, prepareDeckFonts, cancel });
  globalThis.UniPptFontRuntime = api;
  globalThis.__UniPptFontRuntimeTest = Object.freeze({
    md5, sha256, normalizeFace, groupFaces,
    reset() {
      cancel();
      state.localQueryPromise = null;
      state.localQueryState = "idle";
      state.localFonts = [];
    },
    state() {
      return { localPermission: state.localQueryState, localRecordCount: state.localFonts.length,
        installedFaceCount: state.installedFaces.length };
    },
  });
  if (typeof module === "object" && module.exports) module.exports = api;
})();
