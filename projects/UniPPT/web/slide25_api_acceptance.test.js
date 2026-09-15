import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const pptxPath = process.env.UNIPPT_SLIDE25_PPTX;
const serverUrl = (process.env.UNIPPT_SERVER_URL || "http://127.0.0.1:8141").replace(/\/$/, "");

function flattenObjects(objects, result = []) {
  for (const object of objects || []) {
    result.push(object);
    flattenObjects(object.children, result);
  }
  return result;
}

function textRuns(object) {
  return (object?.textParagraphs || []).flatMap((paragraph) => paragraph.runs || []);
}

function normalizedHex(value) {
  return String(value || "").trim().toUpperCase();
}

function rgbFromHex(value) {
  const match = normalizedHex(value).match(/^#([0-9A-F]{2})([0-9A-F]{2})([0-9A-F]{2})$/);
  assert.ok(match, `expected an RGB hex color, received ${value}`);
  return match.slice(1).map((component) => Number.parseInt(component, 16));
}

test("9.pptx slide 25 preserves years, native gray numbers, and gradient title", {
  skip: pptxPath ? false : "set UNIPPT_SLIDE25_PPTX to run the local API acceptance test",
}, async (t) => {
  const response = await fetch(`${serverUrl}/api/import-pptx`, {
    method: "POST",
    headers: {
      "Content-Type": "application/vnd.openxmlformats-officedocument.presentationml.presentation",
      "X-UniPPT-Filename": encodeURIComponent("9.pptx"),
    },
    body: await readFile(pptxPath),
  });
  if (!response.ok) {
    assert.fail(`import failed with HTTP ${response.status}: ${await response.text()}`);
  }
  const deck = await response.json();
  assert.ok(deck.slides?.length >= 25, "the imported deck must contain slide 25");

  const objects = flattenObjects(deck.slides[24].objects);

  await t.test("all five four-digit years remain complete", () => {
    const expected = ["2012", "2015", "2017", "2018", "2019"];
    const actual = objects
      .map((object) => String(object.text || "").trim())
      .filter((value) => /^20\d{2}$/.test(value))
      .sort();
    assert.deepEqual(actual, expected);
    for (const year of expected) {
      const object = objects.find((candidate) => String(candidate.text || "").trim() === year);
      assert.equal(textRuns(object).map((run) => run.text).join(""), year);
    }
  });

  await t.test("Impact number markers resolve bg1 lumMod=50000 to neutral 50% gray", () => {
    const expected = ["30", "40", "50", "60", "70"];
    const markers = objects.filter((object) => expected.includes(String(object.text || "").trim()));
    assert.deepEqual(markers.map((object) => object.text.trim()).sort(), expected);
    for (const marker of markers) {
      const run = textRuns(marker).find((candidate) => String(candidate.text || "").trim());
      assert.equal(run?.nativeFontFamily, "Impact");
      const [red, green, blue] = rgbFromHex(run?.color);
      assert.ok(Math.max(red, green, blue) - Math.min(red, green, blue) <= 2,
        `${marker.text} must be neutral gray, received ${run?.color}`);
      assert.ok(red >= 120 && red <= 136,
        `${marker.text} must retain PowerPoint's 50% background luminance, received ${run?.color}`);
    }
  });

  await t.test("实现过程 retains the native two-stop text gradient", () => {
    const title = objects.find((object) => String(object.text || "").trim() === "实现过程");
    assert.ok(title, "slide 25 title must exist");
    const run = textRuns(title).find((candidate) => candidate.text === "实现过程");
    assert.ok(run?.gradient, "title run must retain a gradient fill");
    assert.ok(Math.abs(Number(run.gradient.angle) - 135) < 0.01);
    assert.deepEqual(run.gradient.stops.map((stop) => ({
      position: Number(stop.position),
      color: normalizedHex(stop.color),
      opacity: Number(stop.opacity),
    })), [
      { position: 0, color: "#FFE09F", opacity: 0.8 },
      { position: 0.93, color: "#FF779B", opacity: 0.8 },
    ]);
  });
});
