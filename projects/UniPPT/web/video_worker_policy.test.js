"use strict";

const assert = require("node:assert/strict");
const path = require("node:path");
const test = require("node:test");
const { pathToFileURL } = require("node:url");

test("video workers follow available CPU parallelism with a measured cap and explicit override", async () => {
  const moduleUrl = pathToFileURL(path.join(
    __dirname, "..", "tools", "html-video-renderer", "worker_policy.mjs",
  )).href;
  const { MAX_RENDER_WORKERS, renderWorkerCount } = await import(moduleUrl);
  assert.equal(MAX_RENDER_WORKERS, 8);
  assert.equal(renderWorkerCount(24, undefined, 8), 8);
  assert.equal(renderWorkerCount(24, undefined, 4), 4);
  assert.equal(renderWorkerCount(3, undefined, 8), 3);
  assert.equal(renderWorkerCount(24, "3", 8), 3);
  assert.equal(renderWorkerCount(24, "99", 8), 8);
  assert.equal(renderWorkerCount(24, "0", 2), 2);
});
