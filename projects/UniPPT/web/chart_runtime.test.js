"use strict";

const assert = require("node:assert/strict");
const chartRuntime = require("./chart_runtime.js");

const normalized = chartRuntime.normalize({
  chartType: "bar",
  categories: ["A"],
  series: [{ name: "Revenue", values: [4, "5", "missing"], color: "" }],
  barDirection: "bar",
  holeSize: 200,
});
assert.equal(normalized.chartType, "bar");
assert.equal(normalized.barDirection, "bar");
assert.deepEqual(normalized.categories, ["A", "2", "3"]);
assert.deepEqual(normalized.series[0].values, [4, 5, null]);
assert.equal(normalized.series[0].color, "#4472c4");
assert.equal(normalized.holeSize, 0.95);

assert.deepEqual(chartRuntime._test.bounds([
  { values: [-2, null, 7] },
  { values: [3] },
]), { min: -2, max: 7 });

const slices = chartRuntime._test.pieSlices([1, 2, null]);
assert.equal(slices.length, 3);
assert.equal(slices[0].total, 3);
assert.ok(Math.abs(slices[1].sweep - Math.PI * 4 / 3) < 1e-10);

assert.deepEqual(chartRuntime._test.splitSegments([1, 2, null, 3]), [
  [{ index: 0, value: 1 }, { index: 1, value: 2 }],
  [{ index: 3, value: 3 }],
]);
assert.match(chartRuntime._test.wedgePath(50, 50, 25, 0, Math.PI), /^M 50 50 L /);

console.log("chart_runtime tests passed");
