import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

await import("./motion_path_runtime.js");

const runtime = globalThis.UniPptMotionPath;
assert.ok(runtime, "motion-path runtime must be installed");

const paths = [
  [95, "M 2.08333E-6 4.44444E-6 L 0.87487 4.44444E-6 ", 0.87487],
  [60, "M 0.03073 4.07407E-6 L -0.08906 4.07407E-6 ", -0.08906],
  [60, "M 0.03073 4.07407E-6 L -2.08333E-6 4.07407E-6 ", -2.08333E-6],
  [61, "M 0.03073 3.33333E-6 L -0.08906 3.33333E-6 ", -0.08906],
  [61, "M 0.03073 3.33333E-6 L -2.08333E-6 3.33333E-6 ", -2.08333E-6],
  [62, "M 0.03073 -3.7037E-6 L -0.08906 -3.7037E-6 ", -0.08906],
  [62, "M 0.03073 -3.7037E-6 L -2.08333E-6 -3.7037E-6 ", -2.08333E-6],
];

for (const [shapeId, path, expectedEndX] of paths) {
  const points = runtime.parse(path);
  assert.ok(points?.length >= 2, `shape ${shapeId} path must parse`);
  assert.ok(Math.abs(points.at(-1).x - expectedEndX) < 1e-9, `shape ${shapeId} endpoint must be retained`);
  const frames = runtime.frames(path, 1280, 720, "rotate(0deg)");
  assert.equal(frames[0].offset, 0);
  assert.equal(frames.at(-1).offset, 1);
  const translatedX = Number(/translate\(([-+\d.eE]+)/.exec(frames.at(-1).transform)?.[1]);
  assert.ok(Math.abs(translatedX - expectedEndX * 1280) < 1e-9, `shape ${shapeId} uses layout-width distance`);
}

const curve = runtime.frames("M 0 0 C .1 0 .2 .2 .3 .2 E", 1280, 720, "");
assert.ok(curve?.length > 20, "cubic PowerPoint paths must be sampled smoothly");
assert.equal(runtime.frames("M nope", 1280, 720, ""), null);
assert.equal(runtime.frames("M 0 0 L Infinity 0", 1280, 720, ""), null);

const app = readFileSync(new URL("./app.js", import.meta.url), "utf8");
assert.match(app, /UniPptMotionPath\?\.frames/);
assert.match(app, /translate\(15%,0\)/, "unparseable paths retain a safe legacy fallback");
