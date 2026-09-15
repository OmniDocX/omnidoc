"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const appSource = fs.readFileSync(path.join(__dirname, "app.js"), "utf8");
const htmlSource = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
const cssSource = fs.readFileSync(path.join(__dirname, "style.css"), "utf8");

assert.match(htmlSource, /id="officeSearchInput"[^>]*aria-controls="officeSearchResults"/, "the title-bar search controls a visible results popup");
assert.match(htmlSource, /id="officeSearchResults"[^>]*role="dialog"[^>]*hidden/, "search results start hidden and expose dialog semantics");
assert.match(htmlSource, /id="officeSearchResultList"[^>]*role="listbox"/, "matches are exposed as a selectable result list");
assert.match(htmlSource, /vendor\/material-symbols\/material-symbols-rounded\.css/, "icon fonts must be local");
assert.match(htmlSource, /math:\s*\{[\s\S]*?vendor\/katex\/katex\.min\.css/, "KaTeX styles must be a deferred local feature");
assert.match(htmlSource, /math:\s*\{[\s\S]*?vendor\/katex\/katex\.min\.js/, "KaTeX script must load independently after first paint");

assert.match(appSource, /officeSearchInput"\)\.oninput\s*=\s*\(\)\s*=>[\s\S]*updateOfficeSearchResults\(\)/, "typing refreshes matches immediately");
assert.match(appSource, /officeSearchInput"\)\.onkeydown\s*=\s*handleOfficeSearchKeydown/, "the search box supports keyboard navigation");
assert.match(appSource, /function handleOfficeSearchKeydown[\s\S]*'Enter'[\s\S]*'ArrowDown'[\s\S]*'ArrowUp'/, "Enter and arrow keys cycle search results");
assert.match(appSource, /event\.altKey\s*&&\s*key === "q"[\s\S]*officeSearchInput/, "Alt+Q focuses the title-bar search");
assert.match(appSource, /function navigateToTextMatch[\s\S]*state\.activeSlide = match\.slideIndex[\s\S]*state\.selectedId = match\.objectId[\s\S]*renderAll\(\)/, "opening a match navigates to and selects its slide object");
assert.match(appSource, /officeSearchResults[\s\S]*officeSearchWrap[\s\S]*closeOfficeSearchResults/, "clicking outside closes search results");

const findStart = appSource.indexOf("function findTextMatches");
const findEnd = appSource.indexOf("\nfunction updateFindResultCount", findStart);
assert.ok(findStart >= 0 && findEnd > findStart, "the text matcher should be extractable");
const context = {
  state: {
    deck: {
      slides: [
        { objects: [{ id: "title", text: "专业技能" }], notes: "" },
        { objects: [{ id: "body", text: "团队合作" }], notes: "补充专业经历" },
      ],
    },
  },
  flattenObjects(objects) { return objects; },
  $() { return { value: "" }; },
};
vm.runInNewContext(`${appSource.slice(findStart, findEnd)}\nthis.api = { findTextMatches };`, context);
const matches = context.api.findTextMatches("专业");
assert.equal(matches.length, 2, "search includes slide text and speaker notes");
assert.equal(
  JSON.stringify(matches.map(({ slideIndex, objectId, source }) => [slideIndex, objectId, source])),
  JSON.stringify([[0, "title", "slide"], [1, null, "notes"]]),
);

assert.match(cssSource, /\.office-search-wrap\s*\{[^}]*position:relative/s, "the popup is anchored to the title search box");
assert.match(cssSource, /\.office-search-results\s*\{[^}]*position:absolute[^}]*z-index:137/s, "results float over the ribbon and workspace");
assert.match(cssSource, /\.office-search-results\[hidden\]\s*\{[^}]*display:none/s, "hidden results cannot intercept pointer input");

console.log("office search tests passed");
