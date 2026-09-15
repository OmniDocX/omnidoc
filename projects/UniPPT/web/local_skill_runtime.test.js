"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "local_skill_runtime.js"), "utf8");

function runtime() {
  const context = { console };
  context.globalThis = context;
  vm.runInNewContext(source, context, { filename: "local_skill_runtime.js" });
  return context.UniPptLocalSkills;
}

function html(name = "deck.html") {
  return { kind: "html", name, html: "<!doctype html><main>可编辑内容</main>" };
}

test("whole-deck intent recognizes direct Chinese creation commands", () => {
  const skills = runtime();
  for (const prompt of [
    "给我创造一个关于AI演讲的超赞PPT",
    "帮我设计一份十页 AI 路演",
    "请直接做一个产品发布演示文稿",
    "能不能帮我做一份 AI 汇报 PPT",
    "来一套 12 页融资路演",
  ]) {
    assert.equal(skills.isWholeDeckCreationIntent(prompt), true, prompt);
  }
});

test("whole-deck intent keeps capability questions and planning requests conversational", () => {
  const skills = runtime();
  for (const prompt of [
    "你能制作 PPT 吗？",
    "如何制作一个关于 AI 的 PPT？",
    "请分析 Manus PPT 和我们的差异",
    "给我一个关于 AI 的 PPT 规划",
    "我们的 AI 助手能实现这种效果吗？",
  ]) {
    assert.equal(skills.isWholeDeckCreationIntent(prompt), false, prompt);
  }
});

test("negated creation and local repairs never replace the deck", () => {
  const skills = runtime();
  for (const prompt of ["不要重新生成幻灯片，只修正当前页面的文字。", "请只修改这一张幻灯片，不要新增页", "不要改写这张 PPT，只调整旋转文本", "Do not create a new PPT. Fix this text."]) assert.equal(skills.isWholeDeckCreationIntent(prompt), false, prompt);
  assert.equal(skills.isWholeDeckCreationIntent("不要修改旧稿，请新建一份十页 PPT"), true);
});

test("explicit free HTML conversion matches one deterministic local skill", () => {
  const skills = runtime();
  const plan = skills.match({
    prompt: "把这个自由 HTML 导入转成可编辑 PPTX",
    mode: "edit",
    attachments: [html()],
  });
  assert.equal(plan.id, "free-html-to-editable-ppt");
  assert.equal(plan.toolName, "presentation.importFreeHtml");
  assert.equal(plan.args.attachmentName, "deck.html");
  assert.equal(skills.match({ prompt: "总结这个 HTML", mode: "edit", attachments: [html()] }), null);
});

test("the local skill blocks chat mode and ambiguous multiple HTML attachments", () => {
  const skills = runtime();
  assert.match(skills.match({ prompt: "转成 PPT", mode: "chat", attachments: [html()] }).error, /编排演示/);
  assert.match(skills.match({
    prompt: "导入为 PPT",
    mode: "edit",
    attachments: [html("a.html"), html("b.html")],
  }).error, /一次只能.*1 个/);
});

test("the local skill asks once, calls one host tool and never needs a model channel", async () => {
  const skills = runtime();
  const plan = skills.match({ prompt: "将附件编译为可编辑演示文稿", mode: "edit", attachments: [html()] });
  let approvals = 0;
  let approvedRequest = null;
  const calls = [];
  const states = [];
  const outcome = await skills.run(plan, {
    beforeTools: async (requests) => { approvals += 1; assert.equal(requests.length, 1); [approvedRequest] = requests; return true; },
    callTool: async (name, args, request) => {
      assert.equal(request, approvedRequest, "the approved request must reach the capability-aware host caller");
      calls.push({ name, args });
      return { slideIds: ["slide-1", "slide-2"], report: { nativeText: 5, nativeShapes: 3, preservedImages: 2 } };
    },
    onToolState: (event) => states.push(event.phase),
  });
  assert.equal(approvals, 1);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].name, "presentation.importFreeHtml");
  assert.match(outcome.text, /2 页/);
  assert.match(outcome.text, /原生可编辑对象 8 个/);
  assert.deepEqual(states, ["requested", "completed"]);
});

test("the completion summary counts array-backed report metrics", async () => {
  const skills = runtime();
  const plan = skills.match({ prompt: "将附件导入为可编辑 PPT", mode: "edit", attachments: [html()] });
  const outcome = await skills.run(plan, {
    beforeTools: async () => true,
    callTool: async () => ({
      slideIds: ["slide-1"],
      report: {
        nativeText: 12,
        nativeShapes: 6,
        localizedFallbacks: 1,
        unsupported: [{ reason: "scripts-disabled" }],
      },
    }),
  });
  assert.match(outcome.text, /不支持项 1 个/);
  assert.doesNotMatch(outcome.text, /NaN/);
});

test("denial is non-mutating and reports cancellation", async () => {
  const skills = runtime();
  const plan = skills.match({ prompt: "HTML 转成 PPT", mode: "edit", attachments: [html()] });
  let calls = 0;
  const outcome = await skills.run(plan, {
    beforeTools: async () => false,
    callTool: async () => { calls += 1; },
  });
  assert.equal(calls, 0);
  assert.equal(outcome.denied, true);
  assert.match(outcome.text, /没有修改/);
});
