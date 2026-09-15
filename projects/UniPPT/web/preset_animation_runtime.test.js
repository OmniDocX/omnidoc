import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

await import("./preset_animation_runtime.js");

const runtime = globalThis.UniPptPresetAnimation;
assert.ok(runtime, "PowerPoint preset animation runtime must be installed");
assert.equal(runtime.frames({ effect: "fade" }), null, "fade keeps the browser's smooth opacity interpolation path");

const randomBars = runtime.frames({ effect: "randomBars", direction: "horizontal" });
assert.ok(randomBars.length > 2);
assert.equal(randomBars[0].opacity, 1, "random bars must reveal geometry rather than fade opacity");
assert.match(randomBars[0].maskImage, /linear-gradient/);
assert.notEqual(randomBars[0].maskSize, randomBars.at(-1).maskSize);
assert.equal(randomBars.at(-1).offset, 1);
assert.equal(randomBars.length, 17, "narrow bars should advance in enough steps to avoid a blinds-like reveal");
assert.equal(randomBars[0].maskImage.match(/linear-gradient\(/g)?.length, 32, "PowerPoint random bars use fine strips");
assert.match(randomBars[0].maskImage, /to bottom/);
const verticalRandomBars = runtime.frames({ effect: "randomBars", direction: "vertical" });
assert.match(verticalRandomBars[0].maskImage, /to right/);
assert.match(verticalRandomBars.at(-1).maskSize, /3\.975% 100%/);

const dissolve = runtime.frames({ effect: "dissolve" });
assert.ok(dissolve.length > 2);
assert.equal(dissolve[0].opacity, 1, "dissolve must use a deterministic cell mask");
assert.notEqual(dissolve[0].maskSize, dissolve.at(-1).maskSize);
assert.match(dissolve.at(-1).maskSize, /12\.75% 17%/);

const wheel = runtime.frames({ effect: "wheel", direction: "1" });
assert.ok(wheel.length > 2);
assert.equal(wheel[0].opacity, 1);
assert.equal(wheel[0].maskImage, "linear-gradient(transparent 0 0)");
assert.match(wheel[1].maskImage, /^repeating-conic-gradient\(from 0deg/);
assert.doesNotMatch(wheel[1].maskImage, /from -90deg/);
assert.notEqual(wheel[0].maskImage, wheel.at(-1).maskImage);
const wheelTwoSpokes = runtime.frames({ effect: "wheel", direction: "2" });
assert.match(wheelTwoSpokes[5].maskImage, /180\.000deg/);
assert.notEqual(wheelTwoSpokes[5].maskImage, wheel[5].maskImage);

const circleIn = runtime.frames({ effect: "circle", direction: "in" });
assert.ok(circleIn.length > 2);
assert.equal(circleIn[0].maskImage, "linear-gradient(transparent 0 0)");
assert.match(circleIn[1].maskImage, /^radial-gradient\(circle farthest-corner/);
assert.match(circleIn[1].maskImage, /93\.750%/);
assert.equal(circleIn.at(-1).maskImage, "linear-gradient(#000 0 0)");
const circleOut = runtime.frames({ effect: "circle", direction: "out" });
assert.match(circleOut[1].maskImage, /6\.250%/);
assert.notEqual(circleIn[1].maskImage, circleOut[1].maskImage);

const wipeRight = runtime.frames({ effect: "wipe", direction: "right" });
assert.equal(wipeRight.length, 15);
assert.equal(wipeRight[0].maskImage, "linear-gradient(transparent 0 0)");
assert.match(wipeRight[7].maskImage, /^linear-gradient\(to right/);
assert.match(runtime.frames({ effect: "wipe", direction: "left" })[7].maskImage, /to left/);
assert.match(runtime.frames({ effect: "wipe", direction: "up" })[7].maskImage, /to top/);
assert.match(runtime.frames({ effect: "wipe", direction: "down" })[7].maskImage, /to bottom/);
assert.equal(wipeRight.at(-1).maskImage, "linear-gradient(#000 0 0)");
assert.ok(wipeRight.every((frame) => frame.opacity === 1));

const split = runtime.frames({ effect: "split", direction: "outHorizontal" });
assert.equal(split[0].clipPath, "inset(50.000% 0 50.000% 0)");
assert.equal(split.at(-1).clipPath, "inset(0.000% 0 0.000% 0)");
assert.ok(split.every((frame) => frame.opacity === 1));

assert.equal(
  runtime.evaluateExpression("#ppt_y+.1", { "#ppt_y": 0.25 }),
  0.35,
  "native #ppt_* arithmetic must be evaluated without eval()",
);
assert.ok(Math.abs(runtime.evaluateExpression("#ppt_w*sin(2.5*pi*$)", { "#ppt_w": 0.2 }, 1) - 0.2) < 1e-9);

const targetNode = {
  style: { left: "100px", top: "50px", width: "200px", height: "100px" },
};
const nativeProperties = runtime.frames({
  effect: "flyIn",
  propertyAnimations: [
    {
      attributes: ["ppt_x"],
      keyframes: [
        { time: 0, value: "0-#ppt_w/2" },
        { time: 100000, value: "#ppt_x" },
      ],
    },
    {
      attributes: ["ppt_w"],
      keyframes: [
        { time: 0, value: "0" },
        { time: 100000, value: "#ppt_w" },
      ],
    },
    {
      attributes: ["style.rotation"],
      keyframes: [
        { time: 0, value: "360" },
        { time: 100000, value: "0" },
      ],
    },
  ],
}, { node: targetNode, slideWidth: 1280, slideHeight: 720, baseTransform: "rotate(5deg)" });
assert.equal(nativeProperties.length, 2);
assert.match(nativeProperties[0].transform, /translate\(-300px,0px\)/);
assert.match(nativeProperties[0].transform, /rotate\(360deg\)/);
assert.match(nativeProperties[0].transform, /scale\(0,1\)/);
assert.match(nativeProperties.at(-1).transform, /translate\(0px,0px\)/);
assert.match(nativeProperties.at(-1).transform, /rotate\(0deg\)/);
assert.match(nativeProperties.at(-1).transform, /scale\(1,1\)/);

targetNode.dataset = { id: "shape-42", base: "rotate(5deg)" };
targetNode.closest = () => ({ style: { width: "1280px", height: "720px" } });
globalThis.document = { querySelectorAll: () => [targetNode] };
const inferredNativeProperties = runtime.frames({
  effect: "flyIn",
  targetObjectId: "shape-42",
  propertyAnimations: [{
    attributes: ["ppt_y"],
    keyframes: [{ time: 0, value: "#ppt_y+.1" }, { time: 100000, value: "#ppt_y" }],
  }],
});
delete globalThis.document;
assert.match(inferredNativeProperties[0].transform, /translate\(0px,72px\)/,
  "the exported player can infer its stage context without a second renderer-specific API");

const formulaFrames = runtime.propertyAnimationFrames({
  propertyAnimations: [{
    attributes: ["ppt_w"],
    keyframes: [
      { time: 0, value: "0", formula: "#ppt_w*sin(2.5*pi*$)" },
      { time: 100000, value: "1" },
    ],
  }],
}, { node: targetNode, slideWidth: 1280, slideHeight: 720 });
assert.equal(formulaFrames.length, 17, "formula-driven p:anim must be sampled across its native interval");
assert.notEqual(formulaFrames[4].transform, formulaFrames[8].transform);

const fadeFloat = runtime.frames({
  effect: "floatIn",
  fadeFilter: "in",
  propertyAnimations: [{
    attributes: ["ppt_y"],
    keyframes: [{ time: 0, value: "#ppt_y+.1" }, { time: 100000, value: "#ppt_y" }],
  }],
}, { node: targetNode, slideWidth: 1280, slideHeight: 720, baseTransform: "" });
assert.equal(fadeFloat[0].opacity, 0, "companion fade must ramp property frames in from opacity 0");
assert.equal(fadeFloat.at(-1).opacity, 1, "companion fade must settle property frames at opacity 1");
assert.ok(fadeFloat[0].opacity < fadeFloat[1].opacity || fadeFloat.length === 2, "fade-in opacity must be monotonic");
assert.match(fadeFloat.at(-1).transform, /translate\(0px,0px\)/, "the fade ramp must not disturb the sampled trajectory");

const fadeOutFloat = runtime.frames({
  effect: "floatOut",
  fadeFilter: "out",
  class: "exit",
  propertyAnimations: [{
    attributes: ["ppt_y"],
    keyframes: [{ time: 0, value: "#ppt_y" }, { time: 100000, value: "#ppt_y+.1" }],
  }],
}, { node: targetNode, slideWidth: 1280, slideHeight: 720, baseTransform: "" });
assert.equal(fadeOutFloat[0].opacity, 1, "exit companion fade must start at opacity 1");
assert.equal(fadeOutFloat.at(-1).opacity, 0, "exit companion fade must ramp property frames out to opacity 0");

const noFadeProperties = runtime.frames({
  effect: "flyIn",
  propertyAnimations: [{
    attributes: ["ppt_y"],
    keyframes: [{ time: 0, value: "#ppt_y+.1" }, { time: 100000, value: "#ppt_y" }],
  }],
}, { node: targetNode, slideWidth: 1280, slideHeight: 720, baseTransform: "" });
assert.ok(noFadeProperties.every((frame) => frame.opacity === 1),
  "effects without a native fade filter keep the legacy full-opacity property frames");

const app = readFileSync(new URL("./app.js", import.meta.url), "utf8");
assert.match(app, /UniPptPresetAnimation\?\.frames\(animation, \{/);
assert.match(app, /propertyAnimations\?\.length/);
assert.match(app, /UniPptMotionPath\?\.frames/, "preset support must preserve motion-path playback");

const index = readFileSync(new URL("./index.html", import.meta.url), "utf8");
assert.match(index, /preset_animation_runtime\.js/);
for (const effect of ["randomBars", "dissolve", "wheel", "circle", "split", "wipe"]) {
  assert.match(index, new RegExp(`option value="${effect}"`));
}
