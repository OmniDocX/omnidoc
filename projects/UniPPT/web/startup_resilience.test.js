"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const serverSource = fs.readFileSync(
  path.join(__dirname, "..", "crates", "unippt-server", "src", "main.rs"),
  "utf8",
);

assert.match(appSource, /const STARTUP_FETCH_TIMEOUT_MS\s*=\s*10_000/,
  "startup must have a finite loading deadline");
assert.match(appSource, /fetch\("\/api\/demo",\s*\{[\s\S]*?signal:\s*controller\.signal/,
  "the initial deck request must be abortable");
assert.match(appSource, /finally\s*\{[\s\S]*?clearTimeout\(timeout\)/,
  "startup must always release its deadline timer");
assert.match(appSource, /function showStartupFailure[\s\S]*?重新加载/,
  "a failed startup must expose an explicit retry action");

assert.match(serverSource, /enum RequestLane\s*\{[\s\S]*?Interactive,[\s\S]*?Work/,
  "server must separate interactive and long-running work");
assert.match(serverSource, /Method::Get\s*\|\s*Method::Head[\s\S]*?RequestLane::Interactive/,
  "GET and HEAD requests must stay on the interactive lane");
assert.match(serverSource, /work_sender\.try_send\(request\)/,
  "a saturated work queue must not block the accept loop");

console.log("startup resilience tests passed");
