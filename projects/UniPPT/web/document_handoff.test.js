"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

function runtime(hash = "") {
  const channels = new Map();
  const messages = [];
  class FakeBroadcastChannel {
    constructor(name) { this.name = name; this.closed = false; (channels.get(name) || channels.set(name, new Set()).get(name)).add(this); }
    postMessage(data) { messages.push(data); for (const peer of channels.get(this.name) || []) if (peer !== this && !peer.closed) queueMicrotask(() => peer.onmessage?.({ data })); }
    close() { this.closed = true; channels.get(this.name)?.delete(this); }
  }
  const location = { href: `http://127.0.0.1:8141/${hash}`, hash };
  const child = { opener: {}, closed: false, close() { this.closed = true; } };
  const context = vm.createContext({
    console, URL, URLSearchParams, BroadcastChannel: FakeBroadcastChannel,
    crypto: { randomUUID: () => "12345678-1234-4123-8123-123456789abc" },
    location, history: { state: null, replaceState(_state, _title, href) { location.href = href; location.hash = new URL(href).hash; } },
    open: () => child, setTimeout, clearTimeout, setInterval, clearInterval, queueMicrotask,
  });
  context.globalThis = context;
  vm.runInContext(fs.readFileSync(path.join(__dirname, "document_handoff.js"), "utf8"), context, { filename: "document_handoff.js" });
  return { api: context.UniPptDocumentHandoff, context, child, messages };
}

test("handoff tokens live only in a fragment and parse strictly", () => {
  const { api } = runtime("#unippt-handoff=12345678-1234-4123-8123-123456789abc");
  assert.equal(api.receiverToken(), "12345678-1234-4123-8123-123456789abc");
  assert.match(api.__test.receiverUrl("12345678-1234-4123-8123-123456789abc"), /#unippt-handoff=/);
  assert.doesNotMatch(api.__test.receiverUrl("12345678-1234-4123-8123-123456789abc"), /\?unippt-handoff=/);
});

test("parent resolves only after the receiver applies and accepts the validated deck", async () => {
  const { api, context } = runtime();
  const session = api.open({ timeoutMs: 2000 });
  context.location.hash = "#unippt-handoff=12345678-1234-4123-8123-123456789abc";
  context.location.href = `http://127.0.0.1:8141/${context.location.hash}`;
  const receiving = api.receive({ timeoutMs: 2000 });
  const deck = { format: "unippt", slides: [{ id: "slide-1", objects: [] }] };
  const publishing = session.publish(deck, { transactionId: "tx-1" });
  const received = await receiving;
  let parentResolved = false;
  void publishing.then(() => { parentResolved = true; });
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(parentResolved, false);
  assert.match(context.location.hash, /unippt-handoff=/);
  assert.equal(received.accept(), true);
  await publishing;
  assert.deepEqual(JSON.parse(JSON.stringify(received.deck)), deck);
  assert.equal(received.outcome.transactionId, "tx-1");
  assert.equal(context.location.hash, "");
});

test("receiver rejection keeps the parent operation failed instead of acknowledging it", async () => {
  const { api, context } = runtime();
  const session = api.open({ timeoutMs: 2000 });
  context.location.hash = "#unippt-handoff=12345678-1234-4123-8123-123456789abc";
  context.location.href = `http://127.0.0.1:8141/${context.location.hash}`;
  const receiving = api.receive({ timeoutMs: 2000 });
  const publishing = session.publish({ format: "unippt", slides: [{ id: "slide-1", objects: [] }] });
  const received = await receiving;
  assert.equal(received.reject(new Error("render failed")), true);
  await assert.rejects(publishing, /render failed/);
  assert.equal(context.location.hash, "");
});

test("receiver readiness transfers a large deck only once before final acceptance", async () => {
  const { api, context, messages } = runtime();
  const session = api.open({ timeoutMs: 2000 });
  context.location.hash = "#unippt-handoff=12345678-1234-4123-8123-123456789abc";
  context.location.href = `http://127.0.0.1:8141/${context.location.hash}`;
  const receiving = api.receive({ timeoutMs: 2000 });
  const publishing = session.publish({ format: "unippt", slides: [{ id: "slide-1", objects: [], payload: "x".repeat(1000) }] });
  const received = await receiving;
  await new Promise((resolve) => setTimeout(resolve, 350));
  assert.equal(messages.filter((entry) => entry.type === "deck").length, 1);
  received.accept();
  await publishing;
});

test("closing the child rejects promptly instead of waiting for the full timeout", async () => {
  const { api, child } = runtime();
  const session = api.open({ timeoutMs: 2000 });
  const publishing = session.publish({ format: "unippt", slides: [{ id: "slide-1", objects: [] }] });
  child.closed = true;
  await assert.rejects(publishing, /已关闭/);
});

test("blocked popup fails without creating a silent current-window fallback", () => {
  const { api, context } = runtime();
  context.open = () => null;
  assert.throws(() => api.open(), /阻止了新窗口/);
});
