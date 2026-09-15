"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const htmlSource = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const cssSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

const editingStart = appSource.indexOf("function replaceRichTextContent");
const editingEnd = appSource.indexOf("\nfunction selectObject", editingStart);
assert.ok(editingStart >= 0 && editingEnd > editingStart, "rich-text reconciliation should be extractable");
const reconciliationContext = {};
vm.runInNewContext(
  `${appSource.slice(editingStart, editingEnd)}\nthis.api = { replaceRichTextContent };`,
  reconciliationContext,
);

const richObject = {
  text: "Title\nBody",
  textStyle: { fontFamily: "Aptos", fontSize: 30, color: "#123456", align: "left" },
  textParagraphs: [
    {
      align: "center",
      level: 0,
      bullet: null,
      sourceIndex: 3,
      runs: [{ text: "Title", bold: true, fontFamily: "Aptos Display", sourceIndex: 7 }],
    },
    {
      align: "left",
      level: 1,
      bullet: "•",
      sourceIndex: 4,
      runs: [{ text: "Body", italic: true, sourceIndex: 8 }],
    },
  ],
};

reconciliationContext.api.replaceRichTextContent(richObject, "Edited title\r\nBody");
assert.equal(richObject.text, "Edited title\nBody", "scene text must receive normalized edited text");
assert.equal(richObject.textParagraphs[0].runs[0].text, "Edited title");
assert.equal(richObject.textParagraphs[0].runs[0].bold, true, "edited paragraph keeps its first run style");
assert.equal(richObject.textParagraphs[0].sourceIndex, 3, "edited paragraph keeps its native source identity");
assert.equal(richObject.textParagraphs[1].runs[0].text, "Body");
assert.equal(richObject.textParagraphs[1].runs[0].italic, true, "untouched paragraph keeps its run formatting");
assert.equal(richObject.textParagraphs[1].bullet, "•", "bullet metadata stays structural instead of entering text");

const selectStart = appSource.indexOf("function selectObject");
const selectEnd = appSource.indexOf("\nfunction renderInspector", selectStart);
const selectSource = appSource.slice(selectStart, selectEnd);
assert.doesNotMatch(selectSource, /formatPaneOpen\s*=\s*Boolean/, "single click must not auto-open Shape Format");
assert.doesNotMatch(selectSource, /renderCanvas\s*\(/, "selection must not replace the node between the two clicks of a double-click");
assert.match(selectSource, /syncCanvasSelection\s*\(/, "selection handles should update without a canvas rebuild");

const editStart = appSource.indexOf("function editText");
const editEnd = appSource.indexOf("\nfunction editingContentForObject", editStart);
const editSource = appSource.slice(editStart, editEnd);
assert.match(editSource, /\.object-content, \.rich-text/, "plain and imported rich text must both be editable");
assert.match(editSource, /content\.contentEditable\s*=\s*"true"/, "double-click must enter native editing");
assert.doesNotMatch(editSource, /content\.textContent\s*=\s*object\.text/, "entering rich-text editing must not flatten styled runs");
assert.match(editSource, /replaceRichTextContent\(object, nextText\)/, "blur must write text back to the scene model");
assert.match(editSource, /keyEvent\.key\s*!==\s*"Escape"/, "Escape must cancel editing");
assert.doesNotMatch(editSource, /event\.preventDefault\(\)/, "entering text editing must preserve the browser's native drag-selection gesture");
assert.match(editSource, /pointerGesture[\s\S]*beginTextDragSelect\(content, event\)/, "the entering pointerdown must start a drag-selection from the press point instead of collapsing the caret");
assert.match(appSource, /function beginTextDragSelect/, "mid-glyph drags must extend Selection via caretRangeFromPoint, not object dragging");
assert.match(appSource, /setDirectionalTextSelection\(selection, origin, focus\)/, "drag-selection must retain the pressed anchor while the pointer becomes the focus");
assert.match(appSource, /target instanceof Element \? target : target\?\.parentElement/, "selectstart must treat a Text node inside the editor as an editing target");

const directionalStart = appSource.indexOf("function setDirectionalTextSelection");
const directionalEnd = appSource.indexOf("\nfunction beginTextDragSelect", directionalStart);
assert.ok(directionalStart >= 0 && directionalEnd > directionalStart, "directional Selection helper should be extractable");
const directionalContext = {};
vm.runInNewContext(
  `${appSource.slice(directionalStart, directionalEnd)}\nthis.api = { setDirectionalTextSelection };`,
  directionalContext,
);
const anchorNode = { textContent: "岗位竞聘" };
const focusNode = { textContent: "岗位竞聘" };
const calls = [];
directionalContext.api.setDirectionalTextSelection(
  { setBaseAndExtent(...args) { calls.push(args); } },
  { startContainer: anchorNode, startOffset: 3 },
  { startContainer: focusNode, startOffset: 1 },
);
assert.deepEqual(calls[0], [anchorNode, 3, focusNode, 1], "a right-to-left drag must keep its later anchor and earlier focus instead of normalizing forward");

const rangeFormattingStart = appSource.indexOf("function richTextRunsInRange");
const rangeFormattingEnd = appSource.indexOf("\nfunction textBoundaryAtOffset", rangeFormattingStart);
assert.ok(rangeFormattingStart >= 0 && rangeFormattingEnd > rangeFormattingStart, "run-range formatter should be extractable");
const rangeFormattingContext = {};
vm.runInNewContext(
  `${appSource.slice(rangeFormattingStart, rangeFormattingEnd)}\nthis.api = { richTextRunsInRange, updateRichTextRange };`,
  rangeFormattingContext,
);
const formattedParagraphs = [{
  runs: [{ text: "abcdef", bold: false, color: "#123456", sourceIndex: 7 }],
}];
assert.equal(rangeFormattingContext.api.richTextRunsInRange(formattedParagraphs, 2, 4).length, 1);
assert.equal(rangeFormattingContext.api.updateRichTextRange(formattedParagraphs, 2, 4, "bold", true), true);
assert.deepEqual(
  JSON.parse(JSON.stringify(formattedParagraphs[0].runs)),
  [
    { text: "ab", bold: false, color: "#123456", sourceIndex: 7 },
    { text: "cd", bold: true, color: "#123456", sourceIndex: null },
    { text: "ef", bold: false, color: "#123456", sourceIndex: null },
  ],
  "formatting a selected substring must split its native run and leave surrounding text unchanged",
);
assert.equal(new Set(formattedParagraphs[0].runs.map((run) => run.sourceIndex).filter((value) => value != null)).size, 1, "a split run must retain its native source index only once");

assert.match(appSource, /function preserveTextSelectionForCommand[\s\S]*?event\.preventDefault\(\)/, "toolbar pointerdown must preserve the browser Selection before the button receives focus");
assert.match(appSource, /function toggleSelectedStyle\(property\)[\s\S]*?applySelectedTextProperty/, "bold and italic must prefer the active character range over the whole object");
assert.match(appSource, /span\.dataset\.textStart[\s\S]*?span\.dataset\.textEnd/, "rendered runs must expose stable model offsets for DOM Selection mapping");
assert.match(appSource, /richTextModelChanged/, "range formatting must keep the text-edit history checkpoint instead of discarding it on blur");

assert.match(appSource, /addEventListener\("contextmenu"[\s\S]*?openFormatPane\(interactionId\)/, "right-click explicitly opens Shape Format for the effective object or group");
assert.match(appSource, /node\.querySelector\(":scope > \.object-content, :scope > \.rich-text"\)[\s\S]*?state\.selectedId === object\.id[\s\S]*?editText\(event, object\.id\)/, "one click inside an already-selected text box must move the caret and enter editing even when its visual content ignores pointer events");
assert.match(appSource, /addEventListener\("pointerdown"[\s\S]*?node\.classList\.contains\("editing"\)[\s\S]*?beginTextDragSelect[\s\S]*?beginDrag\(event, interactionId\)/, "pointer gestures inside an editing text box must drag-select text instead of moving the effective object or group");
assert.match(appSource, /window\.getSelection\?\.\(\) \|\| document\.getSelection\?\.\(\)/, "caret placement should use the cross-browser Selection API without assuming document.getSelection exists");
assert.match(htmlSource, /id="openFormatPane"/, "the ribbon exposes an explicit Shape Format command");
assert.match(cssSource, /\.format-pane\s*\{[^}]*position:absolute[^}]*width:260px/s, "Shape Format must overlay instead of compressing the canvas");
const selectedRule = cssSource.match(/\.scene-object\.selected\s*\{([^}]*)\}/s)?.[1] || "";
assert.doesNotMatch(selectedRule, /z-index/i, "selection must not change native PPT stacking order");
assert.match(cssSource, /\.scene-object\.selected:not\(\.editing\):has\(> \.object-content\)\s*\{[^}]*cursor:text/s, "selected text boxes should advertise one-click caret placement");
assert.match(cssSource, /\.scene-object\.selected\s*>\s*\.object-content[^{}]*\{[^}]*pointer-events\s*:\s*auto/s, "selected plain text must receive the pointerdown that begins native drag selection");
assert.match(cssSource, /\.slide-stage\s+\.scene-object\.editing\s+\*[^{}]*\{[^}]*user-select\s*:\s*text/s, "glyphs inside an editing box must be selectable even when the slide stage disables selection");
assert.match(cssSource, /\.scene-object\.editing\s*\{[^}]*touch-action\s*:\s*auto/s, "an editing text box must not inherit touch-action:none or Chromium will refuse to start a text drag");

assert.match(appSource, /object\.kind === "math" && state\.selectedId === object\.id[\s\S]*?beginTextDragSelect\(node, event\)/, "a selected formula must route an in-canvas drag through native text selection");
assert.match(appSource, /const saved = state\.textSelection\?\.[\s\S]*?miniFormatContext\.range/, "toolbar commands must fall back to their captured range after a native control collapses Selection");
assert.match(appSource, /const finishOnBlur = \(\) => \{[\s\S]*?activeElement\?\.closest\?\.\("#miniFormatToolbar"\)[\s\S]*?content\.removeEventListener\("blur", finishOnBlur\)/, "focus transfer to the mini toolbar must not commit and destroy the live editor");
assert.match(cssSource, /\.scene-object\.kind-math\.selected[\s\S]*?user-select\s*:\s*text/, "selected formula glyphs must allow browser drag selection");

console.log("text editing interaction tests passed");
