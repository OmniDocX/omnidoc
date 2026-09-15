import { createHash } from "node:crypto";
import { copyFile, rename, unlink } from "node:fs/promises";
import { constants } from "node:fs";
import { randomUUID } from "node:crypto";

export const VIDEO_SEGMENT_CACHE_SCHEMA = "unippt-video-segment-v6";

// Production's private /tmp and persistent cache can be separate mounts.
// Publish via a destination-side temporary file so readers never see a partial
// segment, including when a direct rename fails with EXDEV.
export async function publishCachedSegment(source, destination, io = {copyFile, rename, unlink}) {
  try {
    await io.rename(source, destination);
  } catch (error) {
    if (error.code !== "EXDEV") throw error;
    const pending = `${destination}.${randomUUID()}.pending`;
    try {
      await io.copyFile(source, pending, constants.COPYFILE_EXCL);
      await io.rename(pending, destination);
      await io.unlink(source);
    } finally {
      await io.unlink(pending).catch(() => {});
    }
  }
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function visit(value, visitor, key = "", parentKey = "") {
  if (value == null) return;
  if (typeof value !== "object") {
    visitor(value, key, parentKey);
    return;
  }
  if (Array.isArray(value)) {
    for (const item of value) visit(item, visitor, key, parentKey);
    return;
  }
  for (const [childKey, child] of Object.entries(value)) {
    visit(child, visitor, childKey, key);
  }
}

function sceneObjects(slides) {
  const result = [];
  const append = (object) => {
    if (!object || typeof object !== "object") return;
    result.push(object);
    for (const child of object.children || []) append(child);
  };
  for (const slide of slides) {
    for (const collection of [slide?.masterObjects, slide?.layoutObjects, slide?.objects]) {
      for (const object of collection || []) append(object);
    }
  }
  return result;
}

function dynamicDependencies(deck, slides) {
  const entries = Object.entries(deck.extensions?.["org.unippt.dynamic"]?.objects || {});
  if (!entries.length) return [];
  const byId = new Map(entries);
  const selected = [];
  const seen = new Set();
  for (const object of sceneObjects(slides)) {
    let dependency = object.id && byId.has(object.id)
      ? [object.id, byId.get(object.id)]
      : entries.find(([, entry]) => entry?.objectName === object.name);
    if (!dependency || seen.has(dependency[0])) continue;
    seen.add(dependency[0]);
    selected.push(dependency);
  }
  return selected;
}

function fontDependencies(deck, values) {
  const references = new Set();
  visit(values, (value, key, parentKey) => {
    if (typeof value !== "string") return;
    if (/^(?:fontFamily|nativeFontFamily|typeface|fontFace|fontName)$/i.test(key)
      || (/^nativeFonts$/i.test(parentKey) && /^(?:latin|eastAsia|complexScript|symbol)$/i.test(key))) {
      references.add(value.toLocaleLowerCase());
    }
  });
  if (!references.size) return [];
  return (deck.fonts || []).filter((font) => {
    const family = String(font?.family || "").trim().toLocaleLowerCase();
    return family && [...references].some((reference) => reference.includes(family));
  });
}

function referencedAssets(manifest, values) {
  const ids = new Set();
  visit(values, (value) => {
    if (typeof value !== "string") return;
    for (const match of value.matchAll(/unippt-asset:([0-9a-f]{64})/gi)) ids.add(match[1].toLowerCase());
  });
  const assets = new Map((manifest.assetIndex || []).map((asset) => [String(asset.id || "").toLowerCase(), asset]));
  return [...ids].sort().map((id) => assets.get(id) || { id, missing: true });
}

export function segmentCacheDependencies(manifest, slideIndex) {
  const deck = manifest.deck;
  const previousSlide = slideIndex > 0 ? deck.slides[slideIndex - 1] : null;
  const slide = deck.slides[slideIndex];
  const slides = previousSlide ? [previousSlide, slide] : [slide];
  const dynamic = dynamicDependencies(deck, slides);
  const fonts = fontDependencies(deck, [slides, dynamic]);
  const assets = referencedAssets(manifest, [slides, dynamic, fonts]);
  return { previousSlide, slide, dynamic, fonts, assets };
}

export function segmentCacheKey(manifest, slideIndex, profile, width, height, fps) {
  const deck = manifest.deck;
  return sha256(JSON.stringify({
    schema: VIDEO_SEGMENT_CACHE_SCHEMA,
    runtimeHash: manifest.runtimeHash,
    profile: {
      name: profile.name,
      encoder: profile.encoder || "libx264",
      scale: profile.scale,
      jpegQuality: profile.jpegQuality,
      crf: profile.crf,
    },
    width,
    height,
    fps,
    deckWidth: deck.width,
    deckHeight: deck.height,
    ...segmentCacheDependencies(manifest, slideIndex),
  }));
}
