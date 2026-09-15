"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const serverSource = fs.readFileSync(
  path.join(__dirname, "..", "crates", "unippt-server", "src", "main.rs"),
  "utf8",
);

function topLevelFunctionSource(name) {
  const declaration = appSource.indexOf(`function ${name}`);
  assert.ok(declaration >= 0, `${name} must be implemented as a testable named function`);
  // Both helpers are async, and the declaration index lands after that keyword.
  const start = appSource.lastIndexOf("async ", declaration) === declaration - "async ".length
    ? declaration - "async ".length
    : declaration;
  const end = appSource.indexOf("\nasync function ", declaration) < 0
    ? appSource.indexOf("\nfunction ", declaration)
    : Math.min(
      ...[appSource.indexOf("\nfunction ", declaration), appSource.indexOf("\nasync function ", declaration)]
        .filter((index) => index >= 0),
    );
  return appSource.slice(start, end < 0 ? appSource.length : end);
}

const context = {};
vm.runInNewContext(
  `${topLevelFunctionSource("readResponseError")}
   ${topLevelFunctionSource("readJsonResponse")}
   this.readResponseError = readResponseError;
   this.readJsonResponse = readJsonResponse;`,
  context,
);

/** Minimal stand-in for the one-shot body of a `fetch` response. */
function response({ ok = true, status = 200, statusText = "", body = "" } = {}) {
  return { ok, status, statusText, text: async () => body };
}

test("a rejected request without a body still names the failure", async () => {
  // A worker that abandons a request leaves the HTTP layer to answer with a
  // bare status and no payload. Feeding that to JSON.parse used to surface as
  // "Unexpected end of JSON input", which says nothing about what happened.
  const message = await context.readResponseError(
    response({ ok: false, status: 500, statusText: "Internal Server Error", body: "" }),
  );
  assert.match(message, /500/);
  assert.match(message, /Internal Server Error/);
  assert.doesNotMatch(message, /JSON/i, "the user must not be shown a parser error");
});

test("a structured server error is shown verbatim", async () => {
  const message = await context.readResponseError(
    response({ ok: false, status: 422, body: JSON.stringify({ error: "PPTX 解析失败: 缺少 EOCD" }) }),
  );
  assert.equal(message, "PPTX 解析失败: 缺少 EOCD");
});

test("a non-JSON error body is reported rather than swallowed", async () => {
  const message = await context.readResponseError(
    response({ ok: false, status: 502, body: "<html>Bad Gateway</html>" }),
  );
  assert.match(message, /Bad Gateway/);
});

test("a body that cannot even be read falls back to the status", async () => {
  const broken = {
    ok: false,
    status: 503,
    statusText: "Service Unavailable",
    text: async () => { throw new Error("stream closed"); },
  };
  const message = await context.readResponseError(broken);
  assert.match(message, /503/);
});

test("successful payloads parse, and truncated ones report the transport", async () => {
  const parsed = await context.readJsonResponse(response({ body: '{"slides":[1,2]}' }));
  // The parse happens inside the vm realm, so compare values not identities.
  assert.equal(parsed.slides.join(","), "1,2");

  await assert.rejects(
    () => context.readJsonResponse(response({ status: 200, body: '{"slides":[1,' })),
    /无法解析|200/,
    "a truncated success body must blame the response, not the user's file",
  );
  await assert.rejects(
    () => context.readJsonResponse(response({ ok: false, status: 500, body: "" })),
    /500/,
  );
});

test("import and export paths route through the shared reader", () => {
  for (const marker of [
    /const payload = await readJsonResponse\(response\);/,
    /if \(!response\.ok\) throw new Error\(await readResponseError\(response\)\);/,
  ]) {
    assert.match(appSource, marker);
  }
  assert.doesNotMatch(
    appSource,
    /const payload = await response\.json\(\);\s*\n\s*if \(!response\.ok\)/,
    "no caller may parse a body before it knows the request succeeded",
  );
});

test("the server answers a failed body read instead of dropping the request", () => {
  // Returning io::Error from a handler abandons the Request, and the HTTP layer
  // then closes it with an empty 500 the browser cannot parse.
  assert.match(
    serverSource,
    /fn read_limited\(request: &mut Request, limit: u64\) -> Result<Vec<u8>, \(StatusCode, String\)>/,
    "body reads must carry a status the caller can respond with",
  );
  assert.doesNotMatch(
    serverSource,
    /\.take\(MAX_UPLOAD_BYTES \+ 1\)\s*\n\s*\.read_to_end\(&mut bytes\)\?;/,
    "the PPTX upload must not propagate a raw io error",
  );
  assert.doesNotMatch(
    serverSource,
    /\.read_to_end\(&mut body\)\?;/,
    "no request body may be read with the error-propagating operator",
  );
});
