"use strict";

// Deterministic offline entry point for a reviewed image inventory. Uses the
// same compiler and ChangeSet normalisation as the browser; no model or key.
const fs = require("node:fs");
const path = require("node:path");
require("../web/presentation_host.js");

async function main() {
  const [blueprintPath, outputPath, baseUrl = "http://127.0.0.1:8141"] = process.argv.slice(2);
  if (!blueprintPath || !outputPath) throw new Error("Usage: node tools/compile_image_blueprint.cjs blueprint.json output.pptx [local-server-url]");
  const url = new URL(baseUrl);
  if (!["127.0.0.1", "localhost", "[::1]"].includes(url.hostname)) throw new Error("Only a local UniPPT server is supported");
  if (fs.existsSync(outputPath)) throw new Error("Output exists; choose a new filename");
  const blueprint = JSON.parse(fs.readFileSync(blueprintPath, "utf8"));
  const imagePath = path.resolve(path.dirname(blueprintPath), blueprint.imagePath);
  const imageBytes = fs.readFileSync(imagePath);
  const mime = /\.jpe?g$/i.test(imagePath) ? "image/jpeg" : "image/png";
  const sourceSize = blueprint.sourceSize;
  const deck = { format: "unippt", version: 1, title: blueprint.slide.name, width: sourceSize.width, height: sourceSize.height,
    sourceWidthEmu: Math.round(sourceSize.width * 9525), sourceHeightEmu: Math.round(sourceSize.height * 9525), sourceImportId: null, fonts: [], slides: [] };
  const compiled = globalThis.UniPptPresentationHost.compileNativeImageSlide({ ...blueprint, sourceImage: `data:${mime};base64,${imageBytes.toString("base64")}` }, deck);
  const result = globalThis.UniPptPresentationHost.applyChangeSet(deck, { operations: [{ op: "insertSlide", slide: compiled.slide }] });
  const response = await fetch(new URL("/api/export-pptx", url), { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ deck: result.deck }) });
  if (!response.ok) throw new Error(`Export ${response.status}: ${(await response.text()).slice(0, 1000)}`);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, Buffer.from(await response.arrayBuffer()), { flag: "wx" });
  process.stdout.write(JSON.stringify({ outputPath, ...compiled.report }) + "\n");
}
main().catch((error) => { process.stderr.write(error.stack + "\n"); process.exitCode = 1; });
