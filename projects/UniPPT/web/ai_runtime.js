(function (global) {
  "use strict";

  const DEFAULT_MAX_ROUNDS = 8;
  const MAX_STREAM_CONTENT_CHARS = 2 * 1024 * 1024;
  const MAX_STREAM_REASONING_CHARS = 1024 * 1024;
  const MAX_TOOL_ARGUMENT_CHARS = 1024 * 1024;
  const MAX_TOOL_CALLS = 8;
  const MAX_SSE_BUFFER_CHARS = 2 * 1024 * 1024;
  const MAX_TOOL_NAME_BYTES = 128;
  const MAX_TOOL_ID_BYTES = 256;

  function utf8Bytes(value) {
    return new TextEncoder().encode(String(value || "")).byteLength;
  }

  function limitedString(value, maximum, label) {
    const text = String(value || "");
    if (utf8Bytes(text) > maximum) throw new Error(`${label}超过安全上限`);
    return text;
  }

  function openAiTools(definitions) {
    return (definitions || []).map((tool) => ({
      type: "function",
      function: {
        name: tool.name,
        description: tool.description || "",
        parameters: tool.inputSchema || { type: "object", properties: {} },
      },
    }));
  }

  function configuredEndpoint(config) {
    const base = String(config?.base || "").replace(/\/+$/, "");
    return base ? `${base}/chat/completions` : "";
  }

  function appendToolDelta(target, delta) {
    const index = Number(delta?.index ?? 0);
    if (!Number.isInteger(index) || index < 0 || index >= MAX_TOOL_CALLS) throw new Error(`AI 返回了无效的工具调用索引 ${delta?.index}`);
    const call = target[index] || {
      id: "",
      type: "function",
      function: { name: "", arguments: "" },
    };
    if (delta.id && !call.id) call.id = limitedString(delta.id, MAX_TOOL_ID_BYTES, "AI 工具调用 ID");
    if (delta.type && delta.type !== "function") throw new Error(`AI 返回了不支持的工具调用类型 ${delta.type}`);
    if (delta.function?.name) {
      const incoming = String(delta.function.name);
      if (!call.function.name) call.function.name = incoming;
      else if (incoming === call.function.name || call.function.name.startsWith(incoming)) { /* repeated full name or prefix */ }
      else if (incoming.startsWith(call.function.name)) call.function.name = incoming;
      else call.function.name += incoming;
      limitedString(call.function.name, MAX_TOOL_NAME_BYTES, "AI 工具名称");
    }
    if (delta.function?.arguments) {
      call.function.arguments += delta.function.arguments;
      if (utf8Bytes(call.function.arguments) > MAX_TOOL_ARGUMENT_CHARS) throw new Error("AI 工具参数超过 1 MiB 安全上限");
    }
    target[index] = call;
  }

  function normalizeToolCalls(value) {
    const calls = Array.isArray(value) ? value : [];
    if (calls.length > MAX_TOOL_CALLS) throw new Error(`AI 一次返回的工具调用超过 ${MAX_TOOL_CALLS} 个安全上限`);
    return calls.map((call, index) => {
      if (!call || typeof call !== "object") throw new Error(`AI 第 ${index + 1} 个工具调用格式无效`);
      if (call.type && call.type !== "function") throw new Error(`AI 返回了不支持的工具调用类型 ${call.type}`);
      const name = limitedString(call.function?.name, MAX_TOOL_NAME_BYTES, "AI 工具名称");
      const args = limitedString(call.function?.arguments, MAX_TOOL_ARGUMENT_CHARS, "AI 工具参数");
      const id = limitedString(call.id, MAX_TOOL_ID_BYTES, "AI 工具调用 ID");
      if (!name) throw new Error(`AI 第 ${index + 1} 个工具调用缺少函数名`);
      return { id, type: "function", function: { name, arguments: args } };
    });
  }

  function messageFromJson(payload) {
    if (payload?.error) throw new Error(`AI 服务返回错误：${payload.error.message || payload.error}`);
    const choice = payload?.choices?.[0];
    if (!choice || !choice.message || typeof choice.message !== "object") throw new Error("AI 返回的 JSON 响应格式无效");
    const message = choice.message;
    return {
      role: "assistant",
      content: message.content == null ? "" : String(message.content),
      reasoning: String(message.reasoning_content || ""),
      tool_calls: normalizeToolCalls(message.tool_calls),
      finishReason: String(choice.finish_reason || ""),
    };
  }

  function assertCompletionFinished(finishReason) {
    if (finishReason === "length") throw new Error("模型达到输出上限，生成内容未执行，当前文稿未改变；请减少页数或重试");
    if (finishReason === "content_filter") throw new Error("模型输出被内容安全策略截断，当前文稿未改变");
  }

  async function readCompletionResponse(response, callbacks = {}) {
    const contentType = String(response.headers?.get?.("content-type") || "").toLowerCase();
    if (!response.ok) {
      const detail = (await response.text()).slice(0, 800);
      let message = detail;
      try { message = JSON.parse(detail).error || JSON.parse(detail).message || detail; } catch (_) {}
      throw new Error(`AI 服务 HTTP ${response.status}${message ? `：${message}` : ""}`);
    }
    if (!contentType.includes("text/event-stream")) {
      const message = messageFromJson(await response.json());
      assertCompletionFinished(message.finishReason);
      if (utf8Bytes(message.content) > MAX_STREAM_CONTENT_CHARS || utf8Bytes(message.reasoning) > MAX_STREAM_REASONING_CHARS) throw new Error("AI 返回内容超过安全上限");
      if (message.reasoning) callbacks.onThinking?.(message.reasoning);
      if (message.content) callbacks.onToken?.(message.content);
      return message;
    }

    if (!response.body?.getReader) throw new Error("当前浏览器不支持流式响应");
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    const toolCalls = [];
    let content = "";
    let reasoning = "";
    let buffer = "";
    let sawDone = false;
    let reachedEof = false;
    let finishReason = "";
    const consume = (raw) => {
      const line = raw.trim();
      if (!line.startsWith("data:")) return;
      const data = line.slice(5).trim();
      if (!data) return;
      if (data === "[DONE]") { sawDone = true; return; }
      let packet;
      try { packet = JSON.parse(data); }
      catch (error) { throw new Error(`AI 流式响应包含无效 JSON：${error.message}`); }
      if (packet?.error) throw new Error(`AI 服务返回错误：${packet.error.message || packet.error}`);
      const choice = packet?.choices?.[0];
      if (choice?.finish_reason) finishReason = String(choice.finish_reason);
      const delta = choice?.delta;
      if (!delta) return;
      const thought = String(delta.reasoning_content || "");
      const token = String(delta.content || "");
      if (thought) {
        reasoning += thought;
        if (utf8Bytes(reasoning) > MAX_STREAM_REASONING_CHARS) throw new Error("AI 思考流超过 1 MiB 安全上限");
        callbacks.onThinking?.(thought);
      }
      if (token) {
        content += token;
        if (utf8Bytes(content) > MAX_STREAM_CONTENT_CHARS) throw new Error("AI 内容流超过 2 MiB 安全上限");
        callbacks.onToken?.(token);
      }
      for (const call of delta.tool_calls || []) appendToolDelta(toolCalls, call);
      if (delta.tool_calls?.length) callbacks.onToolArguments?.(toolCalls.reduce((size, call) => size + utf8Bytes(call?.function?.arguments), 0));
    };
    try {
      while (!sawDone) {
        const { value, done } = await reader.read();
        buffer += decoder.decode(value || new Uint8Array(), { stream: !done });
        const lines = buffer.split(/\r?\n/);
        buffer = lines.pop() || "";
        for (const line of lines) {
          consume(line);
          if (sawDone) break;
        }
        if (utf8Bytes(buffer) > MAX_SSE_BUFFER_CHARS) throw new Error("AI 流式缓冲区超过 2 MiB 安全上限");
        if (done) { reachedEof = true; break; }
      }
      if (!sawDone && buffer) consume(buffer);
      const terminalFinish = ["stop", "tool_calls", "function_call"].includes(finishReason);
      if (!sawDone && !(reachedEof && terminalFinish)) throw new Error("AI 流式响应未完整结束，当前文稿未改变");
      assertCompletionFinished(finishReason);
      return { role: "assistant", content, reasoning, tool_calls: normalizeToolCalls(toolCalls.filter(Boolean)), finishReason };
    } catch (error) {
      try { await reader.cancel?.(error); } catch (_) {}
      throw error;
    } finally {
      if (sawDone) {
        try { await reader.cancel?.(); } catch (_) {}
      }
      try { reader.releaseLock?.(); } catch (_) {}
    }
  }

  async function completion({ config, messages, tools, signal, onThinking, onToken, onToolArguments, maxTokens, thinking, thinkingBudget, timeoutMs = 180000 }) {
    const ownKey = String(config?.key || "").trim();
    const builtin = !ownKey;
    const url = builtin ? "/api/ai/chat" : configuredEndpoint(config);
    if (!url) throw new Error("请先在 U AI 设置中填写模型接口地址、模型和 API Key");
    if (!builtin && !String(config?.model || "").trim()) throw new Error("请先填写模型名称");
    const headers = { "Content-Type": "application/json" };
    if (!builtin) headers.Authorization = `Bearer ${ownKey}`;
    const qwenCompatible = /(?:dashscope|aliyun)/i.test(String(config?.base || "")) || /^qwen/i.test(String(config?.model || ""));
    const controller = new AbortController();
    const abort = () => controller.abort(signal?.reason);
    if (signal?.aborted) abort();
    else signal?.addEventListener?.("abort", abort, { once: true });
    let timedOut = false;
    const timer = typeof global.setTimeout === "function" ? global.setTimeout(() => { timedOut = true; controller.abort(); }, Math.max(1, timeoutMs)) : null;
    try {
      const response = await fetch(url, {
        method: "POST",
        headers,
        body: JSON.stringify({
          model: builtin ? "" : String(config.model).trim(),
          messages,
          tools: tools?.length ? openAiTools(tools) : undefined,
          tool_choice: tools?.length ? "auto" : undefined,
          ...(qwenCompatible ? { enable_thinking: true } : {}),
          ...((qwenCompatible || builtin) && typeof thinking === "boolean" ? { enable_thinking: thinking } : {}),
          ...((qwenCompatible || builtin) && Number.isInteger(thinkingBudget) && thinkingBudget > 0 ? { thinking_budget: thinkingBudget } : {}),
          ...(Number.isInteger(maxTokens) && maxTokens > 0 ? { max_tokens: Math.min(maxTokens, 65536) } : {}),
          stream: true,
        }),
        signal: controller.signal,
      });
      return await readCompletionResponse(response, { onThinking, onToken, onToolArguments });
    } catch (error) {
      if (timedOut) throw new Error(`模型本轮请求超过 ${Math.round(timeoutMs / 1000)} 秒，已停止等待；本轮工具未执行，之前完成的修改保留。`);
      if (signal?.aborted || error.name === "AbortError") throw error;
      if (builtin && error.name === "TypeError") throw new Error(`本地 UniPPT 服务没有响应，请重新启动服务并刷新页面（${error.message}）`);
      throw error;
    } finally {
      if (timer != null) global.clearTimeout(timer);
      signal?.removeEventListener?.("abort", abort);
    }
  }

  function validateArguments(value, schema = {}, path = "arguments", depth = 0) {
    if (depth > 32) throw new Error(`${path} 嵌套过深`);
    if (schema.oneOf) {
      if (schema.oneOf.some((branch) => { try { validateArguments(value, branch, path, depth + 1); return true; } catch (_) { return false; } })) return;
      const operations = schema.oneOf.flatMap((branch) => branch.properties?.op?.enum || []);
      throw new Error(`${path} 操作格式无效：使用 op，合法值为 ${operations.join("/")}；请按 schema 提供必填字段`);
    }
    const types = Array.isArray(schema.type) ? schema.type : schema.type ? [schema.type] : [];
    const type = value === null ? "null" : Array.isArray(value) ? "array" : typeof value;
    if (types.length && !types.includes(type) && !(type === "number" && Number.isInteger(value) && types.includes("integer"))) throw new Error(`${path} 必须是 ${types.join("/")}，不可把数组或对象写成 JSON 字符串`);
    if (schema.enum && !schema.enum.includes(value)) throw new Error(`${path} 必须是 ${schema.enum.join("/")}`);
    if (type === "string" && schema.pattern && !new RegExp(schema.pattern).test(value)) throw new Error(`${path} 不符合格式 ${schema.pattern}`);
    if (type === "number" && (!Number.isFinite(value) || value < (schema.minimum ?? -Infinity) || value > (schema.maximum ?? Infinity))) throw new Error(`${path} 数值超出范围`);
    if (type === "array") {
      if (value.length < (schema.minItems ?? 0) || value.length > (schema.maxItems ?? Infinity)) throw new Error(`${path} 数组长度无效`);
      if (schema.items) value.forEach((item, i) => validateArguments(item, schema.items, `${path}[${i}]`, depth + 1));
    } else if (type === "object") {
      for (const key of schema.required || []) if (!(key in value)) throw new Error(`${path} 缺少 ${key}`);
      for (const [key, item] of Object.entries(value)) {
        if (schema.additionalProperties === false && !Object.hasOwn(schema.properties || {}, key)) throw new Error(`${path} 不支持字段 ${key}`);
        if (schema.properties?.[key]) validateArguments(item, schema.properties[key], `${path}.${key}`, depth + 1);
      }
    }
  }

  function parseArguments(call) {
    const raw = String(call?.function?.arguments || "").trim();
    if (!raw) return {};
    try { return JSON.parse(raw); }
    catch (error) { throw new Error(`工具 ${call?.function?.name || ""} 的参数不是有效 JSON：${error.message}`); }
  }

  async function run(options) {
    const messages = (options.messages || []).map((message) => ({ ...message }));
    const tools = options.tools || [];
    const toolMap = new Map(tools.map((tool) => [tool.name, tool]));
    const maxRounds = Math.max(1, Math.min(16, Number(options.maxRounds) || DEFAULT_MAX_ROUNDS));
    for (let round = 0; round < maxRounds; round += 1) {
      const message = await completion({
        config: options.config,
        messages,
        tools,
        signal: options.signal,
        maxTokens: options.maxTokens,
        thinking: options.thinking,
        thinkingBudget: options.thinkingBudget,
        timeoutMs: options.timeoutMs,
        onThinking: (token) => options.onThinking?.(token, round),
        onToken: (token) => options.onToken?.(token, round),
        onToolArguments: (bytes) => options.onToolArguments?.(bytes, round),
      });
      const assistant = {
        role: "assistant",
        content: message.content || "",
        ...(message.tool_calls?.length ? { tool_calls: message.tool_calls } : {}),
      };
      messages.push(assistant);
      if (!message.tool_calls?.length) {
        return { text: String(message.content || "").trim(), messages, rounds: round + 1 };
      }

      const requests = [];
      for (const call of message.tool_calls) {
        const name = String(call?.function?.name || "");
        if (!toolMap.has(name)) {
          messages.push({ role: "tool", tool_call_id: call.id || `missing-${round}`, content: JSON.stringify({ ok: false, error: `工具未开放：${name}` }) });
          continue;
        }
        let args;
        try {
          args = parseArguments(call);
          validateArguments(args, toolMap.get(name).inputSchema);
          await options.preflightTool?.(name, args);
        }
        catch (error) {
          messages.push({ role: "tool", tool_call_id: call.id || `invalid-${round}`, content: JSON.stringify({ ok: false, error: error.message }) });
          continue;
        }
        requests.push({ name, args, call, definition: toolMap.get(name), round });
      }
      if (requests.some((request) => options.terminalTools?.includes(request.name)) && requests.length !== 1) throw new Error("图片重建必须单独提交一次工具调用，未执行本轮修改");
      requests.forEach((request) => options.onToolState?.({ phase: "requested", ...request }));
      const batchApproval = requests.length && typeof options.beforeTools === "function"
        ? await options.beforeTools(requests, round)
        : null;
      for (const request of requests) {
        const { name, args, call } = request;
        const approved = batchApproval == null
          ? await (options.beforeTool?.(request) ?? true)
          : Boolean(batchApproval);
        if (!approved) {
          const denied = { ok: false, denied: true, error: "用户拒绝执行该工具" };
          messages.push({ role: "tool", tool_call_id: call.id || `denied-${round}`, content: JSON.stringify(denied) });
          options.onToolState?.({ phase: "denied", name, args, call, result: denied });
          if (options.terminalTools?.includes(name)) return { text: "已取消本次重建，文稿未改变。", messages, rounds: round + 1 };
          continue;
        }
        try {
          const result = await options.callTool(name, args, request);
          messages.push({ role: "tool", tool_call_id: call.id || `tool-${round}`, content: JSON.stringify({ ok: true, result }) });
          options.onToolState?.({ phase: "completed", name, args, call, result });
          if (options.terminalTools?.includes(name)) return { text: result?.completionText || "工具已执行，仍需检查生成结果。", messages, rounds: round + 1, result };
        } catch (error) {
          const failed = { ok: false, error: error.message };
          messages.push({ role: "tool", tool_call_id: call.id || `failed-${round}`, content: JSON.stringify(failed) });
          options.onToolState?.({ phase: "failed", name, args, call, result: failed });
        }
      }
    }
    throw new Error(`模型连续调用工具超过 ${maxRounds} 轮，已安全停止`);
  }

  async function builtinStatus(signal) {
    try {
      const response = await fetch("/api/ai/status", { cache: "no-store", signal });
      if (!response.ok) return { configured: false, error: `HTTP ${response.status}` };
      return await response.json();
    } catch (error) {
      return { configured: false, unreachable: true, error: error.message };
    }
  }

  global.UniPptAiRuntime = Object.freeze({
    run,
    completion,
    readCompletionResponse,
    openAiTools,
    validateArguments,
    builtinStatus,
  });
})(globalThis);
