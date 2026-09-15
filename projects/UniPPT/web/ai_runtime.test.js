"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "ai_runtime.js"), "utf8");

function sseResponse(events, status = 200) {
  const bytes = new TextEncoder().encode(events.join("\n") + "\n");
  let sent = false;
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: { get: () => "text/event-stream; charset=utf-8" },
    body: { getReader: () => ({ read: async () => sent ? { done: true } : (sent = true, { done: false, value: bytes }) }) },
    text: async () => new TextDecoder().decode(bytes),
  };
}

function runtime(fetch) {
  const context = { console, fetch, TextDecoder, TextEncoder, Uint8Array, AbortController, setTimeout, clearTimeout };
  context.globalThis = context;
  vm.runInNewContext(source, context, { filename: "ai_runtime.js" });
  return context.UniPptAiRuntime;
}

test("U AI streams real model tokens instead of synthesizing local canned answers", async () => {
  const seen = [];
  const thoughts = [];
  const ai = runtime(async () => sseResponse([
    `data: ${JSON.stringify({ choices: [{ delta: { reasoning_content: "检查" } }] })}`,
    `data: ${JSON.stringify({ choices: [{ delta: { reasoning_content: "上下文" } }] })}`,
    `data: ${JSON.stringify({ choices: [{ delta: { content: "你好，我是已连接的模型。" } }] })}`,
    "data: [DONE]",
  ]));
  const result = await ai.run({
    config: { base: "https://example.test/v1", model: "test", key: "key" },
    messages: [{ role: "user", content: "你好" }],
    tools: [],
    onThinking: (token) => thoughts.push(token),
    onToken: (token) => seen.push(token),
  });
  assert.equal(result.text, "你好，我是已连接的模型。");
  assert.equal(thoughts.join(""), "检查上下文");
  assert.deepEqual(thoughts, ["检查", "上下文"]);
  assert.equal(seen.join(""), result.text);
});

test("task-specific thinking policy and timeout are respected", async () => {
  let body;
  const ai = runtime(async (_url, options) => { body = JSON.parse(options.body); return sseResponse(["data: [DONE]"]); });
  await ai.completion({ config: {}, messages: [], thinking: false, thinkingBudget: 2048 });
  assert.equal(body.enable_thinking, false);
  assert.equal(body.thinking_budget, 2048);
  const slow = runtime(async (_url, { signal }) => new Promise((_, reject) => signal.addEventListener("abort", () => reject(Object.assign(new Error("abort"), { name: "AbortError" })))));
  await assert.rejects(slow.completion({ config: {}, messages: [], timeoutMs: 10 }), /超过.*秒/);
});

test("schema validation rejects malformed operations before approval", () => {
  const ai = runtime(() => {});
  const host = require("node:vm").runInNewContext(fs.readFileSync(path.join(__dirname, "presentation_host.js"), "utf8") + "\nUniPptPresentationHost", { TextEncoder });
  const schema = host.toolDefinitions.find((t) => t.name === "presentation.applyChangeSet").inputSchema;
  for (const value of [{ operations: "[]" }, { operations: [{ type: "object.remove", objectId: "a" }] }, { operations: [{ op: "updateText", objectId: "a" }] }]) assert.throws(() => ai.validateArguments(value, schema));
  assert.doesNotThrow(() => ai.validateArguments({ operations: [{ op: "updateText", objectId: "a", text: "new" }] }, schema));
});

test("successful terminal reconstruction never enters a duplicate model round", async () => {
  let requests = 0, calls = 0;
  const ai = runtime(async () => { requests++; return sseResponse([`data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [{ index: 0, id: "native", function: { name: "slide.reconstructFromImage", arguments: "{}" } }] } }] })}`, "data: [DONE]"]); });
  const result = await ai.run({ config: {}, messages: [], tools: [{ name: "slide.reconstructFromImage", inputSchema: { type: "object" } }], terminalTools: ["slide.reconstructFromImage"], callTool: async () => { calls++; return { completionText: "待视觉复核" }; } });
  assert.equal(requests, 1); assert.equal(calls, 1); assert.equal(result.text, "待视觉复核");
});

test("native preflight runs before approval and a failed inventory cannot mutate", async () => {
  let approvals = 0, calls = 0;
  const ai = runtime(async () => sseResponse([`data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [{ index: 0, id: "native", function: { name: "native", arguments: "{}" } }] } }] })}`, "data: [DONE]"]));
  await assert.rejects(ai.run({ config: {}, messages: [], maxRounds: 1, tools: [{ name: "native", inputSchema: { type: "object" } }], preflightTool: () => { throw new Error("inventory missing"); }, beforeTools: () => { approvals++; }, callTool: () => { calls++; } }), /超过 1 轮/);
  assert.equal(approvals, 0); assert.equal(calls, 0);
});

test("an offline loopback service is reported as a local service failure", async () => {
  const ai = runtime(async () => { throw new TypeError("Failed to fetch"); });
  await assert.rejects(
    () => ai.run({ config: { base: "", model: "", key: "" }, messages: [{ role: "user", content: "你好" }] }),
    /本地 UniPPT 服务没有响应，请重新启动服务并刷新页面/,
  );
  const status = await ai.builtinStatus();
  assert.equal(status.unreachable, true);
  assert.match(status.error, /Failed to fetch/);
});

test("Qwen-compatible requests keep thinking enabled while streaming editor tool calls", async () => {
  let requestBody = null;
  const ai = runtime(async (_url, options) => {
    requestBody = JSON.parse(options.body);
    return sseResponse([`data: ${JSON.stringify({ choices: [{ delta: { content: "完成" } }] })}`, "data: [DONE]"]);
  });
  await ai.run({
    config: { base: "https://dashscope.aliyuncs.com/compatible-mode/v1", model: "qwen3.7-flash", key: "key" },
    messages: [{ role: "user", content: "重建图片" }],
    tools: [],
    maxTokens: 32768,
  });
  assert.equal(requestBody.enable_thinking, true);
  assert.equal(requestBody.max_tokens, 32768);
});

test("truncated and over-limit model responses fail before any document tool can run", async () => {
  const truncated = runtime(async () => sseResponse([
    `data: ${JSON.stringify({ choices: [{ delta: { content: "不完整" }, finish_reason: "length" }] })}`,
    "data: [DONE]",
  ]));
  await assert.rejects(
    truncated.run({ config: { base: "https://example.test/v1", model: "test", key: "key" }, messages: [], tools: [] }),
    /达到输出上限.*当前文稿未改变/,
  );

  const missingDone = runtime(async () => sseResponse([
    `data: ${JSON.stringify({ choices: [{ delta: { content: "半截响应" } }] })}`,
  ]));
  await assert.rejects(
    missingDone.run({ config: { base: "https://example.test/v1", model: "test", key: "key" }, messages: [], tools: [] }),
    /未完整结束.*当前文稿未改变/,
  );
});

test("terminal finish reason safely completes compatible streams that omit DONE", async () => {
  const ai = runtime(async () => sseResponse([
    `data: ${JSON.stringify({ choices: [{ delta: { content: "完成" }, finish_reason: "stop" }] })}`,
  ]));
  const result = await ai.run({ config: { base: "https://example.test/v1", model: "test", key: "key" }, messages: [], tools: [] });
  assert.equal(result.text, "完成");
});

test("DONE stops reading immediately and ignores any later tool delta", async () => {
  let reads = 0;
  let cancelled = false;
  const bytes = new TextEncoder().encode([
    `data: ${JSON.stringify({ choices: [{ delta: { content: "安全完成" } }] })}`,
    "data: [DONE]",
    `data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [{ index: 0, id: "late", type: "function", function: { name: "object.remove", arguments: '{}' } }] } }] })}`,
    "",
  ].join("\n"));
  const ai = runtime(async () => ({
    ok: true, status: 200, headers: { get: () => "text/event-stream" },
    body: { getReader: () => ({
      read: async () => {
        reads += 1;
        if (reads === 1) return { done: false, value: bytes };
        return new Promise(() => {});
      },
      cancel: async () => { cancelled = true; },
    }) },
  }));
  const result = await Promise.race([
    ai.completion({ config: { base: "https://example.test/v1", model: "test", key: "key" }, messages: [], tools: [] }),
    new Promise((_, reject) => setTimeout(() => reject(new Error("completion hung after DONE")), 100)),
  ]);
  assert.equal(result.content, "安全完成");
  assert.equal(result.tool_calls.length, 0);
  assert.equal(reads, 1);
  assert.equal(cancelled, true);
});

test("non-stream responses share the same tool-call safety limits", async () => {
  const ai = runtime(async () => ({
    ok: true, status: 200, headers: { get: () => "application/json" },
    json: async () => ({
      choices: [{
        finish_reason: "tool_calls",
        message: { role: "assistant", content: "", tool_calls: Array.from({ length: 9 }, (_, index) => ({ id: `call-${index}`, type: "function", function: { name: "object.add", arguments: "{}" } })) },
      }],
    }),
  }));
  await assert.rejects(
    ai.completion({ config: { base: "https://example.test/v1", model: "test", key: "key" }, messages: [], tools: [] }),
    /工具调用超过 8 个/,
  );
});

test("U AI executes a provider tool call only after approval and returns its result to the model", async () => {
  let round = 0;
  let invoked = null;
  let approved = null;
  const ai = runtime(async (_url, options) => {
    const body = JSON.parse(options.body);
    round += 1;
    if (round === 1) {
      assert.equal(body.tools[0].function.name, "slide.add");
      return sseResponse([
        `data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [{ index: 0, id: "call-1", type: "function", function: { name: "slide.add", arguments: '{"slide":{"title":"真实页面"}}' } }] } }] })}`,
        "data: [DONE]",
      ]);
    }
    const toolResult = body.messages.find((message) => message.role === "tool");
    assert.match(toolResult.content, /slide-2/);
    return sseResponse([
      `data: ${JSON.stringify({ choices: [{ delta: { content: "已新增 1 页。" } }] })}`,
      "data: [DONE]",
    ]);
  });
  const result = await ai.run({
    config: { base: "https://example.test/v1", model: "test", key: "key" },
    messages: [{ role: "user", content: "新增一页" }],
    tools: [{ name: "slide.add", description: "新增页面", inputSchema: { type: "object" } }],
    beforeTool: async (request) => { approved = request; return true; },
    callTool: async (name, args, request) => { invoked = { name, args, request }; return { slideId: "slide-2" }; },
  });
  assert.equal(approved.name, "slide.add");
  assert.equal(invoked.name, "slide.add");
  assert.equal(JSON.stringify(invoked.args), JSON.stringify({ slide: { title: "真实页面" } }));
  assert.equal(invoked.request, approved, "the exact approved request must carry its private capability to the host caller");
  assert.equal(result.text, "已新增 1 页。");
  assert.equal(result.rounds, 2);
});

test("pre-tool narration is progress only and is not merged into the final answer", async () => {
  let round = 0;
  const streamed = [];
  const ai = runtime(async () => {
    round += 1;
    if (round === 1) return sseResponse([
      `data: ${JSON.stringify({ choices: [{ delta: { content: "我先检查当前文稿。" } }] })}`,
      `data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [{ index: 0, id: "call-read", type: "function", function: { name: "deck.inspect", arguments: "{}" } }] } }] })}`,
      "data: [DONE]",
    ]);
    return sseResponse([
      `data: ${JSON.stringify({ choices: [{ delta: { content: "检查完成，这是最终答案。" } }] })}`,
      "data: [DONE]",
    ]);
  });
  const result = await ai.run({
    config: { base: "https://example.test/v1", model: "test", key: "key" },
    messages: [{ role: "user", content: "检查文稿" }],
    tools: [{ name: "deck.inspect", inputSchema: { type: "object" } }],
    callTool: async () => ({ slides: 1 }),
    onToken: (token) => streamed.push(token),
  });
  assert.equal(streamed.join(""), "我先检查当前文稿。检查完成，这是最终答案。");
  assert.equal(result.text, "检查完成，这是最终答案。");
});

test("a denied mutating tool is reported to the model and never reaches the document host", async () => {
  let round = 0;
  let calls = 0;
  const ai = runtime(async (_url, options) => {
    round += 1;
    if (round === 1) return sseResponse([
      `data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [{ index: 0, id: "call-2", type: "function", function: { name: "object.remove", arguments: '{"objectId":"danger"}' } }] } }] })}`,
      "data: [DONE]",
    ]);
    const body = JSON.parse(options.body);
    assert.match(body.messages.find((message) => message.role === "tool").content, /用户拒绝/);
    return sseResponse([`data: ${JSON.stringify({ choices: [{ delta: { content: "已取消删除。" } }] })}`, "data: [DONE]"]);
  });
  const result = await ai.run({
    config: { base: "https://example.test/v1", model: "test", key: "key" },
    messages: [{ role: "user", content: "删除对象" }],
    tools: [{ name: "object.remove", inputSchema: { type: "object" } }],
    beforeTool: async () => false,
    callTool: async () => { calls += 1; },
  });
  assert.equal(calls, 0);
  assert.equal(result.text, "已取消删除。");
});

test("multiple tool calls in one model round can be approved as one batch", async () => {
  let round = 0;
  let approvalCount = 0;
  const invoked = [];
  const ai = runtime(async () => {
    round += 1;
    if (round === 1) return sseResponse([
      `data: ${JSON.stringify({ choices: [{ delta: { tool_calls: [
        { index: 0, id: "call-a", type: "function", function: { name: "object.add", arguments: '{"slideId":"s1","object":{"kind":"text"}}' } },
        { index: 1, id: "call-b", type: "function", function: { name: "transition.set", arguments: '{"slideId":"s1","transition":{"kind":"fade"}}' } },
      ] } }] })}`,
      "data: [DONE]",
    ]);
    return sseResponse([`data: ${JSON.stringify({ choices: [{ delta: { content: "批量修改完成。" } }] })}`, "data: [DONE]"]);
  });
  const result = await ai.run({
    config: { base: "https://example.test/v1", model: "test", key: "key" },
    messages: [{ role: "user", content: "统一修改" }],
    tools: [
      { name: "object.add", inputSchema: { type: "object" } },
      { name: "transition.set", inputSchema: { type: "object" } },
    ],
    beforeTools: async (requests) => { approvalCount += 1; assert.equal(requests.length, 2); return true; },
    callTool: async (name) => { invoked.push(name); return { ok: true }; },
  });
  assert.equal(approvalCount, 1);
  assert.equal(invoked.join(","), "object.add,transition.set");
  assert.equal(result.text, "批量修改完成。");
});
