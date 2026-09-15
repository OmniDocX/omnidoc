"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "ai_message_renderer.js"), "utf8");

function runtime() {
  const context = { console, encodeURIComponent, decodeURIComponent, setTimeout, clearTimeout };
  context.globalThis = context;
  vm.runInNewContext(source, context, { filename: "ai_message_renderer.js" });
  return context.UniPptAiMessageRenderer;
}

test("AI message renderer supports the UniDoc Markdown vocabulary", () => {
  const renderer = runtime();
  const html = renderer.markdownToHtml(`# 标题

**重点**、\`代码\`和 $E=mc^2$

- [x] 已完成
- 普通项目

| 功能 | 状态 |
| --- | --- |
| Markdown | 可用 |

> 引用

\`\`\`js
const answer = 42;
\`\`\``);
  assert.match(html, /<h1>标题<\/h1>/);
  assert.match(html, /<strong>重点<\/strong>/);
  assert.match(html, /<code>代码<\/code>/);
  assert.match(html, /class="ai-math ai-math-inline"/);
  assert.match(html, /task-list-item/);
  assert.match(html, /<table>/);
  assert.match(html, /<blockquote>/);
  assert.match(html, /<pre><code class="language-js">/);
});

test("AI message renderer escapes raw HTML and rejects executable URLs", () => {
  const renderer = runtime();
  const html = renderer.markdownToHtml(`<script>alert(1)</script>

[危险](javascript:alert(1)) [安全](https://example.com)`);
  assert.doesNotMatch(html, /<script>/);
  assert.match(html, /&lt;script&gt;/);
  assert.doesNotMatch(html, /href="javascript:/);
  assert.match(html, /href="https:\/\/example\.com"/);
  assert.match(html, /rel="noopener noreferrer"/);
});

test("AI message renderer preserves inline and display LaTeX for KaTeX hydration", () => {
  const renderer = runtime();
  const html = renderer.markdownToHtml(`行内：\\(a^2+b^2=c^2\\)

$$
\\int_0^1 x^2\\,dx
$$`);
  assert.match(html, /data-display="0"/);
  assert.match(html, /data-display="1"/);
  assert.match(html, /%5Cint_0%5E1/);
  assert.match(html, /\\int_0\^1/);
});

test("AI message renderer exposes only safe navigation protocols", () => {
  const renderer = runtime();
  assert.equal(renderer.safeUrl("https://example.com"), "https://example.com");
  assert.equal(renderer.safeUrl("mailto:team@example.com"), "mailto:team@example.com");
  assert.equal(renderer.safeUrl("javascript:alert(1)"), "");
  assert.equal(renderer.safeUrl("data:text/html,bad"), "");
});
