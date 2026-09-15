"use strict";

(function installDocumentCache(global) {
  const FORMAT = "udoc";
  const UNIDOC_TYPE = "pptx";
  const APP = "UniPPT";

  function create() {
    return {
      revision: 0,
      cacheId: null,
      cacheOnlyEligible: false,
      syncedRevision: -1,
      sourceFile: null,
      sourceKind: "scene",
      exportBody: null,
      exportBodyRevision: -1,
      structure: null,
      structureRevision: -1,
    };
  }

  function cacheIdFromDeck(deck) {
    return deck?.cache_id ?? deck?.cacheId ?? deck?.sourceImportId ?? null;
  }

  function reset(cache, deck, options = {}) {
    cache.revision = 0;
    cache.cacheId = options.cacheId ?? cacheIdFromDeck(deck);
    cache.cacheOnlyEligible = options.cacheOnlyEligible !== false && Boolean(cache.cacheId);
    cache.syncedRevision = cache.cacheOnlyEligible ? 0 : -1;
    cache.sourceFile = sourceFileMetadata(options.sourceFile);
    cache.sourceKind = options.sourceKind || "scene";
    cache.exportBody = null;
    cache.exportBodyRevision = -1;
    cache.structure = null;
    cache.structureRevision = -1;
    // The structure shell is cheap and keeps the inspector instantaneous. It
    // deliberately shares the already parsed scene's nested values.
    structure(cache, deck);
    return cache;
  }

  function markChanged(cache) {
    cache.revision += 1;
    cache.exportBody = null;
    cache.exportBodyRevision = -1;
    cache.structure = null;
    cache.structureRevision = -1;
    return cache.revision;
  }

  function exportBody(cache, deck) {
    if (cache.exportBodyRevision !== cache.revision || !cache.exportBody) {
      // Existing servers ignore the two forward-compatible fields. A future
      // server can use cache_id directly without changing this client format.
      cache.exportBody = cache.cacheOnlyEligible && cache.cacheId && cache.syncedRevision === cache.revision
        ? JSON.stringify({ cache_id: cache.cacheId, cache_revision: cache.revision })
        : JSON.stringify({
          deck: global.UniPptPresentationHost?.prepareNativeExport?.(deck) || deck,
          cache_id: cache.cacheId,
          cache_revision: cache.revision,
        });
      cache.exportBodyRevision = cache.revision;
    }
    return cache.exportBody;
  }

  function markSynced(cache, revision = cache?.revision) {
    if (!cache?.cacheId || !Number.isInteger(revision) || cache.revision !== revision) return false;
    cache.cacheOnlyEligible = true;
    cache.syncedRevision = revision;
    cache.exportBody = null;
    cache.exportBodyRevision = -1;
    return true;
  }

  function structure(cache, deck) {
    if (cache.structureRevision === cache.revision && cache.structure) return cache.structure;
    const basename = safeBasename(deck?.title);
    const assetIndex = collectCachedAssets(deck);
    // Runtime handles never belong in a portable file. This shallow copy is
    // intentional: media and all nested scene data remain shared with the
    // parsed in-memory deck, so opening the viewer does not clone hundreds of
    // megabytes of base64 data.
    const portableDeck = { ...deck, sourceImportId: null };
    cache.structure = {
      app: APP,
      unidoc_type: UNIDOC_TYPE,
      format: FORMAT,
      version: 3,
      manifest: {
        format: "udoc-package",
        version: 3,
        unidoc_type: UNIDOC_TYPE,
        app: APP,
        basename,
        root: "document/document.json",
        opcPackage: "pptx/package.json",
        relationships: "rels/relationships.json",
        assets: "assets/index.json",
        features: ["unippt", "pptx", "animations", "omml", "lossless-html", "br", "content-addressed-blobs", "opc-parts-v1"],
      },
      document: {
        format: FORMAT,
        version: 3,
        unidoc_type: UNIDOC_TYPE,
        app: APP,
        deck: portableDeck,
      },
      relationships: {
        version: 1,
        relationships: [
          {
            source: "document/document.json",
            type: "native-opc-package",
            target: "pptx/package.json",
            mode: "internal",
          },
          {
            source: "document/document.json",
            type: "asset-index",
            target: "assets/index.json",
            mode: "internal",
          },
        ],
      },
      assets: assetIndex,
    };
    cache.structureRevision = cache.revision;
    return cache.structure;
  }

  function collectCachedAssets(deck) {
    const entries = new Map();
    const seen = new WeakSet();
    let references = 0;
    const visit = (value) => {
      if (typeof value === "string") {
        if (value.startsWith("data:") || !value.includes("/api/cache/")) return;
        const pattern = /\/api\/cache\/([^/"')\s]+)\/asset\/([a-f0-9]{64})/gi;
        for (const match of value.matchAll(pattern)) {
          references += 1;
          const [, cacheId, id] = match;
          entries.set(id, { id, path: `blobs/sha256/${id}`, cacheId });
        }
        return;
      }
      if (!value || typeof value !== "object" || seen.has(value)) return;
      seen.add(value);
      if (Array.isArray(value)) {
        for (const item of value) visit(item);
      } else {
        for (const item of Object.values(value)) visit(item);
      }
    };
    visit(deck);
    const sorted = [...entries.values()].sort((left, right) => left.id.localeCompare(right.id));
    return {
      format: "unippt-asset-index",
      version: 1,
      uniqueAssets: sorted.length,
      references,
      entries: sorted,
    };
  }

  function safeBasename(value) {
    return String(value || "unippt").replace(/[\\/:*?"<>|]+/g, "-").trim() || "unippt";
  }

  function sourceFileMetadata(file) {
    if (!file) return null;
    // Do not retain the uploaded Blob after the server has cached the native
    // package. Keeping only diagnostics avoids a second 20–100 MiB browser
    // copy while still letting the structure viewer describe the source.
    return {
      name: String(file.name || ""),
      size: Number(file.size) || 0,
      lastModified: Number(file.lastModified) || 0,
    };
  }

  function cloneForHistory(value, seen = new WeakMap()) {
    if (value === null || typeof value !== "object") return value;
    if (seen.has(value)) return seen.get(value);
    if (value instanceof Date) return new Date(value.getTime());
    if (value instanceof ArrayBuffer) return value.slice(0);
    if (ArrayBuffer.isView(value)) return value.slice?.() || value;
    if (typeof Blob !== "undefined" && value instanceof Blob) return value;
    const clone = Array.isArray(value) ? [] : {};
    seen.set(value, clone);
    for (const key of Object.keys(value)) clone[key] = cloneForHistory(value[key], seen);
    return clone;
  }

  function stringSummary(value) {
    const length = value.length;
    if (length <= 2048) return null;
    const data = /^data:([^;,]+)?(?:;[^,]*)?,/i.exec(value);
    const kind = data ? `data URI · ${data[1] || "binary"}` : "长字符串";
    return `${kind} · ${formatBytes(length)} · ${length.toLocaleString()} 字符`;
  }

  function formatBytes(bytes) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
    return `${(bytes / 1024 / 1024).toFixed(2)} MiB`;
  }

  global.UniPptDocumentCache = {
    APP,
    UNIDOC_TYPE,
    create,
    reset,
    markChanged,
    markSynced,
    exportBody,
    structure,
    cloneForHistory,
    stringSummary,
    formatBytes,
    cacheIdFromDeck,
    sourceFileMetadata,
  };
})(globalThis);
