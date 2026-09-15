"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const source = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");

assert.doesNotMatch(source, /renderThumbnailObject/, "thumbnails must not use a lossy parallel renderer");
assert.match(
  source,
  /renderSlideObjects\(slide, scene, false, true, null\)/,
  "thumbnails must reuse the same inherited-layer renderer as the editor canvas",
);
assert.match(source, /renderSlideObjects\(slide, stage, true, false, mediaContext\)/);
assert.match(source, /renderReadOnlySlide\(slide, stage, mediaContext\)/);
assert.match(
  source,
  /UniPptPresentationScene;[\s\S]{0,500}runtime\.renderSlide\(slide, container/,
  "read-only presentation must use the same scene runtime as lossless HTML",
);
assert.match(
  source,
  /renderSlideObjects\(slide, container, false, false, mediaContext\)/,
  "the editor keeps a defensive fallback if a script is blocked",
);
assert.match(source, /textAlignLast\s*=\s*isDistributedAlign/, "distributed text needs last-line distribution");
assert.doesNotMatch(
  source,
  /\.scene-object\.selected[^}]*z-index\s*:/s,
  "selection must not change native PowerPoint stacking order",
);

console.log("render consistency tests passed");
