"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const app = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const css = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");
const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const runtime = fs.readFileSync(path.join(__dirname, "ai_runtime.js"), "utf8");
const localSkills = fs.readFileSync(path.join(__dirname, "local_skill_runtime.js"), "utf8");
const presentationHost = fs.readFileSync(path.join(__dirname, "presentation_host.js"), "utf8");
const renderer = fs.readFileSync(path.join(__dirname, "ai_message_renderer.js"), "utf8");

function directDeckIntentFromRuntime(prompt) {
  const context = vm.createContext({ console });
  context.globalThis = context;
  vm.runInContext(localSkills, context, { filename: "local_skill_runtime.js" });
  return context.UniPptLocalSkills.isWholeDeckCreationIntent(prompt);
}

function sseResponse(content) {
  const bytes = new TextEncoder().encode([
    `data: ${JSON.stringify({ choices: [{ delta: { content } }] })}`,
    "data: [DONE]",
    "",
  ].join("\n"));
  let sent = false;
  return {
    ok: true,
    status: 200,
    headers: { get: () => "text/event-stream; charset=utf-8" },
    body: {
      getReader: () => ({
        read: async () => sent ? { done: true } : (sent = true, { done: false, value: bytes }),
        cancel: async () => {},
      }),
    },
    text: async () => new TextDecoder().decode(bytes),
  };
}

test("U AI uses the UniDoc Copilot task-pane interaction model", () => {
  assert.match(app, /panel\.id = "ai-panel"/);
  for (const className of [
    "ai-resizer", "ai-head", "ai-logo", "ai-msgs", "ai-hello", "ai-chips",
    "ai-composer", "ai-input", "ai-modes", "ai-send",
  ]) {
    assert.match(app, new RegExp(`class=\\"[^\\"]*${className}`));
  }
  assert.match(app, /value="chat" checked/);
  assert.match(app, /value="edit"/);
  assert.match(app, /event\.key === "Enter" && !event\.shiftKey/);
  assert.match(app, /state\.documentHost\.ai\.tools/);
  assert.match(app, /CustomEvent\("unippt:ai-request"/);
  assert.match(app, /UniPptAiRuntime/);
  assert.match(app, /appendToolBatchApproval/);
  assert.match(app, /beforeTools\(requests\)/);
  assert.doesNotMatch(app, /configured \?\? await runLocalAiAssistant/);
  assert.match(app, /localStorage\.setItem\("unippt-ai-width"/);
  assert.match(css, /#ai-panel\.ai-full/);
  assert.match(css, /#ai-panel \.ai-composer:focus-within/);
  assert.match(css, /#ai-panel \.ai-modes label:has\(input:checked\)/);
});

test("U AI has a visible title-bar entry and a keyboard entry point", () => {
  assert.match(html, /rel="icon" href="\{\{asset:\/unippt-logo\.svg\}\}"/);
  assert.match(html, /class="app-icon" src="\{\{asset:\/unippt-logo\.svg\}\}"/);
  assert.match(html, /id="aiAssistantTop"[^>]*>[\s\S]*class="title-ai-logo" src="\{\{asset:\/unippt-logo\.svg\}\}"[\s\S]*U AI<\/button>/);
  assert.match(app, /class="ai-logo" src="\$\{startupAsset\("\/unippt-logo\.svg"\)\}"/);
  assert.match(app, /class="ai-hello-logo" src="\$\{startupAsset\("\/unippt-logo\.svg"\)\}"/);
  assert.match(html, /\{\{asset:\/ai_runtime\.js\}\}/);
  assert.match(html, /\{\{asset:\/ai_attachment_runtime\.js\}\}/);
  assert.match(html, /\{\{asset:\/html_to_ppt_runtime\.js\}\}/);
  assert.match(html, /\{\{asset:\/local_skill_runtime\.js\}\}/);
  assert.match(html, /\{\{asset:\/ai_message_renderer\.js\}\}/);
  assert.match(app, /\$\("#aiAssistantTop"\)\.onclick\s*=\s*\(\)\s*=>\s*\{\s*void openAiAssistant\(\)/);
  assert.match(app, /event\.altKey[\s\S]*event\.key\.toLowerCase\(\) === "a"[\s\S]*openAiAssistant\(\)/);
  assert.match(css, /text-size-adjust:\s*100%/);
});

test("U AI accepts presentation and multimodal attachments", () => {
  assert.match(app, /class="ai-attachment-input"[^>]*accept="\.pptx,\.udoc,\.html,\.htm,image\/\*/);
  assert.match(app, /UniPptAiAttachments/);
  assert.match(app, /runtime\.prepareFile\(file\)/);
  assert.match(app, /attachmentRuntime\.messagePayload\(prompt, submittedAttachments\)/);
  assert.match(app, /attachments: submittedAttachments/);
  assert.match(app, /composer\.addEventListener\("drop"/);
  assert.match(app, /input\.addEventListener\("paste"/);
  assert.match(app, /if \(!prompt && !attachments\.length\) return/);
  assert.match(css, /\.ai-message-attachment img/);
});

test("U AI renders assistant Markdown and LaTeX through the shared safe renderer", () => {
  assert.match(app, /UniPptAiMessageRenderer/);
  assert.match(app, /renderAiMessage\(reply\.body, reply\.rawText\)/);
  assert.match(app, /回复默认使用清晰的 Markdown 排版/);
  assert.match(renderer, /markdownToHtml/);
  assert.match(renderer, /global\.katex\.render/);
  assert.match(renderer, /trust: false/);
  assert.match(css, /#ai-panel \.ai-md-table-wrap/);
  assert.match(css, /#ai-panel \.ai-math-block/);
});

test("tool approvals expose executing, completed, failed and denied states", () => {
  assert.match(app, /toolCards = new Map/);
  assert.match(app, /updateToolCard\(event\)/);
  assert.match(app, /UNIPPT_AI_TOOL_UI/);
  assert.match(app, /appendToolBatchApproval/);
  assert.match(app, /统一允许 \$\{requests\.length\} 项/);
  assert.match(app, /requests\.filter\(\(request\) => !UNIPPT_AI_READ_ONLY_TOOLS\.has/);
  assert.doesNotMatch(app, /toast\(`\$\{event\.name\} 已执行/);
  assert.match(app, /event\.phase === "completed" && !toolUi\.readOnly/);
  assert.match(app, /allow\.textContent = approved \? "执行中…"/);
});

test("streaming follows the tail only until the user scrolls away and messages are copyable", () => {
  assert.match(app, /let aiFollowTail = true/);
  assert.match(app, /messages\.addEventListener\("scroll"/);
  assert.match(app, /if \(!force && !aiFollowTail\) return/);
  assert.match(app, /scrollAiMessagesToEnd\(\)/);
  assert.match(app, /className = "ai-message-copy"|element\("button", "ai-message-copy", "复制"\)/);
  assert.match(app, /navigator\.clipboard\?\.writeText/);
  assert.match(css, /\.ai-msg:hover \.ai-message-copy/);
  assert.match(css, /\.ai-thinking[^}]*white-space:nowrap[^}]*text-overflow:ellipsis/);
});

test("AI thinking and pre-tool narration stay in one compact status line", () => {
  assert.match(app, /reply\.message\.classList\.add\("ai-processing"\)/);
  assert.match(app, /正在分析图片并规划可编辑图层/);
  assert.match(app, /正在生成结构化重建方案/);
  assert.match(app, /appendProgressToken\(thinkingByRound, token, round\)/);
  assert.match(app, /compact\.slice\(-219\)/);
  assert.match(app, /replace\(\/\\s\+\/g, " "\)/);
  assert.match(app, /reply\.message\.classList\.remove\("ai-processing"\)/);
  assert.match(css, /\.ai-processing \.ai-body[^}]*display:none/);
  assert.match(css, /\.ai-processing \.ai-bubble:not\(:has\(\.ai-tool-card\)\)[^}]*padding:4px 10px/);
  assert.match(app, /thinking\.dataset\.elapsed/);
  assert.match(css, /aiThinkingPulse/);
});

test("image reconstruction is one safe attachment-backed tool call", () => {
  assert.match(app, /slide\.reconstructFromImage/);
  assert.match(app, /imageReconstruction[\s\S]*tool\.name === "slide\.reconstructFromImage"/);
  assert.match(app, /sourceImage: attachment\.dataUrl/);
  assert.match(app, /attachmentRuntime\.detectImageText/);
  assert.match(app, /ocrDetection: ocrDetections\.get\(attachment\.name\)/);
  assert.match(app, /OCR 参考检测结果，可由原图校正/);
  assert.match(app, /UniPptImageReconstruction\.detectAnchors/);
  const imageRuntime = fs.readFileSync(path.join(__dirname, "image_reconstruction_runtime.js"), "utf8");
  assert.match(imageRuntime, /timeoutMs=options.timeoutMs \?\? 45000/);
  assert.match(imageRuntime, /detectOnly:true,timeoutMs/);
  assert.match(imageRuntime, /Promise\.allSettled/);
  assert.match(app, /fastImageRound \? globalThis.UniPptImageFast : reconstruction/);
  assert.ok(app.indexOf("const prepared = await (fastImageRound") >= 0);
  assert.ok(app.indexOf("const prepared = await (fastImageRound") < app.indexOf('const request = { name: "slide.reconstructFromImage"'));
  assert.match(app, /mode: "native", preserveSourceImage: false/);
  assert.match(app, /thinkingBudget: imageReconstruction \? 2048/);
  assert.match(app, /terminalTools: imageReconstruction/);
  assert.match(app, /afterSlideId: args\.afterSlideId \|\| context\?\.activeSlide\?\.id/);
  assert.match(app, /未获得替换整份演示文稿的明确指令/);
  assert.match(app, /未获得删除页面或对象的明确指令/);
  assert.match(app, /已阻止破坏性事务/);
  assert.match(app, /只能调用一次 slide\.reconstructFromImage/);
});

test("free HTML conversion is one source-backed atomic tool call", () => {
  assert.match(app, /presentation\.importFreeHtml/);
  assert.match(app, /const htmlAttachments = submittedAttachments\.filter/);
  assert.match(app, /sourceHtml: attachment\.html/);
  assert.match(app, /自由 HTML 并要求转为 PPT/);
  assert.match(app, /未获得把自由 HTML 转换为整份 PPT 的明确指令/);
  assert.match(app, /UniPptLocalSkills\?\.match/);
  assert.match(app, /localSkills\.run\(localSkillPlan/);
  assert.match(localSkills, /free-html-to-editable-ppt/);
  assert.match(localSkills, /presentation\.importFreeHtml/);
  assert.match(app, /const generatedHtmlDeck =/);
  assert.match(app, /UniPptLocalSkills\?\.isWholeDeckCreationIntent\?\.\(prompt\) === true/);
  assert.match(app, /runtime\.completion\(\{[\s\S]*tools: \[\][\s\S]*maxTokens: 32768/);
  assert.match(app, /preflightUniPptAiHtml\(generated\.content/);
  assert.match(app, /notesTemplate\.content\.append\(documentNode\.createTextNode/);
  assert.match(app, /sourceHtml: normalized \? `<!doctype html>\\n\$\{documentNode\.documentElement\.outerHTML\}` : sourceHtml/);
  assert.match(app, /generatedByAi: true/);
  assert.match(app, /assertTargetUnchanged/);
  assert.match(app, /error\?\.code !== "AI_HTML_QUALITY_FAILED"/);
  assert.match(app, /正在修订自由 HTML/);
  assert.match(app, /【质量 findings】/);
  assert.match(app, /【预检错误】/);
  assert.match(app, /正在接收结构修订稿/);
  const generatedPlan = app.slice(app.indexOf("const generatedPlan ="), app.indexOf("setAiProgress(\"自由 HTML 预检通过\""));
  assert.match(generatedPlan, /sourceBytes:/);
  assert.doesNotMatch(generatedPlan.match(/args:\s*\{[\s\S]*?\n\s*\},/)?.[0] || "", /sourceHtml\s*:/);
});

test("a direct Chinese creation request generates HTML and imports it instead of replying with a plan", async () => {
  const prompt = "给我创造一个关于AI演讲的超赞PPT";
  assert.equal(directDeckIntentFromRuntime(prompt), true, "natural Chinese creation wording must select the whole-deck HTML route");

  const generatedHtml = `<!doctype html><html lang="zh-CN"><head>
    <meta name="unippt-title" content="AI 重塑未来">
  </head><body>
    <section class="omnidoc-page" data-page-size="1280x720" data-unippt-role="cover">
      <h1 data-unippt-role="cover-title">AI 重塑未来</h1>
      <template data-unippt-notes>[Sources]\n- No external sources; authored synthesis.\n[/Sources]</template>
    </section>
    <section class="omnidoc-page" data-page-size="1280x720" data-unippt-role="closing">
      <h2 data-unippt-role="closing-title">从一个工作流开始</h2>
      <template data-unippt-notes>[Sources]\n- No external sources; authored synthesis.\n[/Sources]</template>
    </section>
  </body></html>`;
  let completionBody = null;
  let convertedSource = "";
  const converter = {
    convert: async (source, options) => {
      convertedSource = source;
      assert.equal(options.restrictNetwork, true);
      return {
        title: "AI 重塑未来", width: 1280, height: 720,
        slides: [
          {
            name: "AI 重塑未来", background: "#071426",
            notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]",
            semantic: { role: "cover", summary: "AI 重塑未来" },
            objects: [
              { kind: "shape", name: "HTML 形状 · cover-background", frame: { x: 0, y: 0, width: 1280, height: 720 }, style: { fill: "#071426" } },
              { kind: "text", name: "HTML 文本 · cover-title", text: "AI 重塑未来", frame: { x: 80, y: 180, width: 1000, height: 100 }, textStyle: { fontSize: 56, color: "#fff" } },
            ],
          },
          {
            name: "从一个工作流开始", background: "#f5f2ea",
            notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]",
            semantic: { role: "closing", summary: "从一个工作流开始" },
            objects: [
              { kind: "shape", name: "HTML 形状 · closing-background", frame: { x: 0, y: 0, width: 1280, height: 720 }, style: { fill: "#f5f2ea" } },
              { kind: "text", name: "HTML 文本 · closing-title", text: "从一个工作流开始", frame: { x: 90, y: 170, width: 980, height: 90 }, textStyle: { fontSize: 44, color: "#152033" } },
            ],
          },
        ],
        report: { pageCount: 2, nativeText: 2, nativeShapes: 2, preservedImages: 0, localizedFallbacks: 0, unsupported: [], fidelityRisk: "low", editabilityRatio: 100 },
      };
    },
  };
  const context = vm.createContext({
    console,
    structuredClone,
    TextDecoder,
    TextEncoder,
    Uint8Array,
    AbortController,
    setTimeout,
    clearTimeout,
    crypto: { randomUUID: () => "direct-html-runtime-id" },
    fetch: async (_url, options) => {
      completionBody = JSON.parse(options.body);
      return sseResponse(generatedHtml);
    },
    UniPptHtmlToPpt: converter,
  });
  context.globalThis = context;
  vm.runInContext(runtime, context, { filename: "ai_runtime.js" });
  vm.runInContext(localSkills, context, { filename: "local_skill_runtime.js" });
  vm.runInContext(presentationHost, context, { filename: "presentation_host.js" });

  const generated = await context.UniPptAiRuntime.completion({
    config: { base: "https://example.test/v1", model: "test", key: "key" },
    messages: [{ role: "system", content: "只生成完整自由 HTML" }, { role: "user", content: prompt }],
    tools: [],
    maxTokens: 32768,
  });
  assert.equal(generated.content, generatedHtml);
  assert.equal(completionBody.tools, undefined, "HTML authoring must not expose legacy slide tools to the model");
  assert.equal(completionBody.tool_choice, undefined);

  let deck = {
    format: "unippt", version: 1, title: "旧演示", width: 1280, height: 720, fonts: [],
    slides: [{ id: "old-slide", name: "旧页", background: "#fff", notes: "", objects: [], animations: [], transition: null }],
  };
  const originalDeck = deck;
  let revision = 4;
  let commits = 0;
  let authorizer = null;
  const host = context.UniPptPresentationHost.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next, _outcome, options) => {
      assert.equal(options.replace, true);
      assert.equal(options.expectedDeckRef, originalDeck);
      deck = next;
      revision += 1;
      commits += 1;
    },
    getSelection: () => ({ slideId: deck.slides[0].id, objectId: null }),
    ai: { bindMutationAuthorizer: (value) => { authorizer = value; } },
  }, { idFactory: (prefix) => `${prefix}-direct-html-${Math.random()}` });

  const attachment = { kind: "html", name: "AI 重塑未来.html", html: generated.content };
  const plan = {
    id: "free-html-to-editable-ppt",
    label: "AI 自由 HTML → 可编辑 UniPPT",
    toolName: "presentation.importFreeHtml",
    attachment,
    args: { attachmentName: attachment.name, title: "AI 重塑未来", pageCount: 2 },
  };
  assert.equal(plan.args.sourceHtml, undefined, "raw generated HTML stays out of the approval-card arguments");
  let approvedRequest = null;
  let capability = null;
  const invokedTools = [];
  const outcome = await context.UniPptLocalSkills.run(plan, {
    beforeTools: async (requests) => {
      assert.equal(requests.length, 1);
      [approvedRequest] = requests;
      capability = authorizer.issue(requests);
      return true;
    },
    callTool: async (name, args, request) => {
      invokedTools.push(name);
      assert.equal(request, approvedRequest);
      return host.ai.callTool(name, {
        ...args,
        sourceHtml: attachment.html,
        generatedByAi: true,
        audience: "演讲观众",
        purpose: "解释 AI 的业务影响",
        centralTakeaway: "AI 价值来自工作流重构",
        language: "zh-CN",
      }, { capability });
    },
  });

  assert.deepEqual(invokedTools, ["presentation.importFreeHtml"]);
  assert.equal(commits, 1, "the complete generated deck must replace the document atomically once");
  assert.equal(convertedSource, generatedHtml);
  assert.equal(outcome.result.quality.passed, true, JSON.stringify(outcome.result.quality.findings));
  assert.equal(deck.slides.length, 2);
  assert.equal(deck.extensions["org.unippt.ai"].designTokens.generator, "ai-html-composer-v1");
  assert.match(outcome.text, /已通过本地 Skill/);
});

test("AI edit requests use one approval gateway without exposing a mutable host", () => {
  assert.match(app, /const allTools = state\.documentHost\.ai\.tools/);
  assert.match(app, /selection: state\.documentHost\.selection\.get\(\)/);
  assert.doesNotMatch(app, /UniPptAiProvider\?\.run|provider\.run\(/);
  const eventDetail = app.slice(app.indexOf("const detail = {", app.indexOf("const requestMessages")), app.indexOf("const callAiTool", app.indexOf("const requestMessages")));
  assert.match(eventDetail, /toolNames:/);
  assert.doesNotMatch(eventDetail, /host:\s*state\.documentHost/);
  assert.match(app, /beforeTools\(requests\)[\s\S]*approveAiToolBatch/);
  assert.match(app, /approvedMutationCapabilities = new WeakMap/);
  assert.match(app, /event instanceof MouseEvent[\s\S]*event\.isTrusted[\s\S]*event\.currentTarget === allow/);
  assert.match(app, /ai\.callTool\(name,[\s\S]*approvedCallOptions\(request\)/);
  assert.doesNotMatch(app, /globalThis\.(?:UniPpt|UniDoc).*MutationAuthorizer/);
  assert.doesNotMatch(app, /new Function\(/);
});

test("U AI runtime supports streaming OpenAI-compatible responses and confirmed tool calls", () => {
  assert.match(runtime, /text\/event-stream/);
  assert.match(runtime, /reasoning_content/);
  assert.match(runtime, /delta\.tool_calls/);
  assert.match(runtime, /beforeTool/);
  assert.match(runtime, /用户拒绝执行该工具/);
  assert.match(runtime, /\/api\/ai\/chat/);
  assert.match(runtime, /enable_thinking: true/);
});

test("U AI saves a local model key through the loopback server instead of losing it on reload", () => {
  assert.match(app, /fetch\("\/api\/ai\/config"/);
  assert.match(app, /重启和其他浏览器也可使用/);
  assert.match(app, /JSON\.stringify\(\{ \.\.\.next, key: "" \}\)/);
  assert.match(app, /本地服务未连接：请重新启动 UniPPT 后刷新/);
});
