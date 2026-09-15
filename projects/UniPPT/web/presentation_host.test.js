"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

function secureRuntime(extra = {}) {
  const context = vm.createContext({
    console,
    structuredClone,
    TextEncoder,
    setTimeout,
    clearTimeout,
    crypto: { randomUUID: () => "runtime-id" },
    ...extra,
  });
  context.globalThis = context;
  const source = fs.readFileSync(path.join(__dirname, "presentation_host.js"), "utf8");
  vm.runInContext(source, context, { filename: "presentation_host.js" });
  return context.UniPptPresentationHost;
}

// Most historical host tests exercise tool semantics rather than the approval
// boundary. Keep them concise by issuing a fresh capability per mutation; the
// dedicated security tests below use secureRuntime() directly.
function runtime(extra = {}) {
  const nativeRuntime = secureRuntime(extra);
  return {
    ...nativeRuntime,
    create(adapter = {}, options = {}) {
      let authorizer = null;
      const originalBind = adapter.ai?.bindMutationAuthorizer;
      const api = nativeRuntime.create({
        ...adapter,
        ai: {
          ...(adapter.ai || {}),
          bindMutationAuthorizer(value) {
            authorizer = value;
            originalBind?.(value);
          },
        },
      }, options);
      const securedCallTool = api.ai.callTool;
      api.ai.callTool = (name, args, callOptions) => {
        if (callOptions || ["presentation.inspect", "presentation.validate"].includes(name)) {
          return securedCallTool(name, args, callOptions);
        }
        return securedCallTool(name, args, { capability: authorizer.issue([{ name }]) });
      };
      return api;
    },
  };
}

function deckFixture() {
  return {
    format: "unippt",
    version: 1,
    title: "架构测试",
    width: 1280,
    height: 720,
    sourceWidthEmu: 12192000,
    sourceHeightEmu: 6858000,
    sourceImportId: "pptx-native-cache",
    fonts: [],
    slides: [{
      id: "slide-1",
      sourcePartName: "/ppt/slides/slide1.xml",
      name: "封面",
      background: "#fff",
      backgroundAsset: null,
      notes: "",
      masterObjects: [],
      layoutObjects: [],
      inheritedAnimations: [],
      objects: [{
        id: "shape-title",
        sourceShapeId: 42,
        name: "标题 1",
        kind: "text",
        frame: { x: 80, y: 80, width: 900, height: 100, rotation: 0 },
        text: "旧标题",
        textParagraphs: [],
        textFrame: { autoSize: "none" },
        textStyle: { fontSize: 44, color: "#111", align: "left" },
        style: { fill: "transparent", stroke: "transparent", strokeWidth: 0, opacity: 1 },
        children: [],
      }],
      animations: [],
      transition: null,
      sourceTimingXml: "<p:timing/>",
      sourceTransitionXml: null,
    }],
  };
}

function idFactory() {
  let sequence = 0;
  return (prefix) => `${prefix}-test-${++sequence}`;
}

test("semantic extension is namespaced, stable and preserves native bindings", () => {
  const host = runtime();
  const deck = deckFixture();
  host.enrichSemantics(deck, { idFactory: idFactory() });
  const semantic = deck.extensions[host.AI_EXTENSION];
  assert.equal(semantic.schemaVersion, 1);
  assert.equal(semantic.slides["slide-1"].role, "cover");
  assert.equal(semantic.objects["shape-title"].role, "slide-title");
  assert.equal(deck.slides[0].sourcePartName, "/ppt/slides/slide1.xml");
  assert.equal(deck.slides[0].objects[0].sourceShapeId, 42);
});

test("ChangeSet atomically updates text, objects, animation and transition", () => {
  const host = runtime();
  const original = deckFixture();
  const result = host.applyChangeSet(original, {
    transactionId: "tx-1",
    baseRevision: 7,
    operations: [
      { op: "updateText", slideId: "slide-1", objectId: "shape-title", text: "AI 新标题" },
      { op: "addObject", slideId: "slide-1", object: { kind: "text", name: "正文", text: "结构化内容", frame: { x: 100, y: 250, width: 700, height: 180 } }, semantic: { role: "body-text" } },
      { op: "addAnimation", slideId: "slide-1", targetObjectId: "shape-title", animation: { effect: "fade", trigger: "onClick", durationMs: 900 } },
      { op: "setTransition", slideId: "slide-1", transition: { kind: "wind", durationMs: 1600 } },
    ],
  }, { revision: 7, idFactory: idFactory() });
  assert.equal(original.slides[0].objects[0].text, "旧标题", "source deck must remain untouched");
  assert.equal(result.deck.slides[0].objects[0].text, "AI 新标题");
  assert.equal(result.deck.slides[0].objects.length, 2);
  assert.equal(result.deck.slides[0].animations[0].targetObjectId, "shape-title");
  assert.equal(result.deck.slides[0].animations[0].durationMs, 900);
  assert.equal(result.deck.slides[0].transition.kind, "wind");
  assert.deepEqual([...result.applied], ["updateText", "addObject", "addAnimation", "setTransition"]);
  assert.equal(result.issues.filter((issue) => issue.severity === "error").length, 0);
});

test("native identity is protected and failed transactions are non-mutating", () => {
  const host = runtime();
  const original = deckFixture();
  assert.throws(() => host.applyChangeSet(original, {
    operations: [{ op: "updateObject", objectId: "shape-title", patch: { sourceShapeId: 999 } }],
  }), (error) => error.code === "PROTECTED_NATIVE_FIELD");
  assert.equal(original.slides[0].objects[0].sourceShapeId, 42);
  assert.equal(original.extensions, undefined);
});

test("revision conflicts and dangling animation targets are rejected", () => {
  const host = runtime();
  assert.throws(() => host.applyChangeSet(deckFixture(), {
    baseRevision: 3,
    operations: [{ op: "setTitle", title: "冲突" }],
  }, { revision: 4 }), (error) => error.code === "REVISION_CONFLICT");

  const broken = deckFixture();
  broken.slides[0].animations.push({ id: "anim-broken", targetObjectId: "missing", durationMs: 500 });
  const issues = host.validateDeck(broken);
  assert.ok(issues.some((issue) => issue.code === "MISSING_ANIMATION_TARGET" && issue.severity === "error"));
});

test("AI tool host commits through one revisioned adapter and plugins are typed", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  let activationCount = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    getSelection: () => ({ slideId: deck.slides[0].id, objectId: null }),
  }, { idFactory: idFactory() });
  api.registerPlugin({ id: "shared", supportedTypes: ["docs", "pptx"], activate: () => { activationCount += 1; } });
  api.registerPlugin({ id: "word-only", supportedTypes: ["docs"], activate: () => { throw new Error("must not activate"); } });
  assert.equal(activationCount, 1);
  assert.equal(api.plugins.list().find((item) => item.id === "word-only").compatible, false);

  const change = await api.ai.callTool("object.setText", { objectId: "shape-title", text: "工具修改" });
  assert.equal(change.revision, 1);
  assert.equal(deck.slides[0].objects[0].text, "工具修改");
  assert.equal(api.doc.pageCount(), 1);
  assert.match(api.doc.getText(), /工具修改/);

  const created = await api.ai.callTool("presentation.create", {
    title: "AI 自动生成",
    slides: [
      {
        title: "封面",
        subtitle: "结构化生成",
        animations: [
          { targetIndex: 0, effect: "zoom", trigger: "withPrevious", durationMs: 720 },
          { targetName: "副标题", effect: "fade", trigger: "afterPrevious", durationMs: 520 },
        ],
      },
      {
        title: "路线图",
        bullets: ["语义模型", "事务系统", "无损导出"],
        animations: [{ targetIndex: 1, effect: "wipe", direction: "left", durationMs: 760 }],
      },
    ],
  });
  assert.equal(created.slideIds.length, 2);
  assert.equal(deck.title, "AI 自动生成");
  assert.equal(deck.slides.length, 2);
  assert.deepEqual(deck.slides[0].animations.map((animation) => animation.targetObjectId), deck.slides[0].objects.map((object) => object.id));
  assert.equal(deck.slides[0].animations.map((animation) => animation.trigger).join(","), "withPrevious,afterPrevious");
  assert.equal(deck.slides[1].animations[0].targetObjectId, deck.slides[1].objects[1].id);
  assert.equal(deck.slides[1].animations[0].effect, "wipe");
  assert.equal(deck.sourceImportId, null);
});

test("public ai.callTool mutations require a scoped one-time approval capability", async () => {
  const runtimeApi = secureRuntime();
  let deck = deckFixture();
  let revision = 0;
  let authorizer = null;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    ai: { bindMutationAuthorizer: (value) => { authorizer = value; } },
  }, { idFactory: idFactory() });

  assert.equal(api.ai.issueMutationCapability, undefined, "the issuer must not be exposed on UniPpt/UniDoc");
  assert.equal(api.ai.authorizer, undefined, "the private approval bridge must not leak on the public host");
  await assert.rejects(
    api.ai.callTool("object.setText", { objectId: "shape-title", text: "绕过批准" }),
    (error) => error.code === "AI_TOOL_AUTH_REQUIRED",
  );
  assert.equal(revision, 0);
  assert.equal(deck.slides[0].objects[0].text, "旧标题");

  const inspection = await api.ai.callTool("presentation.inspect", { includeObjects: true });
  assert.equal(inspection.slides[0].objects[0].text, "旧标题", "read-only tools remain available without a capability");

  const capability = authorizer.issue([{ name: "object.setText" }]);
  await assert.rejects(
    api.ai.callTool("slide.add", { slide: { title: "越权页面" } }, { capability }),
    (error) => error.code === "AI_TOOL_AUTH_SCOPE_MISMATCH",
  );
  const changed = await api.ai.callTool(
    "object.setText",
    { objectId: "shape-title", text: "批准后的标题" },
    { capability },
  );
  assert.equal(changed.revision, 1);
  assert.equal(deck.slides[0].objects[0].text, "批准后的标题");
  await assert.rejects(
    api.ai.callTool("object.setText", { objectId: "shape-title", text: "重复使用" }, { capability }),
    (error) => error.code === "AI_TOOL_AUTH_INVALID",
  );
  assert.equal(deck.slides[0].objects[0].text, "批准后的标题");
});

test("a failed mutation restores the same approved capability for one deterministic retry", async () => {
  const runtimeApi = secureRuntime();
  let deck = deckFixture();
  let revision = 0;
  let authorizer = null;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    ai: { bindMutationAuthorizer: (value) => { authorizer = value; } },
  }, { idFactory: idFactory() });
  const capability = authorizer.issue([{ name: "object.setText" }]);

  await assert.rejects(
    api.ai.callTool("object.setText", { objectId: "missing", text: "失败" }, { capability }),
    (error) => error.code === "OBJECT_NOT_FOUND",
  );
  assert.equal(revision, 0);
  await api.ai.callTool("object.setText", { objectId: "shape-title", text: "同一次批准的修复重试" }, { capability });
  assert.equal(revision, 1);
  assert.equal(deck.slides[0].objects[0].text, "同一次批准的修复重试");
});

test("slide.add exposes a complete one-call slide schema and returns its generated id", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    getSelection: () => ({ slideId: deck.slides[0].id, objectId: null }),
  }, { idFactory: idFactory() });
  const definition = api.ai.tools.find((tool) => tool.name === "slide.add");
  assert.ok(definition.inputSchema.properties.slide.properties.title);
  assert.ok(definition.inputSchema.properties.slide.properties.subtitle);
  assert.ok(definition.inputSchema.properties.slide.properties.objects);
  assert.match(definition.description, /一次性新增完整幻灯片/);

  const result = await api.ai.callTool("slide.add", {
    afterSlideId: "slide-1",
    slide: { title: "一次完成", subtitle: "无需猜测 ID" },
  });
  assert.equal(result.slideId, result.activeSlideId);
  assert.equal(deck.slides.length, 2);
  assert.equal(deck.slides[1].objects.map((object) => object.text).join("|"), "一次完成|无需猜测 ID");
});

test("presentation.compose turns one semantic plan into a consistent editable executive deck", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  let commits = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; commits += 1; },
    getSelection: () => ({ slideId: deck.slides[0].id, objectId: null }),
  }, { idFactory: idFactory() });
  const definition = api.ai.tools.find((tool) => tool.name === "presentation.compose");
  assert.match(definition.description, /咨询级/);
  assert.equal(definition.inputSchema.required.join(","), "title,audience,purpose,centralTakeaway,slides");
  assert.ok(definition.inputSchema.properties.slides.items.properties.sources);

  const result = await api.ai.callTool("presentation.compose", {
    title: "AI 重塑企业",
    audience: "企业管理层",
    purpose: "推动管理层批准首个 90 天行动",
    centralTakeaway: "竞争优势来自组织能力而非模型接入",
    designPreset: "midnight-consulting",
    slides: [
      { role: "cover", title: "AI 重塑企业的下一个十年", subtitle: "从工具接入走向组织进化" },
      {
        role: "big-number", section: "01 / WHY NOW", title: "采用已跨过临界点",
        metric: { value: "78%", label: "组织已在至少一个环节使用 AI", context: "采用扩散意味着观望本身也是选择。" },
        chart: { chartType: "bar", categories: ["2023", "2024"], series: [{ name: "采用率", values: [55, 78] }] },
        takeaway: "领先者已经开始重构流程。", sources: [{ label: "Stanford HAI 2025", url: "https://hai.stanford.edu/ai-index" }],
      },
      {
        role: "three-columns", section: "02 / MOAT", title: "真正稀缺的不是模型",
        columns: [
          { number: "01", title: "模型接入", body: "能力会快速普及。" },
          { number: "02", title: "系统嵌入", body: "把知识与岗位连接。" },
          { number: "03", title: "组织护城河", body: "重构工作流并持续学习。", accent: true },
        ], takeaway: "工作流、数据与组织能力才构成护城河。",
      },
      {
        role: "process", section: "03 / ROADMAP", title: "90 天把愿景变成证据",
        steps: [
          { period: "0—30 天", title: "划清红线", body: "场景清单与责任人" },
          { period: "31—60 天", title: "验证流程", body: "受控试点与基线" },
          { period: "61—90 天", title: "准备扩张", body: "价值证据与路线图" },
        ], takeaway: "每个里程碑都要回答价值是否可见、风险是否可控。",
      },
      {
        role: "layers", section: "04 / OPERATING SYSTEM", title: "把 AI 变成企业操作系统",
        layers: [
          { title: "业务价值层", body: "战略命题 · 场景优先级 · 经营指标" },
          { title: "应用流程层", body: "智能助理 · 工作流编排 · 系统集成" },
          { title: "数据模型层", body: "可信知识 · 模型组合 · 数据权限" },
          { title: "治理安全层", body: "隐私安全 · 质量监控 · 审计追溯", accent: true },
        ], takeaway: "前台追求速度，后台提供可复制的安全与规模。",
      },
      { role: "closing", title: "现在行动，定义未来", statement: "让 AI 成为组织的能力放大器", footer: "从一个可验证的工作流开始。" },
    ],
  });
  assert.equal(commits, 1, "the full deck must be one atomic replacement");
  assert.equal(result.slideIds.length, 6);
  assert.equal(result.quality.passed, true, JSON.stringify(result.quality.findings));
  assert.ok(result.quality.score >= 80);
  assert.equal(deck.extensions["org.unippt.ai"].designTokens.generator, "executive-composer-v1");
  assert.equal(deck.extensions["org.unippt.ai"].designTokens.preset, "midnight-consulting");
  assert.equal(deck.slides[1].objects.some((object) => object.kind === "chart"), true);
  assert.match(deck.slides[1].notes, /\[Sources\][\s\S]*Stanford HAI 2025[\s\S]*\[\/Sources\]/);
  assert.equal(deck.slides.every((slide) => slide.objects.every((object) => object.sourceShapeId === null)), true);
  assert.equal(deck.slides[0].objects.find((object) => object.name === "cover-title").textStyle.fontSize, 54);
  assert.equal(deck.slides[2].objects.find((object) => object.name === "slide-title").textStyle.fontSize, 36);
  assert.equal(api.document.validate().filter((issue) => issue.severity === "error").length, 0);
});

test("executive quality audit rejects unsourced metrics and overlong titles", () => {
  const host = runtime();
  const deck = host.composeExecutiveDeck({
    title: "审计",
    audience: "管理层",
    purpose: "测试",
    centralTakeaway: "测试",
    slides: [
      { role: "cover", title: "审计演示" },
      { role: "big-number", title: "这是一个故意写得非常非常非常非常非常非常长并且不适合单页表达的页面标题", metric: { value: "99%", label: "没有来源的指标" } },
    ],
  }, { idFactory: idFactory() });
  const quality = host.auditDeckQuality(deck);
  assert.equal(quality.passed, false);
  assert.ok(quality.findings.some((finding) => finding.code === "TITLE_TOO_LONG"));
  assert.ok(quality.findings.some((finding) => finding.code === "EVIDENCE_SOURCE_MISSING"));

  deck.slides[1].notes = "[Sources]\n- No external sources; authored synthesis.\n[/Sources]";
  const placeholderQuality = host.auditDeckQuality(deck);
  assert.ok(placeholderQuality.findings.some((finding) => finding.code === "EVIDENCE_SOURCE_MISSING"), "a placeholder source must not authorize a metric");
});

test("presentation.importFreeHtml replaces the deck once and preserves the compiler report", async () => {
  const converter = {
    convert: async (source, options) => {
      assert.match(source, /omnidoc-page/);
      assert.equal(options.sourceName, "自由稿.html");
      return {
        title: "自由稿", width: 1280, height: 720,
        slides: [{
          name: "HTML 页面 1", background: "#0a1128", notes: "[HTML Import]",
          objects: [
            { kind: "shape", name: "HTML 形状", frame: { x: 0, y: 0, width: 1280, height: 720 }, style: { fill: "#0a1128" } },
            { kind: "text", name: "HTML 文本", text: "浏览器排版是真实布局", frame: { x: 80, y: 90, width: 800, height: 80 }, textStyle: { fontSize: 42, color: "#fff" } },
          ],
        }],
        report: { pageCount: 1, nativeText: 1, nativeShapes: 1, preservedImages: 0, fidelityRisk: "low", editabilityRatio: 100 },
      };
    },
  };
  const runtimeApi = runtime({ UniPptHtmlToPpt: converter });
  let deck = deckFixture();
  let revision = 0;
  let commits = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; commits += 1; },
    getSelection: () => ({ slideId: deck.slides[0].id, objectId: null }),
  }, { idFactory: idFactory() });
  const definition = api.ai.tools.find((tool) => tool.name === "presentation.importFreeHtml");
  assert.match(definition.description, /可编辑 PPT/);
  assert.deepEqual(Array.from(definition.inputSchema.required), ["attachmentName"]);

  const result = await api.ai.callTool("presentation.importFreeHtml", {
    attachmentName: "自由稿.html", sourceHtml: '<section class="omnidoc-page">content</section>',
  });
  assert.equal(commits, 1);
  assert.equal(result.report.editabilityRatio, 100);
  assert.equal(deck.extensions["org.unippt.ai"].designTokens.generator, "html-dom-compiler-v1");
  assert.equal(deck.extensions["org.unippt.ai"].importReport.fidelityRisk, "low");
  assert.equal(deck.slides[0].objects[1].text, "浏览器排版是真实布局");
  assert.equal(deck.slides[0].objects[1].sourceShapeId, null);
});

test("precompiled AI HTML opens one independent document without mutating the parent deck", async () => {
  const converter = {
    convert: async () => ({
      title: "新窗口整稿", width: 1280, height: 720,
      slides: [
        {
          name: "新窗口整稿", background: "#071426",
          notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]",
          semantic: { role: "cover", summary: "新窗口整稿" },
          objects: [{ kind: "text", name: "cover-title", text: "新窗口整稿", frame: { x: 80, y: 160, width: 900, height: 100 }, textStyle: { fontSize: 56, color: "#fff" } }],
        },
        {
          name: "开始行动", background: "#f5f2ea",
          notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]",
          semantic: { role: "closing", summary: "开始行动" },
          objects: [{ kind: "text", name: "closing-title", text: "开始行动", frame: { x: 80, y: 160, width: 900, height: 100 }, textStyle: { fontSize: 44, color: "#111" } }],
        },
      ],
      report: { pageCount: 2, nativeText: 2, nativeShapes: 0, preservedImages: 0, localizedFallbacks: 0, unsupported: [], fidelityRisk: "low", editabilityRatio: 100 },
    }),
  };
  const host = secureRuntime({ UniPptHtmlToPpt: converter });
  const parentDeck = deckFixture();
  let commits = 0, opens = 0, openedDeck = null, authorizer = null;
  const destination = { kind: "new-window", publish() {} };
  const api = host.create({
    getDeck: () => parentDeck,
    getRevision: () => 9,
    commit: () => { commits += 1; },
    openDocument: (deck, _outcome, options) => {
      opens += 1;
      openedDeck = deck;
      assert.equal(options.destination, destination);
    },
    getSelection: () => ({ slideId: "slide-1", objectId: null }),
    ai: { bindMutationAuthorizer(value) { authorizer = value; } },
  }, { idFactory: idFactory() });

  const prepared = await api.ai.prepareFreeHtml({
    attachmentName: "新窗口整稿.html", sourceHtml: "<!doctype html><html><body>generated</body></html>",
    generatedByAi: true, pageCount: 2,
  });
  assert.equal(commits, 0);
  assert.equal(opens, 0);
  const capability = authorizer.issue([{ name: "presentation.importFreeHtml" }]);
  const result = await api.ai.callTool("presentation.importFreeHtml", { preparedId: prepared.preparedId }, { capability, destination });

  assert.equal(commits, 0, "the parent document must never be replaced");
  assert.equal(opens, 1);
  assert.equal(parentDeck.title, "架构测试");
  assert.equal(openedDeck.title, "新窗口整稿");
  assert.equal(result.destination, "new-window");
  assert.equal(result.revision, 0);
});

test("AI-authored free HTML becomes one strict semantic deck replacement", async () => {
  let convertOptions = null;
  let commitOptions = null;
  const converter = {
    convert: async (_source, options) => {
      convertOptions = options;
      return {
        title: "AI 工作流革命", width: 1280, height: 720,
        slides: [
          {
            name: "AI 工作流革命", background: "#071426",
            notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]\n\n[HTML Import]\n[/HTML Import]",
            semantic: { role: "cover", summary: "AI 工作流革命" },
            objects: [
              { kind: "shape", name: "HTML 形状 · cover-background", frame: { x: 0, y: 0, width: 1280, height: 720 }, style: { fill: "#071426" } },
              { kind: "text", name: "HTML 文本 · cover-title", text: "AI 工作流革命", frame: { x: 80, y: 180, width: 1000, height: 100 }, textStyle: { fontSize: 56, color: "#fff" } },
            ],
          },
          {
            name: "从一个工作流开始", background: "#f5f2ea",
            notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]\n\n[HTML Import]\n[/HTML Import]",
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
  const runtimeApi = runtime({ UniPptHtmlToPpt: converter });
  let deck = deckFixture();
  const startingDeck = deck;
  let revision = 7;
  let commits = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next, _outcome, options) => { deck = next; revision += 1; commits += 1; commitOptions = options; },
    getSelection: () => ({ slideId: deck.slides[0].id, objectId: null }),
  }, { idFactory: idFactory() });

  const result = await api.ai.callTool("presentation.importFreeHtml", {
    attachmentName: "AI 工作流革命.html",
    sourceHtml: "<!doctype html><html><body>generated</body></html>",
    generatedByAi: true,
    audience: "企业管理层",
    purpose: "推动首个 AI 工作流试点",
    centralTakeaway: "优势来自工作流重构",
    pageCount: 2,
  });

  assert.equal(convertOptions.restrictNetwork, true);
  assert.equal(commits, 1, "the generated deck must commit exactly once");
  assert.equal(commitOptions.replace, true);
  assert.equal(commitOptions.expectedRevision, 7);
  assert.equal(commitOptions.expectedDeckRef, startingDeck);
  assert.deepEqual(Array.from(result.applied), ["composeFromHtml"]);
  assert.equal(result.quality.passed, true, JSON.stringify(result.quality.findings));
  assert.equal(deck.extensions["org.unippt.ai"].designTokens.generator, "ai-html-composer-v1");
  assert.equal(deck.extensions["org.unippt.ai"].document.audience, "企业管理层");
  assert.equal(deck.extensions["org.unippt.ai"].slides[deck.slides[0].id].role, "cover");
  assert.equal(deck.extensions["org.unippt.ai"].slides[deck.slides[1].id].role, "closing");
  assert.match(deck.slides[0].notes, /\[Sources\]/);
});

test("AI HTML quality failure and revision conflict never replace the active deck", async () => {
  let revision = 3;
  let commits = 0;
  let deck = deckFixture();
  const original = deck;
  const missingTitle = runtime({
    UniPptHtmlToPpt: {
      convert: async () => ({
        title: "不合格", width: 1280, height: 720,
        slides: [{ name: "缺标题", background: "#fff", notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]", semantic: { role: "cover" }, objects: [] }],
        report: { pageCount: 1, nativeText: 0, nativeShapes: 0, preservedImages: 0, unsupported: [] },
      }),
    },
  }).create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: () => { commits += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  await assert.rejects(
    missingTitle.ai.callTool("presentation.importFreeHtml", { attachmentName: "bad.html", sourceHtml: "<html></html>", generatedByAi: true }),
    (error) => error.code === "AI_HTML_QUALITY_FAILED",
  );
  assert.equal(commits, 0);
  assert.equal(deck, original);

  const warningOnly = runtime({
    UniPptHtmlToPpt: {
      convert: async () => ({
        title: "低分稿", width: 1280, height: 720,
        slides: Array.from({ length: 6 }, (_, index) => ({
          name: `第 ${index + 1} 页`, background: "#fff",
          notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]",
          semantic: { role: index === 0 ? "cover" : index === 5 ? "closing" : "content" },
          objects: [{ kind: "text", name: `HTML 文本 · ${index === 0 ? "cover-title" : index === 5 ? "closing-title" : "slide-title"}`, text: `第 ${index + 1} 页结论`, frame: { x: 80, y: 80, width: 800, height: 80 }, textStyle: { fontSize: 12, color: "#111" } }],
        })),
        report: { pageCount: 6, nativeText: 6, nativeShapes: 0, preservedImages: 0, unsupported: [], fidelityRisk: "low" },
      }),
    },
  }).create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: () => { commits += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  await assert.rejects(
    warningOnly.ai.callTool("presentation.importFreeHtml", { attachmentName: "low-score.html", sourceHtml: "<html></html>", generatedByAi: true, pageCount: 6 }),
    (error) => error.code === "AI_HTML_QUALITY_FAILED" && error.details.every((finding) => finding.severity !== "error"),
  );
  assert.equal(commits, 0);

  const revisionConflict = runtime({
    UniPptHtmlToPpt: {
      convert: async () => {
        revision += 1;
        return {
          title: "并发冲突", width: 1280, height: 720,
          slides: [{
            name: "封面", background: "#fff", notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]", semantic: { role: "cover" },
            objects: [{ kind: "text", name: "HTML 文本 · cover-title", text: "并发冲突", frame: { x: 80, y: 80, width: 800, height: 100 }, textStyle: { fontSize: 56, color: "#111" } }],
          }],
          report: { pageCount: 1, nativeText: 1, nativeShapes: 0, preservedImages: 0, unsupported: [] },
        };
      },
    },
  }).create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: () => { commits += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  await assert.rejects(
    revisionConflict.ai.callTool("presentation.importFreeHtml", { attachmentName: "conflict.html", sourceHtml: "<html></html>", generatedByAi: true }),
    (error) => error.code === "REVISION_CONFLICT",
  );
  assert.equal(commits, 0);
  assert.equal(deck, original);

  const pageMismatch = runtime({
    UniPptHtmlToPpt: {
      convert: async () => ({
        title: "少页", width: 1280, height: 720,
        slides: [{ name: "只有一页", background: "#fff", notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]", objects: [] }],
        report: { pageCount: 1, nativeText: 0, nativeShapes: 0, preservedImages: 0, unsupported: [] },
      }),
    },
  }).create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: () => { commits += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  await assert.rejects(
    pageMismatch.ai.callTool("presentation.importFreeHtml", { attachmentName: "short.html", sourceHtml: "<html></html>", generatedByAi: true, pageCount: 2 }),
    (error) => error.code === "AI_HTML_PAGE_COUNT_MISMATCH",
  );
  assert.equal(commits, 0);
});

test("AI HTML import rejects a switched deck even when its reset revision still matches", async () => {
  let revision = 0;
  let commits = 0;
  let deck = deckFixture();
  const original = deck;
  const replacement = deckFixture();
  replacement.title = "用户刚打开的另一份文稿";
  const api = runtime({
    UniPptHtmlToPpt: {
      convert: async () => {
        deck = replacement;
        return {
          title: "陈旧的 AI 结果", width: 1280, height: 720,
          slides: [{
            name: "封面", background: "#fff",
            notes: "[Sources]\n- No external sources; authored synthesis.\n[/Sources]",
            semantic: { role: "cover" },
            objects: [{
              kind: "text", name: "HTML 文本 · cover-title", text: "陈旧的 AI 结果",
              frame: { x: 80, y: 80, width: 800, height: 100 },
              textStyle: { fontSize: 56, color: "#111" },
            }],
          }],
          report: { pageCount: 1, nativeText: 1, nativeShapes: 0, preservedImages: 0, unsupported: [], fidelityRisk: "low" },
        };
      },
    },
  }).create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: () => { commits += 1; revision += 1; },
    getSelection: () => ({ slideId: deck.slides[0].id }),
  }, { idFactory: idFactory() });

  await assert.rejects(
    api.ai.callTool("presentation.importFreeHtml", {
      attachmentName: "stale.html", sourceHtml: "<html></html>", generatedByAi: true, pageCount: 1,
    }),
    (error) => error.code === "REVISION_CONFLICT" && /目标文稿已经切换/.test(error.message),
  );
  assert.equal(commits, 0);
  assert.equal(deck, replacement);
  assert.notEqual(deck, original);
  assert.equal(revision, 0, "切换文稿后 revision 重置为相同值也必须被对象身份锁拦截");
});

test("image reconstruction is one atomic append-only transaction with editable layers", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  let commits = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; commits += 1; },
    getSelection: () => ({ slideId: "slide-1", objectId: null }),
  }, { idFactory: idFactory() });
  const definition = api.ai.tools.find((tool) => tool.name === "slide.reconstructFromImage");
  assert.match(definition.description, /只新增一页/);
  assert.ok(definition.inputSchema.properties.slide.properties.occlusions);
  assert.ok(definition.inputSchema.properties.slide.properties.sourceCrops);
  assert.ok(definition.inputSchema.properties.slide.properties.autoMaskText);

  const result = await api.ai.callTool("slide.reconstructFromImage", {
    attachmentName: "峰会.png",
    sourceImage: "data:image/png;base64,AAAA",
    slide: {
      name: "智启未来",
      background: "#020817",
      sourceCrops: [{ name: "重复地球裁剪", frame: { x: 640, y: 100, width: 560, height: 520 }, crop: { left: .45, top: .05, right: .02, bottom: .04 } }],
      occlusions: [{ name: "标题遮罩", frame: { x: 40, y: 60, width: 600, height: 250 }, style: { fill: "#020817" } }],
      objects: [{ kind: "text", name: "主标题", text: "智启未来", frame: { x: 60, y: 80, width: 540, height: 160 }, textStyle: { fontSize: 72, bold: true, color: "#ffffff" } }],
    },
  });
  assert.equal(commits, 1);
  assert.equal(revision, 1);
  assert.equal(result.preservedSlideCount, 1);
  assert.equal(result.editableObjectCount, 1);
  assert.equal(result.suppressedSourceCropCount, 1);
  assert.equal(deck.slides.length, 2);
  assert.equal(deck.slides[0].id, "slide-1", "existing slides must be preserved");
  assert.equal(deck.slides[1].objects[0].kind, "image");
  assert.equal(deck.slides[1].objects[0].asset, "data:image/png;base64,AAAA");
  assert.equal(deck.slides[1].objects[1].kind, "shape");
  assert.match(deck.slides[1].objects[1].name, /文字击穿遮罩/);
  assert.equal(deck.slides[1].objects[1].style.opacity, 1);
  assert.equal(deck.slides[1].objects[2].text, "智启未来");
  assert.equal(deck.slides[1].objects[2].textStyle.color, "#ffffff");
  assert.equal(deck.slides[1].objects[2].textStyle.fontSize, 72);
  assert.equal(deck.slides[1].objects[2].textParagraphs[0].runs[0].baseline, "normal");
  assert.equal(deck.slides[1].objects.some((object) => object.name === "重复地球裁剪"), false);
});

test("AI objects normalize explicit null text fields before export", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  await api.ai.callTool("slide.add", {
    afterSlideId: "slide-1",
    slide: {
      objects: [{
        kind: "text",
        text: "exportable",
        textStyle: { fontFamily: null, color: null, align: null },
        textParagraphs: [{
          align: null,
          runs: [{ text: "exportable", fontFamily: null, color: null, baseline: null }],
        }],
      }],
    },
  });
  const object = deck.slides[1].objects[0];
  assert.equal(object.textStyle.fontFamily, "Aptos, Microsoft YaHei, sans-serif");
  assert.equal(object.textStyle.color, "#172033");
  assert.equal(object.textStyle.align, "left");
  assert.equal(object.textParagraphs[0].align, "left");
  assert.equal(object.textParagraphs[0].runs[0].fontFamily, object.textStyle.fontFamily);
  assert.equal(object.textParagraphs[0].runs[0].color, object.textStyle.color);
  assert.equal(object.textParagraphs[0].runs[0].baseline, "normal");
});

test("source crops remain ordinary movable image objects only without a full source layer", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  const result = await api.ai.callTool("slide.reconstructFromImage", {
    attachmentName: "source.png",
    sourceImage: "data:image/png;base64,AAAA",
    slide: {
      name: "独立裁剪层",
      preserveSourceImage: false,
      sourceCrops: [{ name: "可移动地球", frame: { x: 640, y: 100, width: 520, height: 480 }, crop: { left: .4, top: .02, right: .01, bottom: .03 } }],
      objects: [{ kind: "text", text: "可编辑文字", frame: { x: 40, y: 40, width: 300, height: 80 } }],
    },
  });
  assert.equal(result.suppressedSourceCropCount, 0);
  const images = deck.slides[1].objects.filter((object) => object.kind === "image");
  assert.equal(images.length, 1);
  assert.equal(images[0].name, "可移动地球");
  assert.equal(images[0].frame.x, 640);
  assert.equal(images[0].frame.y, 100);
  assert.equal(images[0].frame.width, 520);
  assert.equal(images[0].frame.height, 480);
  assert.equal(images[0].frame.rotation, 0);
  assert.equal(images[0].imageCrop.left, .4);
});

test("image reconstruction forces opaque masks even when the model requests transparency", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  await api.ai.callTool("slide.reconstructFromImage", {
    attachmentName: "source.png",
    sourceImage: "data:image/png;base64,AAAA",
    slide: {
      name: "防重影",
      background: "#010712",
      autoMaskText: false,
      occlusions: [{ name: "模型遮罩", frame: { x: 10, y: 10, width: 400, height: 120 }, style: { fill: "#010712", opacity: .2 } }],
      objects: [{ kind: "text", name: "标题", text: "可编辑标题", frame: { x: 20, y: 20, width: 360, height: 90 } }],
    },
  });
  const masks = deck.slides[1].objects.filter((object) => object.kind === "shape");
  assert.equal(masks.length, 1);
  assert.ok(masks.every((mask) => mask.style.opacity === 1));
  const title = deck.slides[1].objects.find((object) => object.kind === "text");
  assert.equal(title.textStyle.color, "#ffffff");
  assert.equal(title.textStyle.fontSize, 63);
});

test("image reconstruction clamps OCR font size to its editable frame width", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  await api.ai.callTool("slide.reconstructFromImage", {
    attachmentName: "source.png",
    sourceImage: "data:image/png;base64,AAAA",
    slide: {
      name: "英文宽度",
      background: "#010712",
      objects: [{ kind: "text", text: "A I  I N N O V A T I O N  S U M M I T", frame: { x: 20, y: 20, width: 260, height: 50 }, textStyle: { fontSize: 36 } }],
    },
  });
  const text = deck.slides[1].objects.find((object) => object.kind === "text");
  assert.ok(text.textStyle.fontSize < 36);
  assert.ok(text.textStyle.fontSize >= 12);
});

test("dedicated OCR coordinates override model text frames and wide model masks", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  let revision = 0;
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => revision,
    commit: (next) => { deck = next; revision += 1; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  const result = await api.ai.callTool("slide.reconstructFromImage", {
    attachmentName: "poster.png",
    sourceImage: "data:image/png;base64,AAAA",
    ocrDetection: {
      image: { width: 1600, height: 900 },
      cleanedImage: "data:image/png;base64,CLEANED",
      cleaning: { strategy: "big-lama-roi-v1" },
      regions: [{
        text: "智启未来",
        location: [100, 120, 700, 120, 700, 280, 100, 280],
        background: "#020817",
        foreground: "#ffffff",
      }],
    },
    slide: {
      name: "OCR authority",
      background: "#020817",
      occlusions: [{ name: "bad-wide-mask", frame: { x: 0, y: 0, width: 1000, height: 500 }, style: { fill: "#000000" } }],
      objects: [{ kind: "text", text: "智启未来", frame: { x: 800, y: 500, width: 300, height: 80 }, textStyle: { color: "#00ffff" } }],
    },
  });
  assert.equal(result.ocrRegionCount, 1);
  const reconstructed = deck.slides[1];
  const masks = reconstructed.objects.filter((object) => object.kind === "shape");
  assert.equal(masks.length, 0, "a neural text-free background must not receive overlay masks");
  assert.equal(reconstructed.objects[0].asset, "data:image/png;base64,CLEANED");
  const text = reconstructed.objects.find((object) => object.kind === "text");
  assert.equal(text.text, "智启未来");
  assert.ok(text.frame.x < 200, "OCR coordinates must replace the model's guessed x position");
  assert.ok(text.frame.y < 150, "OCR coordinates must replace the model's guessed y position");
  assert.equal(text.textStyle.color, "#ffffff", "OCR-derived foreground color is authoritative");
  assert.ok(Math.abs(text.textStyle.fontSize - 110.08) < .01, "font size must match the standalone OCR geometry fit");
  assert.equal(text.textFrame.autoSize, "none");
});

test("native reconstruction preserves visual corrections, missing rotated text, subscripts and aspect ratio", async () => {
  let deck = deckFixture();
  const api = runtime().create({ getDeck: () => deck, getRevision: () => 0, commit: (next) => { deck = next; } }, { idFactory: idFactory() });
  const result = await api.ai.callTool("slide.reconstructFromImage", {
    attachmentName: "figure.png", sourceImage: "data:image/png;base64,AAAA", sourceSize: { width: 1000, height: 500 },
    ocrDetection: { regions: [{ text: "70", location: [200,100,230,100,230,120,200,120] }, { text: "Encoder", location: [300,100,380,100,380,120,300,120] }] },
    slide: { mode: "native", name: "native", verification: { labels: ["Step 1: Prior Generation", "z0", "Encoder"], rotatedTextCount: 1, diagram: true }, objects: [
      { kind: "text", text: "Step 1: Prior Generation", frame: { x: -80, y: 130, width: 220, height: 20, rotation: -90 }, textStyle: { fontSize: 17 } },
      { kind: "text", ocrIndex: 0, frame: { x: 200, y: 100, width: 30, height: 20 }, textRuns: [{ text: "z" }, { text: "0", baseline: "sub" }] },
      { kind: "shape", frame: { x: 400, y: 100, width: 30, height: 100 }, points: [[0,20],[30,0],[30,80],[0,100]], repeat: { count: 3, dx: 10, dy: -5 } },
      { kind: "connector", geometry: "line", frame: { x: 450, y: 140, width: 60, height: 1 }, style: { stroke: "#000000", strokeWidth: 1 } },
    ] },
  });
  const objects = deck.slides[1].objects;
  assert.equal(objects.filter((o) => o.kind === "image").length, 0, "no full-page raster background");
  assert.equal(result.report.nativeText, 3);
  assert.equal(result.report.rotatedText, 1);
  assert.equal(result.report.nativeShapes, 3);
  assert.equal(result.report.nativeConnectors, 1);
  assert.equal(result.report.fallbackTextCount, 1);
  const text = objects.find((o) => o.text === "z0");
  assert.equal(text.textParagraphs[0].runs[1].baseline, "sub");
  assert.ok(text.textParagraphs[0].runs[1].fontSize < text.textParagraphs[0].runs[0].fontSize);
  assert.equal(text.textFrame.wordWrap, false);
  assert.equal(text.textFrame.marginLeft, 0);
  assert.equal(text.frame.x, 256);
  assert.equal(text.frame.y, 168, "same scale on both axes plus letterbox offset");
  assert.equal(objects.find((o) => o.text.startsWith("Step")).frame.rotation, -90);
});

test("native reconstruction refuses a missing visual inventory or unaccounted rotated label atomically", async () => {
  let commits = 0;
  const deck = deckFixture();
  const api = runtime().create({ getDeck: () => deck, getRevision: () => 0, commit: () => commits++ }, { idFactory: idFactory() });
  const args = { sourceImage: "data:image/png;base64,AAAA", sourceSize: { width: 1000, height: 500 }, slide: { mode: "native", objects: [{ kind: "text", text: "title", frame: { x: 10, y: 10, width: 100, height: 30 } }] } };
  await assert.rejects(api.ai.callTool("slide.reconstructFromImage", args), /verification/);
  args.slide.verification = { labels: ["missing rotated label"], rotatedTextCount: 1, diagram: true };
  await assert.rejects(api.ai.callTool("slide.reconstructFromImage", args), /覆盖检查失败/);
  assert.equal(commits, 0);
});

test("OCR reconstruction refuses an unavailable cleaned background", async () => {
  const runtimeApi = runtime();
  let deck = deckFixture();
  const api = runtimeApi.create({
    getDeck: () => deck,
    getRevision: () => 0,
    commit: (next) => { deck = next; },
    getSelection: () => ({ slideId: "slide-1" }),
  }, { idFactory: idFactory() });
  await assert.rejects(api.ai.callTool("slide.reconstructFromImage", {
    attachmentName: "poster.png",
    sourceImage: "data:image/png;base64,AAAA",
    ocrDetection: {
      image: { width: 1600, height: 900 },
      regions: [{ text: "智启未来", location: [100, 120, 700, 120, 700, 280, 100, 280] }],
      cleaning: { strategy: "directional-interpolation-v1-fallback", fallbackReason: "clean-image timeout" },
    },
    slide: { name: "blocked", objects: [] },
  }), /已阻止生成低质量色块遮罩/);
  assert.equal(deck.slides.length, 1);
});
