#!/usr/bin/env node

import { spawn } from "node:child_process";
import { access, mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { performance } from "node:perf_hooks";

function parseArgs(argv) {
  const result = {};
  for (let index = 0; index < argv.length; index += 1) {
    if (!argv[index].startsWith("--")) continue;
    result[argv[index].slice(2)] = argv[index + 1];
    index += 1;
  }
  return result;
}

async function exists(value) {
  try { await access(value); return true; } catch { return false; }
}

function cloneSlide(source, index) {
  const slide = structuredClone(source);
  const suffix = `-worker-bench-${index + 1}`;
  const ids = new Map();
  const rewriteObject = (object) => {
    if (!object) return;
    const oldId = object.id;
    if (oldId) {
      object.id = `${oldId}${suffix}`;
      ids.set(oldId, object.id);
    }
    for (const child of object.children || []) rewriteObject(child);
  };
  for (const objects of [slide.masterObjects, slide.layoutObjects, slide.objects]) {
    for (const object of objects || []) rewriteObject(object);
  }
  for (const animation of [...(slide.inheritedAnimations || []), ...(slide.animations || [])]) {
    if (animation.id) animation.id = `${animation.id}${suffix}`;
    if (ids.has(animation.targetObjectId)) animation.targetObjectId = ids.get(animation.targetObjectId);
  }
  slide.id = `worker-bench-slide-${index + 1}`;
  slide.name = `Worker benchmark ${index + 1}`;
  slide.notes = `Synthetic worker benchmark slide ${index + 1}`;
  const page = (slide.objects || []).find((object) => /page|页码/i.test(object.name || ""));
  if (page) page.text = `${String(index + 1).padStart(2, "0")} / BENCH`;
  return slide;
}

async function createFixture(input, output, slideCount, server) {
  const html = await readFile(input, "utf8");
  const marker = '<script type="application/x-unippt+json" id="unippt-data">';
  const start = html.indexOf(marker);
  const end = start < 0 ? -1 : html.indexOf("</script>", start + marker.length);
  if (start < 0 || end < 0) throw new Error("UniPPT render payload marker is missing");
  const payload = JSON.parse(html.slice(start + marker.length, end));
  const source = payload.deck?.slides?.[0];
  if (!source) throw new Error("benchmark source has no slide");
  payload.deck.slides = Array.from({ length: slideCount }, (_, index) => cloneSlide(source, index));
  if (server) {
    const response = await fetch(new URL("/api/export-html", server), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ deck: payload.deck }),
    });
    if (!response.ok) throw new Error(`fixture export failed (${response.status}): ${await response.text()}`);
    await writeFile(output, Buffer.from(await response.arrayBuffer()));
    return;
  }
  const encoded = JSON.stringify(payload).replaceAll("</script", "<\\/script");
  await writeFile(output, `${html.slice(0, start + marker.length)}${encoded}${html.slice(end)}`);
}

function runRenderer(renderer, args, env) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [renderer, ...args], {
      env: { ...process.env, ...env },
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", reject);
    child.on("close", (code) => code === 0
      ? resolve({ stdout, stderr })
      : reject(new Error(`renderer exited ${code}: ${stderr.slice(-4000)}`)));
  });
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const here = path.dirname(fileURLToPath(import.meta.url));
  const input = path.resolve(args.input || ".tmp/autoplay-qa/demo-autoplay.html");
  const outputRoot = path.resolve(args.output || ".tmp/video-worker-benchmark");
  const slideCount = Math.max(8, Number(args.slides) || 24);
  const repetitions = Math.max(1, Number(args.repetitions) || 2);
  const workers = String(args.workers || "2,3,4,6,8").split(",")
    .map(Number).filter((value) => Number.isInteger(value) && value > 0);
  if (await exists(outputRoot)) throw new Error(`output must not exist for a cold benchmark: ${outputRoot}`);
  await mkdir(outputRoot, { recursive: true });
  const fixture = path.join(outputRoot, "worker-fixture.html");
  await createFixture(input, fixture, slideCount, args.server);
  const renderer = path.join(here, "render.mjs");
  const runs = [];
  for (let repetition = 0; repetition < repetitions; repetition += 1) {
    const order = repetition % 2 ? [...workers].reverse() : workers;
    for (const workerCount of order) {
      const label = `r${repetition + 1}-w${workerCount}`;
      const workspace = path.join(outputRoot, `workspace-${label}`);
      const cache = path.join(outputRoot, `cache-${label}`);
      const output = path.join(outputRoot, `${label}.mp4`);
      const started = performance.now();
      const result = await runRenderer(renderer, [
        "--format", "video", "--input", fixture, "--output", output,
        "--workspace", workspace, "--width", String(Number(args.width) || 960),
        "--height", String(Number(args.height) || 540), "--fps", String(Number(args.fps) || 15),
        "--profile", args.profile || "quality", "--muted", "true",
      ], {
        UNIPPT_RENDER_WORKERS: String(workerCount),
        UNIPPT_VIDEO_SEGMENT_CACHE: cache,
        ...(args.ffmpeg ? { UNIPPT_FFMPEG_PATH: path.resolve(args.ffmpeg) } : {}),
      });
      const seconds = (performance.now() - started) / 1000;
      const summary = result.stderr.trim().split(/\r?\n/).findLast((line) => line.startsWith("offline video rendered"));
      const run = { repetition: repetition + 1, workers: workerCount, seconds, summary };
      runs.push(run);
      process.stdout.write(`${JSON.stringify({ type: "run", ...run })}\n`);
    }
  }
  const summary = workers.map((workerCount) => {
    const seconds = runs.filter((run) => run.workers === workerCount).map((run) => run.seconds);
    return {
      workers: workerCount,
      samples: seconds,
      medianSeconds: median(seconds),
      meanSeconds: seconds.reduce((sum, value) => sum + value, 0) / seconds.length,
    };
  }).sort((left, right) => left.medianSeconds - right.medianSeconds);
  process.stdout.write(`${JSON.stringify({ type: "summary", slideCount, repetitions, summary })}\n`);
}

main().catch((error) => {
  process.stderr.write(`${error?.stack || error}\n`);
  process.exitCode = 1;
});
