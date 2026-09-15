(function initUniPptAiMessageRenderer(global) {
  "use strict";

  const scheduled = new WeakMap();
  const mathSelector = ".ai-math[data-tex]";

  function string(value) {
    return value == null ? "" : String(value);
  }

  function escapeHtml(value) {
    return string(value).replace(/[&<>"']/g, (char) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;",
    })[char]);
  }

  function safeUrl(value) {
    const url = string(value).trim().replace(/[\u0000-\u001f\u007f]/g, "");
    if (!url) return "";
    if (/^(?:https?:|mailto:)/i.test(url) || /^(?:#|\/|\.\/|\.\.\/)/.test(url)) return url;
    return "";
  }

  function mathToken(tex, display, delimiters) {
    const source = string(tex).trim();
    const encoded = encodeURIComponent(source);
    const tag = display ? "div" : "span";
    const css = display ? "ai-math ai-math-block" : "ai-math ai-math-inline";
    return `<${tag} class="${css}" data-tex="${escapeHtml(encoded)}" data-display="${display ? "1" : "0"}">${escapeHtml(delimiters[0] + source + delimiters[1])}</${tag}>`;
  }

  function inlineMarkdown(value) {
    const stashed = [];
    const stash = (html) => {
      const token = `\u0002AI${stashed.length}\u0003`;
      stashed.push(html);
      return token;
    };
    let source = string(value);

    source = source.replace(/`([^`\n]+)`/g, (_all, code) => stash(`<code>${escapeHtml(code)}</code>`));
    source = source.replace(/\\\(([\s\S]+?)\\\)/g, (_all, tex) => stash(mathToken(tex, false, ["\\(", "\\)"])));
    source = source.replace(/\$([^$\n]+?)\$/g, (all, tex) => {
      if (!tex.trim() || /^\s|\s$/.test(tex)) return all;
      return stash(mathToken(tex, false, ["$", "$"]));
    });

    source = escapeHtml(source);
    source = source.replace(/!\[([^\]]*)\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g, (_all, alt, href) => {
      const url = safeUrl(href);
      if (!url || !/^https?:/i.test(url)) return `![${alt}](${href})`;
      return `<img class="ai-md-image" alt="${escapeHtml(alt)}" src="${escapeHtml(url)}" loading="lazy" referrerpolicy="no-referrer">`;
    });
    source = source.replace(/\[([^\]]+)\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g, (_all, label, href) => {
      const url = safeUrl(href);
      if (!url) return `${label} (${href})`;
      const external = /^https?:/i.test(url);
      return `<a href="${escapeHtml(url)}"${external ? ' target="_blank" rel="noopener noreferrer"' : ""}>${label}</a>`;
    });
    source = source
      .replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>")
      .replace(/__([^_\n]+)__/g, "<strong>$1</strong>")
      .replace(/~~([^~\n]+)~~/g, "<del>$1</del>")
      .replace(/(^|[^*\w])\*([^*\n]+)\*(?!\w)/g, "$1<em>$2</em>")
      .replace(/(^|[^_\w])_([^_\n]+)_(?!\w)/g, "$1<em>$2</em>");

    source = source.replace(/\u0002AI(\d+)\u0003/g, (_all, index) => stashed[Number(index)] || "");
    return source;
  }

  function splitTableRow(line) {
    return string(line).trim().replace(/^\|/, "").replace(/\|$/, "")
      .split(/(?<!\\)\|/).map((cell) => cell.trim().replace(/\\\|/g, "|"));
  }

  function markdownToHtml(markdown) {
    const lines = string(markdown).replace(/^\uFEFF/, "").replace(/\r\n?/g, "\n").split("\n");
    const out = [];
    let index = 0;
    const tableSeparator = (line) => /^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(line);

    while (index < lines.length) {
      const line = lines[index];
      const fence = line.match(/^\s*(`{3,}|~{3,})\s*([\w.+-]*)\s*$/);
      if (fence) {
        const marker = fence[1][0];
        const language = fence[2] ? ` class="language-${escapeHtml(fence[2])}"` : "";
        const body = [];
        index += 1;
        while (index < lines.length && !new RegExp(`^\\s*${marker}{${fence[1].length},}\\s*$`).test(lines[index])) body.push(lines[index++]);
        if (index < lines.length) index += 1;
        out.push(`<pre><code${language}>${escapeHtml(body.join("\n"))}</code></pre>`);
        continue;
      }

      if (/^\s*\$\$\s*$/.test(line) || /^\s*\\\[\s*$/.test(line)) {
        const dollar = /\$\$/.test(line);
        const closing = dollar ? /^\s*\$\$\s*$/ : /^\s*\\\]\s*$/;
        const body = [];
        index += 1;
        while (index < lines.length && !closing.test(lines[index])) body.push(lines[index++]);
        if (index < lines.length) index += 1;
        out.push(mathToken(body.join("\n"), true, dollar ? ["$$", "$$"] : ["\\[", "\\]"]));
        continue;
      }
      const singleMath = line.match(/^\s*\$\$([\s\S]+)\$\$\s*$/) || line.match(/^\s*\\\[([\s\S]+)\\\]\s*$/);
      if (singleMath) {
        out.push(mathToken(singleMath[1], true, line.includes("$$") ? ["$$", "$$"] : ["\\[", "\\]"]));
        index += 1;
        continue;
      }

      const heading = line.match(/^\s{0,3}(#{1,6})\s+(.+?)\s*#*\s*$/);
      if (heading) {
        const level = heading[1].length;
        out.push(`<h${level}>${inlineMarkdown(heading[2])}</h${level}>`);
        index += 1;
        continue;
      }
      if (/^\s*(?:-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
        out.push("<hr>");
        index += 1;
        continue;
      }
      if (line.includes("|") && index + 1 < lines.length && tableSeparator(lines[index + 1])) {
        const headers = splitTableRow(line);
        index += 2;
        const rows = [];
        while (index < lines.length && lines[index].trim() && lines[index].includes("|")) rows.push(splitTableRow(lines[index++]));
        out.push(`<div class="ai-md-table-wrap"><table><thead><tr>${headers.map((cell) => `<th>${inlineMarkdown(cell)}</th>`).join("")}</tr></thead><tbody>${rows.map((row) => `<tr>${headers.map((_cell, column) => `<td>${inlineMarkdown(row[column] || "")}</td>`).join("")}</tr>`).join("")}</tbody></table></div>`);
        continue;
      }
      const list = line.match(/^\s*([-+*]|\d+[.)])\s+(.+)$/);
      if (list) {
        const ordered = /^\d/.test(list[1]);
        const tag = ordered ? "ol" : "ul";
        const items = [];
        while (index < lines.length) {
          const item = lines[index].match(/^\s*([-+*]|\d+[.)])\s+(.+)$/);
          if (!item || /^\d/.test(item[1]) !== ordered) break;
          const task = item[2].match(/^\[([ xX])\]\s*(.*)$/);
          items.push(task
            ? `<li class="task-list-item"><input type="checkbox" disabled${/x/i.test(task[1]) ? " checked" : ""}> ${inlineMarkdown(task[2])}</li>`
            : `<li>${inlineMarkdown(item[2])}</li>`);
          index += 1;
        }
        out.push(`<${tag}>${items.join("")}</${tag}>`);
        continue;
      }
      if (/^\s*>\s?/.test(line)) {
        const quote = [];
        while (index < lines.length && /^\s*>\s?/.test(lines[index])) quote.push(lines[index++].replace(/^\s*>\s?/, ""));
        out.push(`<blockquote>${quote.map(inlineMarkdown).join("<br>")}</blockquote>`);
        continue;
      }
      if (!line.trim()) {
        index += 1;
        continue;
      }

      const paragraph = [line];
      index += 1;
      while (index < lines.length && lines[index].trim()
        && !/^\s*(?:#{1,6}\s|`{3,}|~{3,}|[-+*]\s+|\d+[.)]\s+|>\s?|\$\$\s*$|\\\[\s*$)/.test(lines[index])
        && !(lines[index].includes("|") && index + 1 < lines.length && tableSeparator(lines[index + 1]))) {
        paragraph.push(lines[index++]);
      }
      out.push(`<p>${paragraph.map(inlineMarkdown).join("<br>")}</p>`);
    }
    return out.join("");
  }

  function renderMath(root) {
    for (const node of root.querySelectorAll(mathSelector)) {
      let tex = "";
      try { tex = decodeURIComponent(node.dataset.tex || ""); } catch (_) { tex = node.textContent || ""; }
      if (!tex || typeof global.katex?.render !== "function") continue;
      try {
        global.katex.render(tex, node, {
          displayMode: node.dataset.display === "1",
          throwOnError: false,
          strict: "ignore",
          trust: false,
        });
        node.dataset.rendered = "katex";
      } catch (_) {
        // The escaped LaTeX source already in the node is the lossless fallback.
      }
    }
  }

  function renderNow(root, markdown) {
    if (!root) return;
    root.innerHTML = markdownToHtml(markdown);
    root.classList.add("ai-md");
    root.dataset.markdown = string(markdown);
    renderMath(root);
  }

  function renderInto(root, markdown, options = {}) {
    if (!root) return;
    const source = string(markdown);
    const pending = scheduled.get(root);
    if (pending && !options.streaming) {
      (global.cancelAnimationFrame || global.clearTimeout)?.(pending);
      scheduled.delete(root);
    }
    if (!options.streaming) {
      renderNow(root, source);
      return;
    }
    root.dataset.markdown = source;
    if (pending) return;
    const schedule = global.requestAnimationFrame || ((callback) => global.setTimeout(callback, 16));
    const handle = schedule(() => {
      scheduled.delete(root);
      renderNow(root, root.dataset.markdown || "");
    });
    scheduled.set(root, handle);
  }

  global.UniPptAiMessageRenderer = Object.freeze({
    version: 1,
    markdownToHtml,
    renderInto,
    renderMath,
    safeUrl,
  });
})(typeof globalThis !== "undefined" ? globalThis : window);
