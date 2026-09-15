(function (global) {
  "use strict";

  const FREE_HTML_SKILL_ID = "free-html-to-editable-ppt";
  const FREE_HTML_INTENT = /(?:转(?:为|成)?|生成|制作|导入|编译).{0,12}(?:ppt|pptx|演示文稿|幻灯片)|(?:ppt|pptx|演示文稿|幻灯片).{0,12}(?:转(?:为|成)?|生成|制作|导入|编译)/i;
  const WHOLE_DECK_TARGET = /(?:pptx?|演示(?:文稿)?|幻灯片|deck|(?:演讲|汇报|路演)(?:稿|演示|ppt)?|(?:\d{1,2}|[一二三四五六七八九十百]{1,4})\s*(?:页|张))/i;
  const WHOLE_DECK_ACTION = /(?:新建|创建|创造|创作|制作|制造|生成|设计|打造|产出|从零(?:创建|制作|生成|设计)|重新(?:制作|生成|设计|创作)|替换|重做|重建|做|写)/i;
  const WHOLE_DECK_DIRECT_CUE = /(?:给我|帮我|替我|为我|我要|我想要|我需要|来一(?:份|套|个)|请(?:帮|替|为|给|直接|创建|创造|创作|制作|生成|设计|打造|做|写))/i;
  const WHOLE_DECK_META_QUESTION = /(?:能否|能不能|可不可以|是否|是不是|会不会|支不支持|支持|具备.{0,6}能力|擅长|如何|怎么|为什么|是什么|(?:能|会|可以)(?=.{0,20}(?:新建|创建|创造|创作|制作|制造|生成|设计|打造|做|写)))/i;
  const WHOLE_DECK_ADVISORY = /(?:想想|分析|讨论|建议|评价|比较|研究|规划|构思|思路|方案|教程|方法)/i;
  const definitions = Object.freeze([
    Object.freeze({
      id: FREE_HTML_SKILL_ID,
      label: "自由 HTML → 可编辑 PPT",
      description: "不经过模型坐标推理，直接用浏览器布局引擎把一个自由 HTML 附件分层编译为可编辑演示文稿。",
      toolName: "presentation.importFreeHtml",
    }),
  ]);

  function htmlAttachments(attachments) {
    return (attachments || []).filter((attachment) => attachment?.kind === "html"
      && typeof attachment.html === "string" && attachment.html.trim());
  }

  function isWholeDeckCreationIntent(value) {
    // Route only affirmative clauses. A prohibition such as “不要重新生成
    // 幻灯片” must never authorize replacing the current document.
    const clauses = String(value || "").normalize("NFKC").split(/[，。！？；\n,;!?]+/);
    const prompt = clauses.filter((clause) => !/(?:不要|不用|无需|禁止|不得|别|不必|不允许|do\s+not|don't|never)/i.test(clause)).join(" ").replace(/\s+/g, " ").trim();
    if (/(?:只|仅)(?:需|要)?(?:修改|修正|修复|校正|调整|编辑|改|更新)|(?:当前|现有).{0,12}(?:修改|修正|编辑|校正)/.test(prompt)
      && !/(?:新建|创建|从零|重新生成|重做)(?:一份|一套|整个|整份|整套)/.test(prompt)) return false;
    if (!prompt || !WHOLE_DECK_TARGET.test(prompt)) return false;
    const hasAction = WHOLE_DECK_ACTION.test(prompt);
    const hasDirectCue = WHOLE_DECK_DIRECT_CUE.test(prompt);
    if (!hasAction && !hasDirectCue) return false;

    // Capability questions and requests for advice must stay conversational. A
    // direct imperative such as “能不能帮我做一份 PPT” is still an edit request.
    const asksForAdvice = WHOLE_DECK_ADVISORY.test(prompt);
    if (asksForAdvice && (!hasAction || WHOLE_DECK_META_QUESTION.test(prompt))) return false;
    if (WHOLE_DECK_META_QUESTION.test(prompt) && !hasDirectCue) return false;
    return true;
  }

  function match(options = {}) {
    const prompt = String(options.prompt || "").trim();
    const candidates = htmlAttachments(options.attachments);
    if (!candidates.length || !FREE_HTML_INTENT.test(prompt)) return null;
    const definition = definitions[0];
    if (options.mode !== "edit") {
      return {
        ...definition,
        error: "自由 HTML 转 PPT 会替换当前演示文稿，请先切换到“编排演示”模式",
      };
    }
    if (candidates.length !== 1) {
      return {
        ...definition,
        error: `一次只能把 1 个自由 HTML 编译为演示文稿；当前检测到 ${candidates.length} 个`,
      };
    }
    const attachment = candidates[0];
    return {
      ...definition,
      attachment,
      args: {
        attachmentName: attachment.name,
        title: String(options.title || "").trim() || undefined,
      },
    };
  }

  function completionText(plan, result) {
    const report = result?.report || {};
    const metricCount = (value) => {
      if (Array.isArray(value)) return value.length;
      const count = Number(value || 0);
      return Number.isFinite(count) ? count : 0;
    };
    const slideCount = Array.isArray(result?.slideIds) ? result.slideIds.length : 0;
    const native = metricCount(report.nativeText) + metricCount(report.nativeShapes);
    const images = metricCount(report.preservedImages);
    const fallbacks = metricCount(report.localizedFallbacks);
    const unsupported = metricCount(report.unsupported);
    const opensNewWindow = result?.destination === "new-window";
    return [
      `已通过本地 Skill 将 **${plan.attachment.name}** 编译为 ${slideCount || "可编辑"} 页演示文稿。`,
      `原生可编辑对象 ${native} 个，保留图片 ${images} 个，局部保真回退 ${fallbacks} 个，不支持项 ${unsupported} 个。`,
      opensNewWindow
        ? "新演示文稿已在独立窗口打开，原窗口文稿未被替换；可在新窗口直接编辑并导出 PPTX。"
        : "转换在一个事务中完成，可用 Ctrl+Z 整体撤销；确认效果后可直接导出 PPTX。",
    ].join("\n\n");
  }

  async function run(plan, options = {}) {
    if (!plan || plan.id !== FREE_HTML_SKILL_ID) return { handled: false };
    if (plan.error) throw new Error(plan.error);
    if (typeof options.callTool !== "function") throw new Error("自由 HTML Skill 没有可用的演示文稿宿主");
    const request = {
      name: plan.toolName,
      args: plan.args,
      call: { id: `local-skill:${FREE_HTML_SKILL_ID}`, type: "function" },
      definition: { name: plan.toolName },
      round: 0,
      skillId: FREE_HTML_SKILL_ID,
      destination: plan.destination || null,
    };
    options.onToolState?.({ phase: "requested", ...request });
    const approved = typeof options.beforeTools === "function"
      ? await options.beforeTools([request], 0)
      : true;
    if (!approved) {
      const denied = { ok: false, denied: true, error: "用户拒绝执行该 Skill" };
      options.onToolState?.({ phase: "denied", ...request, result: denied });
      return { handled: true, denied: true, text: "已取消自由 HTML 转 PPT，没有修改当前文稿。" };
    }
    try {
      const result = await options.callTool(plan.toolName, plan.args, request);
      options.onToolState?.({ phase: "completed", ...request, result });
      return { handled: true, result, text: completionText(plan, result) };
    } catch (error) {
      options.onToolState?.({
        phase: "failed",
        ...request,
        result: { ok: false, error: String(error?.message || error) },
      });
      throw error;
    }
  }

  global.UniPptLocalSkills = Object.freeze({
    definitions,
    match,
    run,
    isFreeHtmlIntent(prompt) { return FREE_HTML_INTENT.test(String(prompt || "")); },
    isWholeDeckCreationIntent,
  });
})(globalThis);
