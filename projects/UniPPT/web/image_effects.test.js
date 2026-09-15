import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const app = readFileSync(new URL("./app.js", import.meta.url), "utf8");
assert.match(app, /applyImageEffects\(node, image, object\.imageEffects\)/);
assert.match(app, /feColorMatrix/);
assert.match(app, /feComponentTransfer/);
assert.match(app, /shadowColor/);
assert.match(app, /highlightColor/);
assert.match(app, /effects\?\.softEdgeRadius/);
assert.match(app, /maskComposite = "intersect"/);
assert.match(app, /webkitMaskComposite = "source-in"/);
