(() => {
"use strict";

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];
const MIN_ZOOM = 0.1;
const MAX_ZOOM = 5;
const ZOOM_STEP = 0.1;
const PRESENTATION_MIN_FPS = 30;
const PRESENTATION_FRAME_BUDGET_MS = 1000 / PRESENTATION_MIN_FPS;
const PRESENTATION_DROPPED_FRAME_MS = 36;
const PRESENTATION_MAX_FRAME_MS = 67;
const STARTUP_FETCH_TIMEOUT_MS = 10_000;
const PRESENTATION_DEFAULT_AUTO_ADVANCE_MS = 1_000;
const PRESENTATION_RECORDING_DEFAULT_SLIDE_MS = 5_000;
const PRESENTATION_MIN_POST_ANIMATION_MS = 80;
const autoTextFitCache = new Map();
const miniFormatContext = { range: null, dismissedId: null, frame: 0, dragging: false };
let mcpBridge = null;
let mcpPolicy=null,mcpInitialization=null;
const imageReconstructionSources = new Map();
const state = {
  deck: null,
  activeSlide: 0,
  selectedId: null,
  zoom: 0.55,
  autoFit: true,
  textSelection: null,
  canvasPointer: null,
  history: [],
  future: [],
  presenterSlide: 0,
  presenterAnimationCursor: 0,
  presenterAnimationTargetCursor: null,
  presenterAnimationBatchStartCursor: null,
  presenterTimers: [],
  presenterPlayers: [],
  presenterPlaybackToken: 0,
  presenterControlsTimer: 0,
  presenterAutoPlayTimer: 0,
  presenterSlideStartedAt: 0,
  presenterBoundaryTimer: 0,
  presenterFullscreen: false,
  presenterRangeStart: 0,
  presenterRangeEnd: 0,
  presenterMode: "show",
  presenterVolume: 1,
  presenterMuted: false,
  slideShowSettings: {
    range: "all",
    from: 1,
    to: null,
    loop: false,
    autoPlay: false,
    useTimings: true,
    fullscreen: true,
  },
  rehearsalTimings: {},
  rehearsalStartedAt: 0,
  rehearsalSlideStartedAt: 0,
  rehearsalClockTimer: 0,
  editingFormulaId: null,
  insertingObjectKind: null,
  editingInsertObjectId: null,
  selectedAnimationId: null,
  animationPreviewTimers: [],
  formatPaneOpen: false,
  inkTool: "select",
  inkColor: "#202124",
  inkWidths: { pen: 4, highlighter: 16, eraser: 18 },
  inkPointerId: null,
  inkPoints: [],
  inkPreview: null,
  inkEraseCheckpointed: false,
  inkEraseChanged: false,
  internalClipboard: null,
  pasteOffset: 0,
  formatPainter: null,
  findCursor: -1,
  outlineView: false,
  hostCommitDepth: 0,
  documentCache: globalThis.UniPptDocumentCache?.create?.() || null,
  documentHost: null,
};
const handles = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
let fitFrame = 0;
const activeSlideTransitions = new WeakMap();
let embeddedFontFaces = [];
let thumbnailResizeObserver = null;
let fontInstallGeneration = 0;
let fontMetricRefreshTimer = 0;
let fontEventsBound = false;
let verifiedFontBatchActive = 0;
let udocStructureRefreshFrame = 0;
let transitionCloneIdSequence = 0;
let aiMutationAuthorizer = null;
const activePresentationFrameProbes = new Set();
const presentationPerformance = {
  tier: "high",
  lastFps: 60,
  lastSample: null,
  slowRuns: 0,
  fastRuns: 0,
  qualityByKind: Object.create(null),
  streaksByKind: Object.create(null),
  samples: [],
};

function startUniPptEditor() {
  bindUi();
  globalThis.UniPptLocalSettings.install();
  showStartupSurface('loading','正在打开 UniPPT','正在连接文稿服务…');
  void loadInitialDeck();
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", startUniPptEditor, { once: true });
} else {
  startUniPptEditor();
}

function ensureFeature(name) {
  return globalThis.UniPptStartup?.loadFeature?.(name) || Promise.resolve();
}

function startupAsset(path) {
  return globalThis.UniPptStartup?.asset?.(path) || path;
}

async function loadInitialDeck() {
  const controller = new AbortController();
  let timeout = 0;
  $("#slideStage")?.setAttribute("aria-busy", "true");
  showStartupSurface('loading','正在打开文稿','正在连接文稿服务…');
  setStatus("正在连接文稿服务…");
  try {
    const startup = globalThis.UniPptStartup;
    if (globalThis.UniPptDocumentHandoff?.receiverToken?.()) {
      setStatus("正在接收新生成的演示文稿…");
      const received = await globalThis.UniPptDocumentHandoff.receive({ timeoutMs: 60_000 });
      try {
        state.deck = normalizeDeck(received.deck);
        resetDocumentCache({ sourceKind: "ai-handoff" });
        void installEmbeddedFonts(state.deck);
        renderAll();
        globalThis.UniPptStartup?.metrics && (globalThis.UniPptStartup.metrics.deckReadyAt = performance.now());
        await new Promise((resolve) => requestAnimationFrame(() => {
          fitSlide();
          globalThis.UniPptStartup?.markFirstPaint?.();
          resolve();
        }));
        received.accept?.();
        setStatus("新演示文稿已就绪 · 可编辑并可直接导出 PPTX");
        toast("新生成的演示文稿已在独立窗口打开");
        return;
      } catch (error) {
        received.reject?.(error);
        throw error;
      }
    }
    const earlyRequest = startup?.takeDemoResponse?.();
    const request = earlyRequest || fetch("/api/demo", {
      cache: "no-store",
      signal: controller.signal,
    }).then(
      (response) => ({ response }),
      (error) => ({ error }),
    );
    const deadline = new Promise((_, reject) => {
      timeout = setTimeout(() => {
        controller.abort();
        startup?.abortDemo?.();
        reject(new DOMException("启动请求超时", "AbortError"));
      }, STARTUP_FETCH_TIMEOUT_MS);
    });
    const result = await Promise.race([request, deadline]);
    if (result?.error) throw result.error;
    const response = result?.response;
    if (!response) throw new Error("文稿服务没有返回响应");
    if (!response.ok) {
      const payload = await response.json().catch(() => null);
      throw new Error(payload?.error || `文稿服务返回 HTTP ${response.status}`);
    }
    state.deck = normalizeDeck(await response.json());
    resetDocumentCache({ sourceKind: "demo" });
    void installEmbeddedFonts(state.deck);
    renderAll();
    globalThis.UniPptStartup?.metrics && (globalThis.UniPptStartup.metrics.deckReadyAt = performance.now());
    requestAnimationFrame(() => {
      fitSlide();
      globalThis.UniPptStartup?.markFirstPaint?.();
    });
    setStatus("就绪 · UniPPT 文档引擎已连接");
  } catch (error) {
    const message = controller.signal.aborted
      ? "启动超过 10 秒：后台任务繁忙或服务未响应"
      : `启动失败：${error?.message || "无法连接文稿服务"}`;
    showStartupFailure(message);
  } finally {
    clearTimeout(timeout);
    $("#slideStage")?.removeAttribute("aria-busy");
    if(state.deck)requestAnimationFrame(()=>{void initializeGlobalMcp();});
  }
}

function initializeGlobalMcp(){
  if(mcpInitialization)return mcpInitialization;
  mcpInitialization=(async()=>{
    await ensureFeature('mcp');installDocumentHost();
    if(!state.documentHost||!globalThis.UniPptMcpConnectionPolicy)return;
    let storage;try{storage=window.localStorage;}catch{}
    const button=$('#mcpConnect');
    const updateButton=(connected=false)=>{
      const enabled=mcpPolicy?.enabled===true;
      button.setAttribute('aria-checked',String(enabled));
      button.textContent=enabled?(connected?'本机 MCP · 已连接':'本机 MCP · 连接中'):'本机 MCP · 关';
      $('#settingsMcpStatus').textContent=enabled?(connected?'已连接 · 本机文稿':'已开启 · 正在连接'):'已关闭';
    };
    mcpBridge ||= globalThis.UniPptMcpBridge.create({host:state.documentHost,getDeck:()=>state.deck,undo,
      getSourceImage:slideId=>imageReconstructionSources.get(slideId),
      onStatus(connected,message){updateButton(connected);button.title=message+'。全局开关仅限本机；点击关闭后所有本机标签页同步。';},
    });
    mcpPolicy=globalThis.UniPptMcpConnectionPolicy.create({origin:location.origin,storage,onChange(enabled){updateButton();void mcpBridge.setAutoConnect(enabled);}});
    window.addEventListener('storage',event=>mcpPolicy.storageChanged(event));
    updateButton();await mcpBridge.setAutoConnect(mcpPolicy.enabled);
  })().catch(error=>{mcpInitialization=null;$('#mcpConnect').textContent='本机 MCP · 重试';$('#mcpConnect').title=error.message;});
  return mcpInitialization;
}

function showStartupFailure(message) {
  setStatus(message);
  showStartupSurface('error','暂时无法打开文稿',message,'重新加载',()=>void loadInitialDeck());
}

function showStartupSurface(mode,title,detail,label,action) {
  document.body.dataset.startupState=mode;
  const surface=$('#startupSurface');surface.hidden=mode==='ready';
  surface.setAttribute('aria-busy',String(mode==='loading'));
  for(const element of $$('.ribbon,.slide-pane,.notes-strip,.utility-rail'))element.inert=mode!=='ready';
  if(mode==='ready')return;
  surface.replaceChildren();
  const logo=document.createElement('img');logo.src=startupAsset('/unippt-logo.svg');logo.alt='';logo.width=44;logo.height=44;
  const heading=document.createElement('h1');heading.textContent=title;
  const text=document.createElement('p');text.textContent=detail;
  surface.append(logo,heading,text);
  if(action){const button=document.createElement('button');button.type='button';button.textContent=label;button.onclick=action;surface.append(button);}
}

function bindUi() {
  bindBrowserGuards();
  window.addEventListener("unippt:feature-ready", (event) => {
    if (event.detail?.name === "ai") installDocumentHost();
    if (event.detail?.name === "render" && state.deck) {
      renderSlideList();
      renderCanvas();
    }
  });
  window.addEventListener("unippt:katex-ready", () => {
    if (!state.deck) return;
    renderSlideList();
    renderCanvas();
  });
  $("#openFile").onclick = openDocumentPicker;
  $("#fileInput").onchange = (event) => openFile(event.target.files[0]);
  $("#newDeck").onclick = newDeck;
  $("#saveUdocQuick").onclick = saveUdoc;
  $("#savePptx").onclick = savePptx;
  $("#saveUdoc").onclick = saveUdoc;
  $("#exportMenuButton").onclick = (event) => {
    event.stopPropagation();
    toggleExportMenu(event.currentTarget);
  };
  $("#exportHtml").onclick = () => { closeExportMenu(); void exportLosslessHtml(); };
  $("#exportPdf").onclick = () => { closeExportMenu(); void exportPdf(); };
  $("#exportImages").onclick = () => { closeExportMenu(); void exportSlideImages(); };
  $("#exportVideoFast").onclick = () => { closeExportMenu(); void exportVideoFast(); };
  $("#exportVideo").onclick = () => { closeExportMenu(); void exportVideo(); };
  $("#exportVideoRecording").onclick = () => { closeExportMenu(); void exportVideoRecording(); };
  $("#videoExportEngine").onchange = updateVideoExportSummary;
  $("#videoExportProfile").onchange = updateVideoExportSummary;
  $("#videoExportResolution").onchange = updateVideoExportSummary;
  $("#videoExportFps").onchange = updateVideoExportSummary;
  $("#videoExportMuted").onchange = updateVideoExportSummary;
  $("#confirmVideoExport").onclick = () => { void confirmVideoExport(); };
  window.addEventListener("beforeprint", setupNativePrintPages);
  window.addEventListener("afterprint", teardownNativePrintPages);
  document.addEventListener("pointerdown", (event) => {
    if (!$("#exportMenuPopup")?.hidden
      && !event.target.closest?.("#exportMenuPopup, #exportMenuButton, #utilityConvert")) closeExportMenu();
    if (!$("#slideContextMenu")?.hidden
      && !event.target.closest?.("#slideContextMenu")) closeSlideContextMenu();
    if (!$("#officeSearchResults")?.hidden
      && !event.target.closest?.("#officeSearchWrap")) closeOfficeSearchResults();
  });
  $("#aiAssistantTop").onclick = () => { void openAiAssistant(); };
  $("#mcpConnect").onclick = async event => {
    try {
      if(!event.isTrusted)return;
      await initializeGlobalMcp();
      if(mcpPolicy)mcpPolicy.setEnabled(!mcpPolicy.enabled);
    } catch (error) { toast('MCP 连接失败：请先在 MCP 客户端调用 UniPPT 工具启动本机桥接。' + error.message); }
  };
  $("#undo").onclick = undo;
  $("#redo").onclick = redo;
  $("#deckTitle").onchange = (event) => mutate(() => {
    state.deck.title = event.target.value || "演示文稿1";
  });

  $$("[data-ribbon]").forEach((button) => {
    button.onclick = () => activateRibbon(button.dataset.ribbon);
  });
  $$(".utility-rail [data-target-ribbon]").forEach((button) => {
    button.onclick = () => activateRibbon(button.dataset.targetRibbon);
  });
  $("#utilityUploads").onclick = openDocumentPicker;
  $("#utilityConvert").onclick = (event) => {
    const anchor = event.currentTarget;
    activateRibbon("file");
    requestAnimationFrame(() => toggleExportMenu(anchor, true));
  };
  $("#addSlide").onclick = addSlide;
  $("#addSlideRail").onclick = addSlide;
  $("#duplicateSlide").onclick = duplicateSlide;
  $("#moveSlideUp").onclick = () => moveSlide(-1);
  $("#moveSlideDown").onclick = () => moveSlide(1);
  $("#deleteSlide").onclick = deleteSlide;
  $("#slideContextNew").onclick = () => runSlideContextAction(addSlide);
  $("#slideContextDuplicate").onclick = () => runSlideContextAction(duplicateSlide);
  $("#slideContextMoveUp").onclick = () => runSlideContextAction(() => moveSlide(-1));
  $("#slideContextMoveDown").onclick = () => runSlideContextAction(() => moveSlide(1));
  $("#slideContextTransition").onclick = openCurrentSlideTransitionSettings;
  $("#slideContextDelete").onclick = () => runSlideContextAction(deleteSlide);
  $("#slideList").addEventListener("scroll", closeSlideContextMenu, { passive: true });
  window.addEventListener("resize", closeSlideContextMenu);
  $("#officeSearchInput").oninput = () => {
    state.findCursor = -1;
    updateOfficeSearchResults();
  };
  $("#officeSearchInput").onfocus = updateOfficeSearchResults;
  $("#officeSearchInput").onkeydown = handleOfficeSearchKeydown;
  $("#addText").onclick = addText;
  $("#addShape").onclick = addShape;
  $("#addImage").onclick = () => $("#imageInput").click();
  $("#imageInput").onchange = (event) => addImageFile(event.target.files[0]);
  $("#captureScreen").onclick = captureScreen;
  $("#addTable").onclick = () => openInsertObjectDialog("table");
  $("#addChart").onclick = () => openInsertObjectDialog("chart");
  $("#addSmartArt").onclick = () => openInsertObjectDialog("smartArt");
  $("#addSymbol").onclick = () => openInsertObjectDialog("symbol");
  $("#addVideo").onclick = () => $("#videoInput").click();
  $("#videoInput").onchange = (event) => addMediaFile(event.target.files[0], "video");
  $("#addAudio").onclick = () => $("#audioInput").click();
  $("#audioInput").onchange = (event) => addMediaFile(event.target.files[0], "audio");
  $("#addDynamicContent").onclick = () => openInsertObjectDialog("dynamic");
  $("#addFormula").onclick = () => openFormulaDialog();
  bindInkTools();
  bindTransitionInputs();
  $$('[data-add-animation]').forEach((button) => {
    button.onclick = () => addAnimation(button.dataset.addAnimation);
  });
  $("#addMotionPath").onclick = () => addAnimation("motionPath");
  $("#previewAnimations").onclick = previewAnimations;
  $("#toggleAnimationPane").onclick = openAnimationPane;
  $("#closeAnimationPane").onclick = closeAnimationPane;
  $("#playAnimationPane").onclick = previewAnimations;
  $("#removeAnimation").onclick = deleteSelectedAnimation;
  $("#deleteAnimation").onclick = deleteSelectedAnimation;
  $("#animationMoveUp").onclick = () => moveAnimation(-1);
  $("#animationMoveDown").onclick = () => moveAnimation(1);
  bindAnimationInputs();

  $("#slideNotes").onchange = (event) => mutate(() => {
    currentSlide().notes = event.target.value;
  }, false);
  $("#deleteObject").onclick = deleteSelected;
  $("#propBold").onclick = () => toggleSelectedStyle("bold");
  $("#propItalic").onclick = () => toggleSelectedStyle("italic");
  $("#ribbonBold").onclick = () => toggleSelectedStyle("bold");
  $("#ribbonItalic").onclick = () => toggleSelectedStyle("italic");
  $("#ribbonUnderline").onclick = () => toggleSelectedRunStyle("underline");
  $("#ribbonStrike").onclick = () => toggleSelectedRunStyle("strikethrough");
  $("#ribbonSubscript").onclick = () => toggleSelectedBaseline("sub");
  $("#ribbonSuperscript").onclick = () => toggleSelectedBaseline("super");
  for (const id of [
    "propBold", "propItalic", "ribbonBold", "ribbonItalic", "ribbonUnderline",
    "ribbonStrike", "ribbonSubscript", "ribbonSuperscript", "growFont", "shrinkFont",
  ]) {
    $("#" + id)?.addEventListener("pointerdown", preserveTextSelectionForCommand);
  }
  $("#growFont").onclick = () => adjustSelectedFontSize(2);
  $("#shrinkFont").onclick = () => adjustSelectedFontSize(-2);
  $("#ribbonFontFamily").onchange = (event) => updateSelected((object) => {
    applyNativeFontFamily(object, event.target.value);
  });
  $("#ribbonFontSize").onchange = (event) => updateSelected((object) => {
    const points = Math.max(1, Number(event.target.value) || 1);
    applyTextStyle(object, "fontSize", points * 96 / 72);
  });
  bindRibbonCommands();
  $("#editFormula").onclick = () => openFormulaDialog(state.selectedId);
  $("#openFormatPane").onclick = () => openFormatPane();
  $("#closeFormatPane").onclick = closeFormatPane;
  bindInspectorInputs();

  $("#zoom").oninput = (event) => {
    setZoom(Number(event.target.value) / 100, false, currentCanvasZoomAnchor());
  };
  $("#zoomOut").onclick = () => adjustZoom(-ZOOM_STEP, currentCanvasZoomAnchor());
  $("#zoomIn").onclick = () => adjustZoom(ZOOM_STEP, currentCanvasZoomAnchor());
  $("#canvasViewport").addEventListener("pointermove", rememberCanvasPointer);
  $("#canvasViewport").addEventListener("pointerenter", rememberCanvasPointer);
  document.addEventListener("selectionchange", rememberActiveTextSelection);
  bindMiniFormatToolbar();
  $("#fitSlide").onclick = fitSlide;
  $("#normalView").onclick = () => {
    showNormalView();
    $$(".status-view").forEach((button) => button.classList.toggle("active", button.id === "normalView"));
  };
  $("#viewUdocStructure").onclick = showUdocStructure;
  $("#readingView").onclick = () => { void startPresentationWhenReady("current"); };
  $("#closeUdocStructure").onclick = closeUdocStructure;
  $("#confirmUdocStructure").onclick = closeUdocStructure;
  $("#copyUdocStructure").onclick = copyUdocStructure;
  $("#udocStructureDialog").addEventListener("close", syncUdocStructureClosedState);

  $("#present").onclick = () => { void startPresentationWhenReady("current"); };
  $("#presentRibbon").onclick = () => { void startPresentationWhenReady("beginning"); };
  $("#presentCurrentRibbon").onclick = () => { void startPresentationWhenReady("current"); };
  $("#presentAutoRibbon").onclick = () => { void startPresentationWhenReady("current", { autoPlay: true }); };
  $("#setupSlideShow").onclick = openSlideShowSettings;
  $("#rehearseTimings").onclick = startRehearsal;
  $("#closeSlideShowSettings").onclick = closeSlideShowSettings;
  $("#cancelSlideShowSettings").onclick = closeSlideShowSettings;
  $("#applySlideShowSettings").onclick = applySlideShowSettings;
  $("#slideShowAll").onchange = syncSlideShowRangeInputs;
  $("#slideShowRange").onchange = syncSlideShowRangeInputs;
  $("#discardRehearsalTimings").onclick = discardRehearsalTimings;
  $("#applyRehearsalTimings").onclick = applyRehearsalTimings;
  const presenter = $("#presenter");
  const presentControls = presenter.querySelector(".present-controls");
  const prevSlidePresent = $("#prevSlidePresent");
  const prevPresent = $("#prevPresent");
  const nextPresent = $("#nextPresent");
  const nextSlidePresent = $("#nextSlidePresent");
  const presentAutoPlay = $("#presentAutoPlay");
  const closePresent = $("#closePresent");
  const presentMute = $("#presentMute");
  const presentVolume = $("#presentVolume");
  prevPresent.textContent = "‹";
  prevPresent.title = "上一个动画节点；到页首后返回上一张（←）";
  prevPresent.setAttribute("aria-label", "上一个动画节点或上一张幻灯片");
  nextPresent.textContent = "›";
  nextPresent.title = "下一个动画节点；到页尾后进入下一张（→ 或空格）";
  nextPresent.setAttribute("aria-label", "下一个动画节点或下一张幻灯片");
  prevSlidePresent.textContent = "⇤";
  prevSlidePresent.title = "上一页（Ctrl+Home）";
  prevSlidePresent.setAttribute("aria-label", "上一页");
  nextSlidePresent.textContent = "⇥";
  nextSlidePresent.title = "下一页（Ctrl+End）";
  nextSlidePresent.setAttribute("aria-label", "下一页");
  presentAutoPlay.title = "自动放映：本页全部动画完成后自动换页（A）";
  presentAutoPlay.setAttribute("aria-label", "自动放映");
  closePresent.textContent = "×";
  closePresent.title = "退出放映（Esc）";
  closePresent.setAttribute("aria-label", "退出幻灯片放映");
  presentMute.onclick = (event) => {
    event.stopPropagation();
    togglePresentationMute();
    showPresentationControls(true);
  };
  presentVolume.oninput = (event) => {
    event.stopPropagation();
    setPresentationVolume(Number(event.target.value) / 100);
    showPresentationControls(true);
  };
  presentVolume.onchange = () => showPresentationControls();
  closePresent.onclick = (event) => {
    event.stopPropagation();
    closePresentation();
  };
  prevPresent.onclick = (event) => {
    event.stopPropagation();
    stepPresentation(-1);
    showPresentationControls();
  };
  nextPresent.onclick = (event) => {
    event.stopPropagation();
    stepPresentation(1);
    showPresentationControls();
  };
  prevSlidePresent.onclick = (event) => {
    event.stopPropagation();
    jumpPresentationPage(-1);
    showPresentationControls();
  };
  nextSlidePresent.onclick = (event) => {
    event.stopPropagation();
    jumpPresentationPage(1);
    showPresentationControls();
  };
  presentAutoPlay.onclick = (event) => {
    event.stopPropagation();
    togglePresentationAutoPlay();
    showPresentationControls(true);
  };
  presentControls.addEventListener("pointerdown", (event) => event.stopPropagation());
  presentControls.addEventListener("click", (event) => event.stopPropagation());
  presentControls.addEventListener("pointerenter", () => showPresentationControls(true));
  presentControls.addEventListener("pointerleave", hidePresentationControls);
  presenter.addEventListener("pointermove", (event) => {
    const nearBottom = presenter.clientHeight - event.clientY < 88;
    if (nearBottom) showPresentationControls(true);
    else if (!presentControls.matches(":hover,:focus-within")) hidePresentationControls();
  });
  presenter.addEventListener("pointerleave", hidePresentationControls);
  presenter.onclick = (event) => {
    // PowerPoint advances from a click anywhere on the slide, including text,
    // pictures and grouped shapes. Hyperlink handlers stop propagation when
    // they consume a click, while the floating controls are excluded here.
    if (!event.target.closest?.(".present-controls")) stepPresentation(1);
  };
  document.addEventListener("fullscreenchange", handlePresentationFullscreenChange);

  $("#formulaSource").oninput = () => {
    const parsed = parseLatexEnvelope($("#formulaSource").value, $("#formulaDisplay").checked);
    if (parsed.delimited) $("#formulaDisplay").checked = parsed.display;
    renderFormulaPreview();
  };
  $("#formulaDisplay").onchange = () => {
    const parsed = parseLatexEnvelope($("#formulaSource").value, $("#formulaDisplay").checked);
    $("#formulaSource").value = formatLatexSource(parsed.latex, $("#formulaDisplay").checked);
    renderFormulaPreview();
  };
  $("#commitFormula").onclick = commitFormula;
  $("#cancelFormula").onclick = () => $("#formulaDialog").close();
  $("#commitInsertObject").onclick = commitInsertObject;
  $("#cancelInsertObject").onclick = closeInsertObjectDialog;
  $("#closeInsertObject").onclick = closeInsertObjectDialog;
  $$(".formula-presets [data-latex]").forEach((button) => {
    button.onclick = () => {
      $("#formulaSource").value = formatLatexSource(button.dataset.latex, $("#formulaDisplay").checked);
      renderFormulaPreview();
    };
  });
  $$(".theme-chip").forEach((button) => {
    button.onclick = () => applyTheme(button.className);
  });

  $("#slideStage").addEventListener("pointerdown", handleInkPointerDown, { capture: true });
  $("#slideStage").addEventListener("pointermove", handleInkPointerMove, { capture: true });
  $("#slideStage").addEventListener("pointerup", handleInkPointerUp, { capture: true });
  $("#slideStage").addEventListener("pointercancel", cancelInkPointer, { capture: true });
  $("#slideStage").addEventListener("pointerdown", (event) => {
    if (event.target === $("#slideStage")) selectObject(null);
  });
  window.addEventListener("resize", () => {
    if (!$("#presenter").hidden) fitPresentationStage();
    else if (state.autoFit) scheduleFitSlide();
  });
  new ResizeObserver(() => {
    if (state.deck && state.autoFit && $("#presenter").hidden) scheduleFitSlide();
  }).observe($("#canvasViewport"));
  document.addEventListener("keydown", handleKeyboard);
}

function openDocumentPicker() {
  // Local Font Access must be invoked before this trusted click/key event
  // returns. The file-input change event no longer has transient activation.
  void globalThis.UniPptFontRuntime?.authorizeLocalFonts?.();
  $("#fileInput").click();
}

function installDocumentHost() {
  const runtime = globalThis.UniPptPresentationHost;
  if (!runtime?.create || state.documentHost) return;
  state.documentHost = runtime.create({
    getDeck: () => state.deck,
    getRevision: () => state.documentCache?.revision || 0,
    getSelection: () => ({
      slideId: state.deck?.slides?.[state.activeSlide]?.id || null,
      objectId: state.selectedId,
      animationId: state.selectedAnimationId,
    }),
    setSelection(selection = {}) {
      if (selection.slideId && state.deck) {
        const index = state.deck.slides.findIndex((slide) => slide.id === selection.slideId);
        if (index >= 0) state.activeSlide = index;
      }
      state.selectedId = selection.objectId || null;
      state.selectedAnimationId = selection.animationId || null;
      renderAll();
      state.documentHost?._emit?.("selection", state.documentHost.selection.get());
    },
    async commit(nextDeck, outcome, commitOptions = {}) {
      if (commitOptions.replace) {
        if (Object.prototype.hasOwnProperty.call(commitOptions, "expectedDeckRef") && commitOptions.expectedDeckRef !== state.deck) {
          throw new Error("文稿已切换，已阻止陈旧的异步结果覆盖当前文稿");
        }
        if (commitOptions.expectedRevision != null && Number(commitOptions.expectedRevision) !== Number(state.documentCache?.revision || 0)) {
          throw new Error(`文稿已更新：期望 revision ${commitOptions.expectedRevision}，当前为 ${state.documentCache?.revision || 0}`);
        }
        replaceDeck(normalizeDeck(nextDeck), "AI 已创建结构化演示文稿", { sourceKind: "ai", undoable: true });
        return;
      }
      state.hostCommitDepth += 1;
      try {
        mutate(() => {
          state.deck = normalizeDeck(nextDeck);
          if (outcome.activeSlideId) {
            const index = state.deck.slides.findIndex((slide) => slide.id === outcome.activeSlideId);
            if (index >= 0) state.activeSlide = index;
          }
          state.selectedId = outcome.selectedObjectId ?? state.selectedId;
          state.selectedAnimationId = outcome.selectedAnimationId ?? null;
        });
      } finally {
        state.hostCommitDepth -= 1;
      }
    },
    async openDocument(nextDeck, outcome, commitOptions = {}) {
      const destination = commitOptions.destination;
      if (destination?.kind !== "new-window" || typeof destination.publish !== "function") {
        throw new Error("新文稿窗口会话无效，已保持当前文稿不变");
      }
      await destination.publish(normalizeDeck(nextDeck), outcome);
      setStatus("AI 新演示文稿已在独立窗口打开");
      toast("新演示文稿已在新窗口打开；当前文稿未被替换");
    },
    exportAs: {
      udoc: saveUdoc,
      pptx: savePptx,
      html: exportLosslessHtml,
      pdf: exportPdf,
      images: exportSlideImages,
      video: exportVideo,
    },
    ui: {
      addButton: addPluginButton,
      openPanel: openPluginPanel,
      toast,
      removePluginContributions(pluginId) {
        document.querySelectorAll(`[data-plugin-id="${cssEscape(pluginId)}"]`).forEach((node) => node.remove());
      },
    },
    ai: {
      open: openAiAssistant,
      bindMutationAuthorizer(authorizer) {
        aiMutationAuthorizer = authorizer;
      },
    },
  });
  // One host, two names: new code uses UniPpt while generic UniDoc plugins can
  // feature-detect `unidoc_type === "pptx"` and reuse the common surface.
  globalThis.UniPpt = state.documentHost;
  globalThis.UniDoc = state.documentHost;
  state.documentHost.registerPlugin({
    id: "ai-assistant",
    name: "U AI",
    version: "2.0",
    builtin: true,
    supportedTypes: ["docs", "pptx"],
    activate(api) {
      api.ui.addButton({ pluginId: "ai-assistant", icon: "✦", label: "U AI", title: "AI 创建与编辑演示文稿", onClick: () => api.ai.open() });
    },
  });
}

function addPluginButton(options = {}) {
  const host = $("#pluginTools");
  if (!host) return null;
  const button = element("button", "plugin-btn");
  button.type = "button";
  button.dataset.pluginId = options.pluginId || "anonymous";
  button.title = options.title || options.label || "插件";
  button.append(element("span", "", options.icon || "◇"), element("span", "", options.label || "插件"));
  button.onclick = () => {
    try { options.onClick?.(); } catch (error) { toast(`插件执行失败：${error.message}`); }
  };
  host.append(button);
  return button;
}

function openPluginPanel(options = {}) {
  const id = `plugin-panel-${String(options.id || "default").replace(/[^a-z0-9_-]/gi, "-")}`;
  document.querySelectorAll(".plugin-panel").forEach((panel) => { if (panel.id !== id) panel.remove(); });
  let panel = document.getElementById(id);
  if (!panel) {
    panel = element("section", "plugin-panel");
    panel.id = id;
    panel.dataset.pluginId = options.pluginId || options.id || "anonymous";
    document.body.append(panel);
  }
  const header = element("header");
  header.append(element("strong", "", options.title || "插件"));
  const close = element("button", "", "×");
  close.type = "button";
  close.title = "关闭";
  close.onclick = () => panel.remove();
  header.append(close);
  const body = element("div", "plugin-panel-body");
  panel.replaceChildren(header, body);
  if (typeof options.render === "function") options.render(body);
  else if (typeof options.html === "string") body.innerHTML = options.html;
  return { el: panel, body, close: () => panel.remove() };
}

const UNIPPT_AI_CONFIG_KEY = "unippt-ai-config";
const UNIPPT_AI_DEFAULT_CONFIG = Object.freeze({
  base: "https://dashscope.aliyuncs.com/compatible-mode/v1",
  model: "qwen3.7-flash",
  key: "",
});

function loadUniPptAiConfig() {
  try {
    return { ...UNIPPT_AI_DEFAULT_CONFIG, ...JSON.parse(localStorage.getItem(UNIPPT_AI_CONFIG_KEY) || "{}") };
  } catch (_) {
    return { ...UNIPPT_AI_DEFAULT_CONFIG };
  }
}

const UNIPPT_AI_READ_ONLY_TOOLS = new Set(["presentation.inspect", "presentation.validate"]);

const UNIPPT_AI_TOOL_UI = Object.freeze({
  "presentation.inspect": { label: "读取演示文稿", completed: "已读取演示文稿结构", readOnly: true },
  "presentation.validate": { label: "检查演示文稿", completed: "演示文稿检查完成", readOnly: true },
  "presentation.compose": { label: "编排整套咨询级演示文稿", completed: "整套演示文稿已编排并完成质量检查" },
  "presentation.importFreeHtml": { label: "把自由 HTML 编译为可编辑 PPT", completed: "自由 HTML 已分层转换为可编辑幻灯片" },
  "presentation.create": { label: "创建演示文稿", completed: "演示文稿已创建" },
  "slide.add": { label: "添加幻灯片", completed: "幻灯片已添加" },
  "slide.reconstructFromImage": { label: "从图片重建可编辑幻灯片", completed: "图片已重建为可编辑幻灯片" },
  "slide.update": { label: "更新幻灯片", completed: "幻灯片已更新" },
  "slide.remove": { label: "删除幻灯片", completed: "幻灯片已删除" },
  "object.add": { label: "添加页面对象", completed: "页面对象已添加" },
  "object.update": { label: "更新页面对象", completed: "页面对象已更新" },
  "object.setText": { label: "修改文字", completed: "文字已更新" },
  "object.remove": { label: "删除页面对象", completed: "页面对象已删除" },
  "animation.add": { label: "添加动画", completed: "动画已添加" },
  "transition.set": { label: "设置切换效果", completed: "切换效果已更新" },
  "presentation.applyChangeSet": { label: "应用演示文稿修改", completed: "演示文稿修改已应用" },
});

function uniPptAiToolUi(name) {
  return UNIPPT_AI_TOOL_UI[name] || {
    label: "执行演示文稿操作",
    completed: "演示文稿操作已完成",
    readOnly: UNIPPT_AI_READ_ONLY_TOOLS.has(name),
  };
}

const UNIPPT_AI_HTML_MAX_CHARS = 2 * 1024 * 1024;
const UNIPPT_AI_HTML_MAX_NODES = 8000;
const UNIPPT_AI_HTML_MAX_PAGES = 24;

function uniPptUtf8Bytes(value) {
  return new TextEncoder().encode(String(value || "")).byteLength;
}

function uniPptAiHtmlAuthoringPrompt() {
  return [
    "你是 UniPPT 的自由 HTML 演示设计引擎。根据用户需求生成一份完整、自包含、静态的 HTML 演示稿。",
    "只返回原始 HTML 文档，从 <!doctype html> 开始到 </html> 结束；不要 Markdown 代码围栏、解释、前言或后记。",
    "整稿使用一份共享 <style>。每页必须是 <section class=\"omnidoc-page\" data-page-size=\"1280x720\">，固定 1280×720、position:relative、overflow:hidden；页面可以位于容器中但绝不能互相嵌套，禁止用脚本计算布局。",
    "每页添加 data-title 和 data-unippt-role；第一页 role=cover，末页 role=closing，其余按 content/metric/process/comparison/roadmap 等语义填写。",
    "每页主标题元素必须标记 data-unippt-role=\"cover-title\"、\"slide-title\" 或 \"closing-title\"。数字指标标记 metric-value；重要文本、形状和图片可用 data-unippt-role 提供稳定语义名。",
    "每页都放置 <template data-unippt-notes>，写简洁演讲提示，并以 [Sources]...[/Sources] 记录本页来源；没有外部来源时写 '- No external sources; authored synthesis.'。来源 URL 只能作为模板内纯文本，不能做外链。",
    "不得虚构统计数字、客户案例、研究结论或来源 URL。用户没有提供可靠证据时使用定性表达；带 metric-value、图表或明确证据数字的页面必须给出真实的非占位来源，否则整稿会被质量门禁拒绝。",
    "使用浏览器原生 HTML/CSS 自由设计：Grid、Flex、绝对定位、线性渐变、圆角、边框、阴影、内联 SVG 均可。让相邻页面轮廓不同，但全稿字体、颜色、留白和层级统一。",
    "所有可见文案面向演讲受众；每页只有一个明确结论。封面极简，正文宁可删减也不要缩小字体：封面标题至少 50px、页面标题至少 35px、正文至少 16px。",
    "图表优先用普通 div、边框和 CSS 尺寸表达，使其能转为原生可编辑形状与文字；不要把关键文字放进 SVG。",
    "严禁 script、iframe、object、embed、video、audio、form、外部 stylesheet、@import、事件属性和任何 http/https/协议相对资源；图片只能使用必要的 data:image 或纯 CSS/SVG，避免大体积 base64。",
    "不要使用 backdrop-filter、mix-blend-mode、mask、3D transform、CSS filter、伪元素生成正文或依赖 hover/click 的状态。",
    "在 <head> 写入 meta：unippt-title、unippt-audience、unippt-purpose、unippt-takeaway。页面数严格遵守用户要求；未指定时根据叙事需要生成 8—12 页。",
  ].join("\n");
}

function extractUniPptAiHtml(raw) {
  let source = String(raw || "").trim();
  source = source.replace(/^```(?:html)?\s*/i, "").replace(/\s*```$/i, "").trim();
  const start = source.search(/<!doctype\s+html|<html\b/i);
  const end = source.toLowerCase().lastIndexOf("</html>");
  if (start < 0 || end < start) throw new Error("模型返回内容不是完整 HTML 文档，已保持原文稿不变");
  source = source.slice(start, end + 7).trim();
  if (uniPptUtf8Bytes(source) > UNIPPT_AI_HTML_MAX_CHARS) throw new Error("AI 生成的 HTML 超过 2 MiB UTF-8，请减少重复样式和内联资源");
  return source;
}

function requestedUniPptPageCount(prompt) {
  const text = String(prompt || "");
  const digits = text.match(/(?:生成|制作|创建|新建|做).{0,18}?(\d{1,2})\s*(?:页|张)|(?:^|\D)(\d{1,2})\s*(?:页|张).{0,18}?(?:ppt|演示|幻灯片|演讲|汇报|路演)/i);
  const numeric = Number(digits?.[1] || digits?.[2]);
  if (Number.isInteger(numeric) && numeric >= 2 && numeric <= UNIPPT_AI_HTML_MAX_PAGES) return numeric;
  const chinese = text.match(/([一二三四五六七八九十]{1,3})\s*(?:页|张)/)?.[1];
  if (!chinese) return null;
  const values = { 一: 1, 二: 2, 三: 3, 四: 4, 五: 5, 六: 6, 七: 7, 八: 8, 九: 9 };
  let count = 0;
  if (chinese === "十") count = 10;
  else if (chinese.startsWith("十")) count = 10 + (values[chinese[1]] || 0);
  else if (chinese.includes("十")) count = (values[chinese[0]] || 0) * 10 + (values[chinese.split("十")[1]] || 0);
  else count = values[chinese] || 0;
  return count >= 2 && count <= UNIPPT_AI_HTML_MAX_PAGES ? count : null;
}

function preflightUniPptAiHtml(raw, expectedPageCount = null) {
  const sourceHtml = extractUniPptAiHtml(raw);
  const documentNode = new DOMParser().parseFromString(sourceHtml, "text/html");
  if (documentNode.querySelector("parsererror")) throw new Error("AI 生成的 HTML 语法不完整，已保持原文稿不变");
  const forbidden = documentNode.querySelector("script,iframe,object,embed,video,audio,form,link,base");
  if (forbidden) throw new Error(`AI 生成的 HTML 含不允许的 <${forbidden.tagName.toLowerCase()}>，已阻止执行`);
  const nodes = Array.from(documentNode.querySelectorAll("*"));
  if (nodes.length > UNIPPT_AI_HTML_MAX_NODES) throw new Error(`AI 生成的 HTML 节点过多（${nodes.length} > ${UNIPPT_AI_HTML_MAX_NODES}）`);
  if (documentNode.querySelector('meta[http-equiv="refresh" i]')) throw new Error("AI 生成的 HTML 含 meta refresh，已阻止跳转");
  for (const node of nodes) {
    for (const attribute of Array.from(node.attributes || [])) {
      const name = attribute.name.toLowerCase();
      const value = String(attribute.value || "").trim();
      if (/^on/.test(name)) throw new Error(`AI 生成的 HTML 含事件属性 ${attribute.name}，已阻止执行`);
      if (name === "srcset" && value) throw new Error("AI 生成的 HTML 含 srcset，已阻止潜在联网加载");
      if (["src", "href", "xlink:href", "poster", "action", "formaction"].includes(name) && value
        && !/^#/.test(value) && !/^data:image\//i.test(value)) {
        throw new Error(`AI 生成的 HTML 含不安全资源地址 ${attribute.name}，已阻止联网加载`);
      }
    }
  }
  const styleText = [
    ...Array.from(documentNode.querySelectorAll("style")).map((style) => style.textContent || ""),
    ...nodes.map((node) => node.getAttribute?.("style") || ""),
  ].join("\n");
  if (/@import\b/i.test(styleText)) throw new Error("AI 生成的 CSS 含 @import，已阻止联网加载");
  for (const match of styleText.matchAll(/url\(\s*["']?([^"')]+)["']?\s*\)/ig)) {
    const value = String(match[1] || "").trim();
    if (value && !/^#/.test(value) && !/^data:/i.test(value)) throw new Error("AI 生成的 CSS 含外部 url()，已阻止联网加载");
  }
  const pages = Array.from(documentNode.querySelectorAll(".omnidoc-page"));
  if (pages.length < 2 || pages.length > UNIPPT_AI_HTML_MAX_PAGES) {
    throw new Error(`AI 自由 HTML 必须包含 2—${UNIPPT_AI_HTML_MAX_PAGES} 个 .omnidoc-page；当前为 ${pages.length} 个`);
  }
  if (expectedPageCount && pages.length !== expectedPageCount) {
    throw new Error(`用户要求 ${expectedPageCount} 页，但模型只生成 ${pages.length} 页；已保持原文稿不变`);
  }
  if (pages.some((page) => page.querySelector(".omnidoc-page"))) throw new Error("AI 自由 HTML 的 .omnidoc-page 不能互相嵌套");
  let normalized = false;
  pages.forEach((page, index) => {
    if (!/^1280\s*[x×]\s*720$/i.test(String(page.getAttribute("data-page-size") || ""))) {
      throw new Error(`第 ${index + 1} 页缺少 data-page-size=\"1280x720\"`);
    }
    const title = page.querySelector('[data-unippt-role="cover-title"], [data-unippt-role="slide-title"], [data-unippt-role="closing-title"]');
    if (!title?.textContent?.trim()) throw new Error(`第 ${index + 1} 页缺少带语义标记的主标题`);
    const role = String(page.getAttribute("data-unippt-role") || "").trim();
    if (index === 0 && role !== "cover") throw new Error("第 1 页必须标记 data-unippt-role=\"cover\"");
    if (index === pages.length - 1 && role !== "closing") throw new Error(`第 ${pages.length} 页必须标记 data-unippt-role=\"closing\"`);
    let notesTemplate = page.querySelector("template[data-unippt-notes]");
    const notes = notesTemplate?.content?.textContent || "";
    if (!/\[Sources\][\s\S]*\[\/Sources\]/.test(notes)) {
      if (!notesTemplate) {
        notesTemplate = documentNode.createElement("template");
        notesTemplate.setAttribute("data-unippt-notes", "");
        page.append(notesTemplate);
      }
      notesTemplate.content.append(documentNode.createTextNode(
        `${notes.trim() ? "\n\n" : ""}[Sources]\n- No external sources; authored synthesis.\n[/Sources]`,
      ));
      normalized = true;
    }
  });
  const meta = (name, fallback = "") => String(documentNode.querySelector(`meta[name="${name}"]`)?.content || fallback).trim();
  return {
    sourceHtml: normalized ? `<!doctype html>\n${documentNode.documentElement.outerHTML}` : sourceHtml,
    pageCount: pages.length, nodeCount: documentNode.querySelectorAll("*").length,
    title: meta("unippt-title", documentNode.title || "U AI 演示文稿"),
    audience: meta("unippt-audience"), purpose: meta("unippt-purpose"),
    centralTakeaway: meta("unippt-takeaway"), language: String(documentNode.documentElement.lang || "zh-CN"),
  };
}

function uniPptAiSystemPrompt(mode) {
  const editing = mode === "edit";
  return [
    "你是 U AI，UniPPT 演示文稿编辑器内真实运行的 AI Agent。",
    "当前文档不是截图或扁平文本，而是可无损回写 PPTX 的结构化场景模型。",
    "你必须区分普通问答与文档操作：问候和咨询直接自然回答，不要擅自检查或修改文档。",
    "回复默认使用清晰的 Markdown 排版；数学内容使用 $...$ 行内 LaTeX 或 $$...$$ 块级 LaTeX，不要输出未经请求的原始 HTML。",
    "用户可能附加 PPTX、UDOC、HTML、文本或图片：文稿附件已转换为带页码的结构化摘要，图片已作为真实视觉输入发送；必须结合附件内容回答并明确引用文件名。",
    editing
      ? "当前为编排演示模式。需要读取或修改时必须调用已提供的工具；修改工具会先展示给用户确认。"
      : "当前为只读对话模式。你只能调用 presentation.inspect / presentation.validate，不能修改文档。",
    "绝不能声称已经创建、修改或修复，除非相应工具返回成功。工具失败或被拒绝时必须如实说明。",
    "整套新 PPT 的生成由宿主在普通工具循环之前执行“自由 HTML 生成 → 沙箱预检 → 浏览器布局编译 → 单次原子替换”，不要改用 presentation.compose、presentation.create 或逐页 slide.add。除非用户明确把已有 HTML 附件转为 PPT，否则不要调用 presentation.importFreeHtml；除非用户明确说删除/移除，否则禁止调用 slide.remove、object.remove 或包含删除操作的 ChangeSet。",
    "用户附加自由 HTML 并要求转为 PPT、幻灯片或可编辑演示时，必须只调用一次 presentation.importFreeHtml，attachmentName 必须匹配附件名。不要让模型复述或改写 HTML 坐标：宿主会用浏览器计算后的真实 DOM/CSS 布局，把文本、色块/边框、图片、SVG 和 Canvas 分层编译；脚本、iframe、无 poster 视频等不可靠内容会在 report 中明确列出。",
    "如果整稿请求到达本工具循环，说明宿主未识别为明确的新建意图；此时不得擅自替换整稿，应先自然说明需要明确提出生成或替换一整套演示。",
    "整套演示必须形成累进叙事而不是话题清单：通常按背景/利害关系 → 证据 → 含义/选择 → 行动展开；封面保持极简，结尾必须解决开场问题并给出决策、行动或综合结论。相邻页面使用不同但一致的轮廓。",
    "每页只承担一个叙事任务，标题必须是面向观众的明确结论而不是宽泛主题。可见文案全部服务于演讲受众，禁止出现生成步骤、规划说明、占位内容和模型自述。正文宁可删减或拆页，也不要缩小字号和堆满页面。",
    "视觉质量底线：全稿只使用一个 designPreset；封面标题至少 50pt、页面标题至少 35pt、正文至少 16pt；维持统一左右边距、颜色、字体和页码；避免重复卡片墙和伪交互 UI。用原生图表、流程、矩阵、对比或时间线把证据变成意义，不要把每页都做成项目符号列表。",
    "不得虚构数据、案例、来源或人物。凡是 metric/chart 的数字都必须在该页 sources 中提供可核查来源；没有可靠来源时改用定性论证，不要编造数字。外部来源会自动写入 speaker notes 的 [Sources] 块。",
    "宿主会对整稿自动执行结构与质量检查；如果宿主报告失败，必须如实说明当前文稿未改变，不能改用旧的固定模板整稿工具绕过检查。",
    "用户附图并要求转为可编辑 PPT、还原或提取图层时，只能调用一次 slide.reconstructFromImage，slide.mode=native。必须直接看原图识别所有模块、文字、旋转方向和连接关系，在 objects 中一次提交原生图层。坐标统一用原始图片像素，不转换成 1280×720；宿主等比适配。禁止整页底图、遮罩和重复叠图。照片用 sourceCrops，普通形状、层叠结构、边框、图标和箭头用原生对象。保留现有页面。",
    "OCR 仅是参考，必须纠正错字并补出漏检文字，特别是 90/270 度文字、公式、上下标。text 对象的 ocrIndex 指向所校正的 OCR 索引；没有 OCR 的新增文字不填 index。误检用 ignoredOcr:[{index,reason}] 明确排除，未处理的 OCR 会作为待校对文字保留。旋转文字用未旋转文本框宽高和 frame.rotation=-90/90；数学上下标用 textRuns 的 baseline=sub/super，不写成字面下划线。",
    "先清点图中的标签和旋转文字，在 verification 提供 labels（校正后的文字完整串）、rotatedTextCount、diagram。再按原图层级排列对象：底层边框、连接线、模块、标签。repeat:{count,dx,dy} 可压缩相同的层叠形状；多边形用 points，曲线用局部像素 M/L/C/Z path；箭头头部可用独立三角形。kind=connector 用于连接线，kind=shape 用于模块。textStyle 提供原图字体、字号、字重、颜色。sourceCrops 的 crop 是四边裁掉的 0–1 比例，每张小照片独立。不要输出长篇分析、不要调用 inspect 读取与原图无关的现有页面，直接提交完整方案。结构检查不是视觉验收，不得声称已经与原图完全一致。",
    "一般局部修改优先使用 slide/object/animation/transition 精准工具；多个关联修改必须合并到一次 presentation.applyChangeSet 原子提交，不要拆成一串小工具调用。",
    "新增单页时，slide.add.slide 可以直接携带 title、subtitle、bullets、objects、animations 和 transition，应一次提交完整页面，不要先建空白页再猜测生成的 slideId。",
    "若后续工具依赖前一个工具返回的新 ID，必须先单独执行前一个工具并等待结果，下一轮再使用返回的 slideId/objectId/animationId，禁止猜测 ID 或并行提交依赖调用。",
    "生成的页面必须给出真实内容、清晰视觉层级、可编辑对象、恰当的 PowerPoint 动画与切换效果，禁止空洞占位文案。",
    "完成后用简洁中文总结实际结果，并提醒用户所有修改都可 Ctrl+Z 撤销。",
  ].join("\n");
}

function uniPptAiContext(host) {
  const deck = host.document.getSummary({ includeObjects: false });
  const selection = host.selection.get();
  let active = null;
  const activeSlide = deck.slides?.[state.activeSlide] || deck.slides?.[0];
  if (activeSlide?.id) active = host.document.getSummary({ slideId: activeSlide.id, includeObjects: true });
  return {
    document: deck,
    activeSlide: active?.slides?.[0] || null,
    selection,
    selectedText: host.selection.getText(),
    revision: host.document.getRevision(),
  };
}

async function openAiAssistant(prefill = "") {
  try {
    await ensureFeature("ai");
    installDocumentHost();
    if (!state.documentHost) throw new Error("演示文稿宿主尚未就绪");
    openAiAssistantPanel(prefill);
  } catch (error) {
    toast(`AI 助手加载失败：${error.message || error}`);
  }
}

function openAiAssistantPanel(prefill = "") {
  let panel = document.getElementById("ai-panel");
  if (!panel) {
    panel = document.createElement("aside");
    panel.id = "ai-panel";
    panel.innerHTML = `
      <div class="ai-resizer" title="拖动调整面板宽度"></div>
      <header class="ai-head">
        <span class="ai-title"><img class="ai-logo" src="${startupAsset("/unippt-logo.svg")}" alt="UniPPT"><span class="ai-name">U AI</span><span class="ai-product">演示文稿助手</span></span>
        <button type="button" class="ai-gear" title="工具与上下文" aria-label="工具与上下文">⚙</button>
        <button type="button" class="ai-max" title="全屏铺满 / 还原" aria-label="全屏铺满或还原">⛶</button>
        <button type="button" class="ai-close" title="关闭" aria-label="关闭">×</button>
      </header>
      <div class="ai-cfg hidden">
        <strong>模型与结构化工具</strong><span class="ai-provider-status">正在检测内置模型通道…</span>
        <label>接口地址<input class="ai-base" type="url" placeholder="OpenAI 兼容 /v1"></label>
        <label>模型<input class="ai-model" type="text" placeholder="例如 qwen3.7-flash / gpt-5"></label>
        <label>API Key<input class="ai-key" type="password" autocomplete="off" placeholder="留空使用服务端内置通道"></label>
        <button type="button" class="ai-save-config">保存模型设置</button><button type="button" class="ai-test-config">测试连接</button>
        <p>API Key 留空时走同源服务端代理，密钥由 UNIPPT_AI_KEY 配置且绝不下发；填写 Key 时浏览器直连所填 OpenAI-compatible 接口。未连接真实模型时不会用固定话术冒充 AI。</p>
        <p>当前开放 <span class="ai-tool-count">0</span> 个结构化工具；读取立即执行，任何修改都会先请求确认，并以原子 ChangeSet 进入撤销栈。</p>
        <button type="button" class="ai-inspect">检查当前演示文稿</button>
      </div>
      <div class="ai-msgs">
        <div class="ai-hello">
          <img class="ai-hello-logo" src="${startupAsset("/unippt-logo.svg")}" alt="UniPPT">
          <div class="ai-hello-h">你好，今天想做什么演示？</div>
          <div class="ai-hello-sub">我是 U AI。我能读取当前演示，也能理解你附加的 PPTX、UDOC、HTML 和图片，并通过可撤销的结构化工具创建或精准修改 PPT。</div>
        </div>
      </div>
      <div class="ai-dock">
        <div class="ai-chips">
          <button type="button" class="ai-chip" data-q="检查当前演示的排版、文字溢出、动画目标和设计一致性，并给出可执行修复方案">✓ 检查文稿</button>
          <button type="button" class="ai-chip" data-mode="edit" data-q="优化当前幻灯片的视觉层级与排版，保留内容和原生动画语义">✦ 优化当前页</button>
          <button type="button" class="ai-chip" data-mode="edit" data-q="创建一份 8 页的产品发布演示，现代科技风，包含数据图表和逐项出现动画">▣ 生成 PPT</button>
        </div>
        <div class="ai-context"><span>已自动附上：当前页、选择对象、文稿语义与设计令牌</span></div>
        <div class="ai-composer">
          <textarea class="ai-input" rows="1" placeholder="给 U AI 发消息…（Enter 发送，Shift+Enter 换行）"></textarea>
          <div class="ai-attachments" hidden></div>
          <div class="ai-composer-tools">
          <button type="button" class="ai-attach-button" title="添加 PPTX、UDOC、HTML、图片或文本附件" aria-label="添加附件">＋</button>
          <input class="ai-attachment-input" type="file" accept=".pptx,.udoc,.html,.htm,image/*,.txt,.md,.json,.csv,.xml,.yaml,.yml" multiple hidden>
          <button type="button" class="ai-context-toggle" title="使用当前演示上下文" aria-pressed="true">◎</button>
            <div class="ai-modes">
              <label><input type="radio" name="unippt-ai-mode" value="chat" checked><span>对话</span></label>
              <label><input type="radio" name="unippt-ai-mode" value="edit"><span>编排演示</span></label>
            </div>
            <select class="ai-image-speed" aria-label="图片还原速度" title="快速首轮不等于精度通过；精修另行执行">
              <option value="fast">图片：快速首轮 · 目标 30s</option>
              <option value="precision">图片：完整精修 · 较慢</option>
            </select>
            <span class="ai-tools-spacer"></span>
            <button type="button" class="ai-send" title="发送" aria-label="发送"></button>
          </div>
        </div>
      </div>`;
    document.body.append(panel);

    const messages = panel.querySelector(".ai-msgs");
    const input = panel.querySelector(".ai-input");
    const send = panel.querySelector(".ai-send");
    const conversation = [];
    const attachments = [];
    const toolCards = new Map();
    const approvedMutationCapabilities = new WeakMap();
    const approvedDocumentDestinations = new WeakMap();
    let aiAbort = null;
    let aiFollowTail = true;
    const isAiNearTail = () => messages.scrollHeight - messages.scrollTop - messages.clientHeight <= 40;
    const scrollAiMessagesToEnd = (force = false) => {
      if (!force && !aiFollowTail) return;
      messages.scrollTop = messages.scrollHeight;
      aiFollowTail = true;
    };
    messages.addEventListener("scroll", () => { aiFollowTail = isAiNearTail(); }, { passive: true });
    const copyAiMessage = async (text) => {
      const value = String(text || "");
      if (!value) return;
      try {
        if (navigator.clipboard?.writeText) await navigator.clipboard.writeText(value);
        else {
          const field = element("textarea");
          field.value = value;
          field.setAttribute("readonly", "");
          field.style.position = "fixed";
          field.style.opacity = "0";
          document.body.append(field);
          field.select();
          document.execCommand("copy");
          field.remove();
        }
        toast("消息已复制");
      } catch (error) {
        toast(`复制失败：${error.message || "浏览器未授权剪贴板"}`);
      }
    };
    const renderAiMessage = (body, text, streaming = false) => {
      const source = String(text || "");
      const renderer = globalThis.UniPptAiMessageRenderer;
      if (renderer?.renderInto) renderer.renderInto(body, source, { streaming });
      else body.textContent = source;
      return source;
    };
    const appendMessage = (role, text, messageAttachments = []) => {
      panel.querySelector(".ai-hello")?.remove();
      const message = element("div", `ai-msg ai-${role}`);
      const avatar = element("span", "ai-avatar", role === "user" ? "你" : "");
      if (role === "assistant") {
        const logo = document.createElement("img");
        logo.src = startupAsset("/unippt-logo.svg");
        logo.alt = "UniPPT";
        avatar.append(logo);
      }
      const bubble = element("div", "ai-bubble");
      const body = element("div", "ai-body");
      const copy = element("button", "ai-message-copy", "复制");
      copy.type = "button";
      copy.title = "复制这条消息";
      copy.setAttribute("aria-label", "复制这条消息");
      if (role === "assistant") renderAiMessage(body, text);
      else body.textContent = String(text || "");
      bubble.append(body);
      if (messageAttachments.length) {
        const preview = element("div", "ai-message-attachments");
        for (const attachment of messageAttachments) {
          const item = element("span", `ai-message-attachment ${attachment.kind}`);
          if (attachment.kind === "image") {
            const image = document.createElement("img");
            image.src = attachment.dataUrl;
            image.alt = attachment.name;
            item.append(image);
          } else item.append(element("b", "", attachment.kind === "document" ? "P" : "T"));
          item.append(element("span", "", attachment.name));
          preview.append(item);
        }
        bubble.append(preview);
      }
      bubble.append(copy);
      message.append(avatar, bubble);
      messages.append(message);
      const entry = { message, bubble, body, copy, rawText: String(text || "") };
      copy.hidden = !entry.rawText;
      copy.onclick = () => copyAiMessage(entry.rawText);
      scrollAiMessagesToEnd(true);
      return entry;
    };
    const toolRequestKey = (request) => request.call?.id || `${request.name}:${JSON.stringify(request.args || {})}`;
    const updateToolCard = (event) => {
      const record = toolCards.get(toolRequestKey(event));
      if (!record) return;
      const { card, row } = record;
      const title = card.querySelector(":scope > strong");
      const allow = card.querySelector(".ai-tool-allow");
      const stateLabel = row.querySelector(".ai-tool-state");
      const toolUi = uniPptAiToolUi(event.name);
      if (event.phase === "completed") {
        row.dataset.phase = "completed";
        row.classList.add("completed");
        stateLabel.textContent = "已完成";
      } else if (event.phase === "failed") {
        row.dataset.phase = "failed";
        row.classList.add("failed");
        stateLabel.textContent = `失败：${event.result?.error || "未知错误"}`;
      } else if (event.phase === "denied") {
        row.dataset.phase = "denied";
        row.classList.add("denied");
        stateLabel.textContent = "已拒绝";
      }
      const rows = [...card.querySelectorAll(".ai-tool-batch-row")];
      const terminal = rows.filter((item) => ["completed", "failed", "denied"].includes(item.dataset.phase));
      if (terminal.length === rows.length) {
        const failures = rows.filter((item) => item.dataset.phase === "failed").length;
        const denied = rows.filter((item) => item.dataset.phase === "denied").length;
        card.classList.add(failures ? "failed" : denied ? "denied" : "completed");
        title.textContent = failures
          ? `${rows.length - failures} 项完成，${failures} 项失败`
          : denied ? `已拒绝 ${denied} 项修改` : `${rows.length} 项修改已全部完成`;
        allow.textContent = failures ? "部分失败" : denied ? "已拒绝" : "已完成";
      }
      scrollAiMessagesToEnd();
    };
    const appendToolBatchApproval = (reply, requests) => new Promise((resolve) => {
      const card = element("section", "ai-tool-card ai-tool-batch");
      const title = element("strong", "", requests.length === 1
        ? `请求执行：${uniPptAiToolUi(requests[0].name).label}`
        : `请求统一执行 ${requests.length} 项修改`);
      const list = element("div", "ai-tool-batch-list");
      for (const request of requests) {
        const row = element("div", "ai-tool-batch-row");
        row.dataset.phase = "pending";
        row.append(
          element("span", "ai-tool-name", uniPptAiToolUi(request.name).label),
          element("span", "ai-tool-state", "待确认"),
        );
        const detail = document.createElement("details");
        detail.append(element("summary", "", "查看参数"), element("pre", "", JSON.stringify(request.args, null, 2)));
        row.append(detail);
        list.append(row);
        toolCards.set(toolRequestKey(request), { card, row });
      }
      const actions = element("div", "ai-tool-actions");
      const deny = element("button", "ai-tool-deny", requests.length > 1 ? "全部拒绝" : "拒绝");
      const allow = element("button", "ai-tool-allow", requests.length > 1 ? `统一允许 ${requests.length} 项` : "允许执行");
      actions.append(deny, allow);
      card.append(title, list, actions);
      reply.bubble.append(card);
      scrollAiMessagesToEnd();
      const settle = (approved) => {
        deny.disabled = true;
        allow.disabled = true;
        card.classList.add(approved ? "approved" : "denied");
        title.textContent = approved ? `正在执行 ${requests.length} 项修改` : `已拒绝 ${requests.length} 项修改`;
        allow.textContent = approved ? "执行中…" : "已拒绝";
        card.querySelectorAll(".ai-tool-batch-row").forEach((row) => {
          row.dataset.phase = approved ? "queued" : "denied";
          row.querySelector(".ai-tool-state").textContent = approved ? "等待执行" : "已拒绝";
        });
        resolve(approved);
      };
      deny.onclick = () => settle(false);
      allow.onclick = (event) => {
        const trustedApproval = event instanceof MouseEvent
          && event.isTrusted
          && event.currentTarget === allow;
        if (!trustedApproval) {
          toast("已阻止脚本模拟 AI 修改批准");
          return;
        }
        const newDocumentRequests = requests.filter((request) => request.destination === "new-window");
        const opened = [];
        try {
          for (const request of newDocumentRequests) {
            const destination = globalThis.UniPptDocumentHandoff?.open?.({ timeoutMs: 60_000 });
            if (!destination) throw new Error("新文稿窗口运行时未加载，请刷新页面");
            approvedDocumentDestinations.set(request, destination);
            opened.push(destination);
          }
        } catch (error) {
          opened.forEach((destination) => destination.cancel?.("新窗口创建失败"));
          toast(error?.message || "无法打开新文稿窗口");
          return;
        }
        settle(true);
      };
    });
    const approveAiToolBatch = async (reply, requests) => {
      const approved = await appendToolBatchApproval(reply, requests);
      if (!approved) return false;
      const mutations = requests.filter((request) => !UNIPPT_AI_READ_ONLY_TOOLS.has(request.name));
      if (!mutations.length) return true;
      if (typeof aiMutationAuthorizer?.issue !== "function") {
        mutations.forEach((request) => approvedDocumentDestinations.get(request)?.cancel?.("修改授权器不可用"));
        throw new Error("演示文稿宿主没有提供 AI 修改授权器，请刷新页面");
      }
      let capability;
      try { capability = aiMutationAuthorizer.issue(mutations); }
      catch (error) {
        mutations.forEach((request) => approvedDocumentDestinations.get(request)?.cancel?.("修改授权签发失败"));
        throw error;
      }
      for (const request of mutations) approvedMutationCapabilities.set(request, capability);
      return true;
    };
    const approvedCallOptions = (request) => ({
      capability: approvedMutationCapabilities.get(request),
      destination: approvedDocumentDestinations.get(request),
    });
    const refreshProviderStatus = async () => {
      const statusEl = panel.querySelector(".ai-provider-status");
      const config = loadUniPptAiConfig();
      if (config.key) {
        statusEl.textContent = `自定义模型：${config.model || "未填写模型"}`;
        statusEl.className = "ai-provider-status ready";
        return;
      }
      const status = await globalThis.UniPptAiRuntime?.builtinStatus?.();
      if (status?.unreachable) {
        statusEl.textContent = "本地服务未连接：请重新启动 UniPPT 后刷新";
        statusEl.className = "ai-provider-status missing";
        return;
      }
      const ready = !!status?.configured;
      statusEl.textContent = ready
        ? `内置通道已连接：${status.model || "服务端模型"}`
        : "内置通道未配置：请设置 UNIPPT_AI_KEY 或填写自己的 API Key";
      statusEl.className = `ai-provider-status ${ready ? "ready" : "missing"}`;
    };
    const renderAttachments = () => {
      const bar = panel.querySelector(".ai-attachments");
      bar.hidden = !attachments.length;
      bar.replaceChildren(...attachments.map((attachment, index) => {
        const card = element("div", `ai-attachment ${attachment.kind}`);
        if (attachment.kind === "image") {
          const preview = document.createElement("img");
          preview.src = attachment.dataUrl;
          preview.alt = attachment.name;
          card.append(preview);
        } else card.append(element("span", "ai-attachment-type", attachment.format || "DOC"));
        const label = element("span", "ai-attachment-label");
        label.append(element("strong", "", attachment.name));
        label.append(element("small", "", [
          attachment.slideCount ? `${attachment.slideCount} 页` : "",
          globalThis.UniPptAiAttachments?.readableSize?.(attachment.size) || "",
        ].filter(Boolean).join(" · ")));
        const remove = element("button", "ai-attachment-remove", "×");
        remove.type = "button";
        remove.title = `移除 ${attachment.name}`;
        remove.onclick = () => { attachments.splice(index, 1); renderAttachments(); };
        card.append(label, remove);
        return card;
      }));
    };
    const addAttachmentFiles = async (files) => {
      const runtime = globalThis.UniPptAiAttachments;
      if (!runtime?.prepareFile) return toast("附件运行时未加载，请刷新页面");
      const button = panel.querySelector(".ai-attach-button");
      button.classList.add("loading");
      for (const file of Array.from(files || []).slice(0, Math.max(0, 8 - attachments.length))) {
        try {
          const attachment = await runtime.prepareFile(file);
          attachments.push(attachment);
          renderAttachments();
        } catch (error) { toast(`附件 ${file.name} 添加失败：${error.message}`); }
      }
      button.classList.remove("loading");
      if (attachments.length) input.focus();
    };
    const sendRequest = async () => {
      const prompt = input.value.trim();
      if (send.classList.contains("busy")) { aiAbort?.abort(); return; }
      if (!prompt && !attachments.length) return;
      const mode = panel.querySelector('input[name="unippt-ai-mode"]:checked')?.value || "chat";
      const submittedAttachments = attachments.splice(0);
      renderAttachments();
      const attachmentRuntime = globalThis.UniPptAiAttachments;
      if (!attachmentRuntime?.messagePayload) return toast("附件运行时未加载，请刷新页面");
      const payload = attachmentRuntime.messagePayload(prompt, submittedAttachments);
      appendMessage("user", payload.displayText, submittedAttachments);
      input.value = "";
      input.style.height = "auto";
      send.classList.add("busy");
      const reply = appendMessage("assistant", "");
      const thinking = element("div", "ai-thinking", "正在连接模型…");
      thinking.setAttribute("role", "status");
      thinking.setAttribute("aria-live", "polite");
      reply.message.classList.add("ai-processing");
      reply.bubble.insertBefore(thinking, reply.body);
      const thinkingByRound = new Map();
      const progressByRound = new Map();
      const appendProgressToken = (store, token, round = 0) => {
        const next = `${store.get(round) || ""}${token}`.slice(-1200);
        store.set(round, next);
        return next;
      };
      const setAiProgress = (label, value = "") => {
        const compact = String(value || "").replace(/\s+/g, " ").trim();
        const visible = compact.length > 220 ? `…${compact.slice(-219)}` : compact;
        thinking.textContent = visible ? `${label}：${visible}` : label;
        thinking.dataset.waiting = /等待.*确认/.test(label) ? 'true' : 'false';
        thinking.title = thinking.textContent;
        scrollAiMessagesToEnd();
      };
      const finishAiProcessing = () => {
        clearInterval(progressTimer);
        thinking.remove();
        reply.message.classList.remove("ai-processing");
      };
      const progressStartedAt = Date.now();
      const progressTimer = setInterval(() => {
        if (!thinking.isConnected) return;
        if (thinking.dataset.waiting === 'true') { thinking.dataset.elapsed = '等待确认'; return; }
        thinking.dataset.elapsed = `${Math.max(1, Math.floor((Date.now() - progressStartedAt) / 1000))}s`;
      }, 1000);
      aiAbort = new AbortController();
      const imagePrecisionStages = [];
      let imagePrecisionOcr = null;
      try {
        const allTools = state.documentHost.ai.tools;
        const imageAttachments = submittedAttachments.filter((attachment) => attachment.kind === "image");
        const htmlAttachments = submittedAttachments.filter((attachment) => attachment.kind === "html" && attachment.html);
        const imageReconstruction = imageAttachments.length > 0 && /(?:可编辑|转(?:为|成)?\s*(?:ppt|幻灯片)|还原|重建|提取.{0,8}(?:文本|图层|版式))/i.test(prompt);
        const localSkillPlan = globalThis.UniPptLocalSkills?.match?.({
          prompt, mode, attachments: submittedAttachments,
        }) || null;
        const freeHtmlImport = localSkillPlan?.id === "free-html-to-editable-ppt";
        const ocrDetections = new Map();
        const imageWorkflowStarted = imageReconstruction ? Date.now() : 0;
        const fastImageRound = panel.querySelector('.ai-image-speed')?.value !== 'precision';
        if (imageReconstruction) {
          if (!fastImageRound) {
            const qualityStatus = await fetch('/api/ai/reconstruction/status', {signal: aiAbort.signal}).then(r => r.ok ? r.json() : null);
            if (!qualityStatus?.available) throw new Error('本机尚未配置实际 PPTX 质量渲染器；已停止，未调用付费识别。');
          }
          if (imageAttachments.length !== 1) throw new Error("原生图片重建一次处理 1 张图片，请分别提交");
          if (!attachmentRuntime.detectImageText) throw new Error("OCR 检测运行时未加载，请刷新页面");
          for (const attachment of imageAttachments) {
            setAiProgress("正在定位图片文字", attachment.name);
            let detection;
            try {
              detection = await globalThis.UniPptImageReconstruction.detectAnchors(attachment, attachmentRuntime.detectImageText, { signal: aiAbort.signal, timeoutMs: fastImageRound ? 20000 : 45000 });
            } catch (error) {
              if (aiAbort.signal.aborted) throw error;
              detection = { image: { width: attachment.width, height: attachment.height }, regions: [], warning: error.message };
            }
            ocrDetections.set(attachment.name, detection);
            imagePrecisionOcr = detection;
            setAiProgress(
              "开始原生重建（无需整页擦字）",
              `${attachment.name} · ${detection.regions.length} 个文字框`,
            );
          }
        }
        const explicitDelete = /(?:删除|移除|清空|删掉|去掉).{0,12}(?:幻灯片|页面|对象|图层)|(?:幻灯片|页面|对象|图层).{0,12}(?:删除|移除|清空|删掉|去掉)/i.test(prompt);
        const explicitReplaceDeck = (freeHtmlImport
          || globalThis.UniPptLocalSkills?.isWholeDeckCreationIntent?.(prompt) === true)
          && !imageReconstruction;
        const generatedHtmlDeck = mode === "edit" && explicitReplaceDeck && !freeHtmlImport && !imageReconstruction;
        const tools = mode !== "edit"
          ? allTools.filter((tool) => UNIPPT_AI_READ_ONLY_TOOLS.has(tool.name))
          : imageReconstruction
            ? allTools.filter((tool) => UNIPPT_AI_READ_ONLY_TOOLS.has(tool.name) || tool.name === "slide.reconstructFromImage")
            : allTools.filter((tool) => {
                if (tool.name === "slide.reconstructFromImage") return false;
                if (tool.name === "presentation.importFreeHtml") return freeHtmlImport;
                if (["presentation.create", "presentation.compose"].includes(tool.name)) return false;
                if (["slide.remove", "object.remove"].includes(tool.name)) return explicitDelete;
                return true;
              });
        const useContext = panel.querySelector(".ai-context-toggle").getAttribute("aria-pressed") !== "false";
        const context = useContext ? uniPptAiContext(state.documentHost) : null;
        if (imageReconstruction && mode === "edit") {
          const reconstruction = globalThis.UniPptImagePrecision;
          if (!reconstruction?.prepare) throw new Error("自动视觉复核运行时未加载，请刷新页面");
          const attachment = imageAttachments[0];
          const tracer = globalThis.UniPptImageNativeTracer;
          const pixels = await tracer.browserPixels(attachment);
          const anchors = ocrDetections.get(attachment.name)?.anchors;
          const prepared = await (fastImageRound ? globalThis.UniPptImageFast : reconstruction).prepare({
            attachment, prompt, config: loadUniPptAiConfig(), signal: aiAbort.signal,
            deadline: imageWorkflowStarted + 29000,
            pixels,
            ...(anchors?.length ? { anchors, photos: tracer.detectPhotos(pixels,attachment.width,attachment.height),
              ...(!fastImageRound ? {anchorSheet: await tracer.browserAnchorSheet(attachment,anchors)} : {}),
              trace: (inventory) => tracer.trace(pixels,attachment.width,attachment.height,inventory) } : {}),
            ocrDetection: ocrDetections.get(attachment.name),
            encodePixels: reconstruction.encodePixels,
            decodePixels: reconstruction.decodePixels,
            renderNative: reconstruction.renderNative,
            onStage: (stage) => imagePrecisionStages.push(stage),
            preflight: (args) => state.documentHost.ai.previewImageReconstruction(args),
            onProgress: setAiProgress,
          });
          const preview = document.createElement("img");
          preview.src = prepared.preview;
          preview.alt = fastImageRound ? "快速首轮浏览器预览（未做 PPTX 渲染验收）" : "实际 PPTX 导出渲染预览（含精度测量，仍需人工检查）";
          preview.style.cssText = "display:block;width:100%;height:auto;margin:8px 0;border:1px solid #ddd";
          reply.bubble.append(preview);
          const request = { name: "slide.reconstructFromImage", args: { ...prepared.args, sourceImage: undefined },
            call: { id: `image-${Date.now()}` }, definition: allTools.find((t) => t.name === "slide.reconstructFromImage") };
          const workflowMs = Date.now() - imageWorkflowStarted;
          prepared.report.workflowMs = workflowMs;
          prepared.report.withinTarget = fastImageRound ? workflowMs <= 30000 : null;
          prepared.report.preparationAndOcrMs = Math.max(0, workflowMs - prepared.report.totalMs);
          const qualitySummary = `识别至预览 ${(workflowMs/1000).toFixed(1)} 秒（含 OCR，不含等待确认） · ${prepared.inventory.labels.length} 段文字 · ${prepared.report.unresolvedText} 处文字需复核`;
          const qualityNote = document.createElement("p");
          qualityNote.textContent = qualitySummary + (prepared.report.warnings.length ? ` · ${prepared.report.warnings.length} 项待复核提示` : "");
          reply.bubble.append(qualityNote);
          const reportLink = document.createElement("a");
          reportLink.textContent = "下载重建诊断";
          reportLink.download = attachment.name.replace(/\.[^.]+$/, '') + '_重建诊断.json';
          reportLink.href = URL.createObjectURL(new Blob([JSON.stringify({report:prepared.report,args:prepared.args,modelStages:imagePrecisionStages,ocr:ocrDetections.get(attachment.name)},null,2)],{type:'application/json'}));
          reply.bubble.append(reportLink);
          setAiProgress(fastImageRound ? "可编辑首轮已生成，等待确认（尚未精度验收）" : "实际 PPTX 已渲染，等待人工确认", qualitySummary);
          const approved = await approveAiToolBatch(reply, [request]);
          if (!approved) {
            updateToolCard({ phase: "denied", ...request });
            reply.rawText = "已取消重建，文稿未改变。";
          } else {
            if (aiAbort.signal.aborted) throw new DOMException("Aborted", "AbortError");
            try {
              const result = await state.documentHost.ai.callTool(request.name, {
                ...prepared.args, afterSlideId: context?.activeSlide?.id,
              }, approvedCallOptions(request));
              updateToolCard({ phase: "completed", ...request, result });
              const reconstructedSlideId = result.slideId || result.activeSlideId;
              if (reconstructedSlideId) {
                imageReconstructionSources.set(reconstructedSlideId, {name: attachment.name, dataUrl: attachment.dataUrl, width: attachment.width, height: attachment.height});
                while (imageReconstructionSources.size > 4) imageReconstructionSources.delete(imageReconstructionSources.keys().next().value);
              }
              reply.rawText = `${result.completionText || "已新增原生可编辑页面。"}\n\n${qualitySummary}。` + (fastImageRound
                ? `${prepared.report.withinTarget ? '本次在 30 秒目标内' : '本次超出 30 秒目标'}；${prepared.report.modelCompleted ? `Flash 首轮差异校对完成（${prepared.report.correctionsApplied} 处）` : 'Flash 校正未完成，本轮保留 OCR 草稿'}。未执行原生 PPTX 渲染验收。可用顶部 MCP 连接交给 Codex 修改，或选择“完整精修”重新提交图片。`
                : `已进行 ${prepared.report.history.length} 次实际 PPTX 渲染与位置校正，${prepared.report.modelCalls} 次模型请求。仍请核对公式和细节，不能视为精度全面通过。`) + '可 Ctrl+Z 撤销。';
            } catch (error) {
              updateToolCard({ phase: "failed", ...request, result: { error: error.message } });
              throw error;
            }
          }
          finishAiProcessing();
          renderAiMessage(reply.body, reply.rawText);
          if (approved) {
            const downloadPage = document.createElement('button');
            downloadPage.textContent = '下载这一页可编辑 PPTX';
            downloadPage.className = 'button';
            downloadPage.addEventListener('click', async () => {
              downloadPage.disabled = true;
              const started = performance.now();
              try {
                const deck = globalThis.UniPptPresentationHost.compileNativeImageDeck(prepared.args);
                const response = await fetch('/api/export-pptx', {method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({deck})});
                if (!response.ok) throw new Error((await response.text()).slice(0,300));
                downloadBlob(attachment.name.replace(/\.[^.]+$/, '') + '_网页版精度重建.pptx', await response.blob());
                downloadPage.textContent = `再次下载这一页 PPTX（导出 ${((performance.now()-started)/1000).toFixed(1)} 秒）`;
              } catch (error) { toast('下载失败：' + error.message); }
              finally { downloadPage.disabled = false; }
            });
            reply.bubble.append(downloadPage);
          }
          reply.copy.hidden = false;
          conversation.push({ role:"user", content:payload.conversationText }, { role:"assistant", content:reply.rawText });
          return;
        }
        if (localSkillPlan) {
          const localSkills = globalThis.UniPptLocalSkills;
          if (typeof localSkills?.run !== "function") throw new Error("U AI 本地 Skill 运行时未加载，请刷新页面");
          setAiProgress("已匹配本地 Skill", localSkillPlan.label);
          const attachment = localSkillPlan.attachment;
          if (!attachment?.html) throw new Error("自由 HTML Skill 找不到原始附件源码");
          if (typeof state.documentHost.ai.prepareFreeHtml !== "function") throw new Error("自由 HTML 预编译运行时未加载，请刷新页面");
          setAiProgress("正在预编译自由 HTML", `${attachment.name} · 批准后将立即装载`);
          const preparedImport = await state.documentHost.ai.prepareFreeHtml({
            ...localSkillPlan.args,
            attachmentName: attachment.name,
            sourceHtml: attachment.html,
          });
          setAiProgress("预编译完成", `${preparedImport.slideCount} 页 · 等待确认`);
          const localResult = await localSkills.run(localSkillPlan, {
            beforeTools(requests) {
              setAiProgress("等待确认", uniPptAiToolUi(requests[0].name).label);
              return approveAiToolBatch(reply, requests);
            },
            async callTool(name, args, request) {
              setAiProgress("正在装载预编译文稿", attachment.name);
              return state.documentHost.ai.callTool(name, {
                ...args,
                preparedId: preparedImport.preparedId,
              }, approvedCallOptions(request));
            },
            onToolState(event) {
              updateToolCard(event);
              const toolUi = uniPptAiToolUi(event.name);
              if (event.phase === "completed") {
                setAiProgress("已完成", toolUi.completed);
                toast(`${toolUi.completed}，可 Ctrl+Z 撤销`);
              } else if (event.phase === "failed") setAiProgress("执行失败", toolUi.label);
              else if (event.phase === "denied") setAiProgress("已拒绝", toolUi.label);
            },
          });
          finishAiProcessing();
          reply.rawText = localResult.text;
          renderAiMessage(reply.body, reply.rawText);
          reply.copy.hidden = false;
          conversation.push(
            { role: "user", content: payload.conversationText },
            { role: "assistant", content: reply.rawText },
          );
          return;
        }
        if (generatedHtmlDeck) {
          const runtime = globalThis.UniPptAiRuntime;
          const localSkills = globalThis.UniPptLocalSkills;
          if (!runtime?.completion) throw new Error("U AI 运行时未加载，请刷新页面");
          if (typeof localSkills?.run !== "function") throw new Error("U AI 本地 Skill 运行时未加载，请刷新页面");
          let receivedCharacters = 0;
          const thinkingTail = new Map();
          const targetDeck = state.deck;
          const targetRevision = Number(state.documentCache?.revision || 0);
          const targetTitle = String(state.deck?.title || "当前演示文稿");
          const assertTargetUnchanged = () => {
            const currentRevision = Number(state.documentCache?.revision || 0);
            if (state.deck !== targetDeck || currentRevision !== targetRevision) {
              throw new Error(`生成期间目标文稿“${targetTitle}”已改变，已阻止覆盖；请在当前文稿上重新发送生成请求`);
            }
          };
          setAiProgress("正在生成自由 HTML 整稿", "模型正在完成叙事、视觉与逐页来源");
          const generated = await runtime.completion({
            config: loadUniPptAiConfig(),
            messages: [
              { role: "system", content: uniPptAiHtmlAuthoringPrompt() },
              ...(context ? [{ role: "system", content: `【当前演示仅作语义参考；不要复制其坐标】\n${JSON.stringify(context)}` }] : []),
              ...conversation.slice(-10),
              { role: "user", content: payload.content },
            ],
            tools: [], signal: aiAbort.signal,
            maxTokens: 32768,
            onThinking(token) {
              const tail = appendProgressToken(thinkingTail, token, 0);
              setAiProgress("正在设计自由 HTML", tail);
            },
            onToken(token) {
              receivedCharacters += String(token || "").length;
              setAiProgress("正在接收自由 HTML", `${receivedCharacters.toLocaleString()} 字符`);
            },
          });
          assertTargetUnchanged();
          let prepared;
          try {
            prepared = preflightUniPptAiHtml(generated.content, requestedUniPptPageCount(prompt));
          } catch (error) {
            setAiProgress("正在修复 HTML 结构", error.message);
            let repairCharacters = 0;
            const repaired = await runtime.completion({
              config: loadUniPptAiConfig(),
              messages: [
                { role: "system", content: `${uniPptAiHtmlAuthoringPrompt()}\n你正在修复一份未通过 UniPPT 静态预检的 HTML。保持原主题、叙事和精确页数，只修复预检问题，并重新输出完整 HTML。` },
                { role: "user", content: `【原始需求】\n${prompt}\n\n【预检错误】\n${error.message}\n\n【待修复 HTML】\n${String(generated.content || "")}` },
              ],
              tools: [], signal: aiAbort.signal, maxTokens: 32768,
              onThinking(token) {
                const tail = appendProgressToken(thinkingTail, token, 1);
                setAiProgress("正在修复 HTML 结构", tail);
              },
              onToken(token) {
                repairCharacters += String(token || "").length;
                setAiProgress("正在接收结构修订稿", `${repairCharacters.toLocaleString()} 字符`);
              },
            });
            assertTargetUnchanged();
            prepared = preflightUniPptAiHtml(repaired.content, requestedUniPptPageCount(prompt));
          }
          if (typeof state.documentHost.ai.prepareFreeHtml !== "function") throw new Error("自由 HTML 预编译运行时未加载，请刷新页面");
          let preparedImport = null;
          for (let attempt = 0; attempt < 2; attempt += 1) {
            assertTargetUnchanged();
            setAiProgress(attempt ? "正在复检修订稿" : "正在预编译自由 HTML", `${prepared.pageCount} 页 · 批准后立即打开`);
            try {
              preparedImport = await state.documentHost.ai.prepareFreeHtml({
                attachmentName: `${String(prepared.title || "U AI 演示文稿").replace(/[\\/:*?"<>|]+/g, "-").slice(0, 72) || "U AI 演示文稿"}.html`,
                title: prepared.title, pageCount: prepared.pageCount,
                sourceHtml: prepared.sourceHtml, generatedByAi: true,
                audience: prepared.audience, purpose: prepared.purpose,
                centralTakeaway: prepared.centralTakeaway, language: prepared.language,
              });
              break;
            } catch (error) {
              if (attempt > 0 || error?.code !== "AI_HTML_QUALITY_FAILED") throw error;
              const findings = Array.isArray(error.details) ? error.details.slice(0, 16) : [];
              const repairSummary = findings.map((finding) => ({
                code: finding.code, slide: Number.isInteger(finding.slideIndex) ? finding.slideIndex + 1 : null,
                message: finding.message,
              }));
              setAiProgress("质量门禁要求修订", repairSummary.map((finding) => finding.code).join("、") || "正在修复版式");
              let repairedCharacters = 0;
              const repaired = await runtime.completion({
                config: loadUniPptAiConfig(),
                messages: [
                  { role: "system", content: `${uniPptAiHtmlAuthoringPrompt()}\n你正在修订一份未通过 UniPPT 浏览器编译质量门禁的 HTML。必须保持原主题、叙事和精确页数，逐项修复 findings，并重新输出完整 HTML。` },
                  { role: "user", content: `【原始需求】\n${prompt}\n\n【质量 findings】\n${JSON.stringify(repairSummary)}\n\n【待修订 HTML】\n${prepared.sourceHtml}` },
                ],
                tools: [], signal: aiAbort.signal, maxTokens: 32768,
                onThinking(token) {
                  const tail = appendProgressToken(thinkingTail, token, 1);
                  setAiProgress("正在修订自由 HTML", tail);
                },
                onToken(token) {
                  repairedCharacters += String(token || "").length;
                  setAiProgress("正在接收修订稿", `${repairedCharacters.toLocaleString()} 字符`);
                },
              });
              assertTargetUnchanged();
              prepared = preflightUniPptAiHtml(repaired.content, requestedUniPptPageCount(prompt));
            }
          }
          if (!preparedImport?.preparedId) throw new Error("AI 自由 HTML 修订未产生可提交结果");
          const safeTitle = String(prepared.title || "U AI 演示文稿").replace(/[\\/:*?"<>|]+/g, "-").slice(0, 72) || "U AI 演示文稿";
          const attachment = {
            kind: "html", name: `${safeTitle}.html`, format: "AI FREE HTML",
            size: prepared.sourceHtml.length, slideCount: prepared.pageCount, html: prepared.sourceHtml,
          };
          const generatedPlan = {
            id: "free-html-to-editable-ppt",
            label: "AI 自由 HTML → 可编辑 UniPPT",
            description: "模型自由设计整套静态 HTML；宿主已完成浏览器布局编译与质量检查，批准后在新窗口立即装载。",
            toolName: "presentation.importFreeHtml",
            destination: "new-window",
            attachment,
            args: {
              attachmentName: attachment.name,
              title: prepared.title,
              pageCount: prepared.pageCount,
              nodeCount: prepared.nodeCount,
              sourceBytes: uniPptUtf8Bytes(prepared.sourceHtml),
              targetTitle,
            },
          };
          setAiProgress("预编译与质量检查完成", `${prepared.pageCount} 页 · 等待确认`);
          const localResult = await localSkills.run(generatedPlan, {
            beforeTools(requests) {
              setAiProgress("等待确认", `在新窗口打开 ${prepared.pageCount} 页演示文稿；“${targetTitle}”保持不变`);
              return approveAiToolBatch(reply, requests);
            },
            async callTool(name, args, request) {
              setAiProgress("正在打开新文稿", `${prepared.pageCount} 页已预编译完成`);
              try {
                return await state.documentHost.ai.callTool(name, {
                  ...args,
                  preparedId: preparedImport.preparedId,
                }, approvedCallOptions(request));
              } catch (error) {
                approvedDocumentDestinations.get(request)?.cancel?.(error?.message || "新文稿打开失败");
                throw error;
              }
            },
            onToolState(event) {
              updateToolCard(event);
              const toolUi = uniPptAiToolUi(event.name);
              if (event.phase === "completed") {
                setAiProgress("已完成", toolUi.completed);
                toast("新演示文稿已在独立窗口打开；当前文稿保持不变");
              } else if (event.phase === "failed") setAiProgress("执行失败", toolUi.label);
              else if (event.phase === "denied") setAiProgress("已拒绝", toolUi.label);
            },
          });
          const quality = localResult.result?.quality;
          const warnings = quality?.findings?.filter((finding) => finding.severity !== "error") || [];
          finishAiProcessing();
          reply.rawText = localResult.denied
            ? localResult.text
            : [
                localResult.text,
                `自由 HTML 整稿质量：${quality?.score ?? "已检查"}${quality ? "/100" : ""}${warnings.length ? `；仍有 ${warnings.length} 条非阻断建议` : "；未发现阻断问题"}。`,
                "完整 HTML 未写入聊天历史；新窗口中的页面均已转换为可编辑 UniPPT 对象，原文稿保持不变。",
              ].join("\n\n");
          renderAiMessage(reply.body, reply.rawText);
          reply.copy.hidden = false;
          conversation.push(
            { role: "user", content: payload.conversationText },
            { role: "assistant", content: reply.rawText },
          );
          return;
        }
        const requestMessages = [
          { role: "system", content: uniPptAiSystemPrompt(mode) },
          ...(context && !imageReconstruction ? [{ role: "system", content: `【当前演示上下文】\n${JSON.stringify(context)}` }] : []),
          ...(ocrDetections.size ? [{ role: "system", content: `【OCR 参考检测结果，可由原图校正】\n${JSON.stringify(Array.from(ocrDetections.entries()).map(([attachmentName, detection]) => ({ attachmentName, image: detection.image, model: detection.model, regions: detection.regions.map((region, index) => ({ index, text: region.text, location: region.location })) })))}\nlocation 为原图四点坐标。原生重建不得丢弃视觉识别补出的文字，完整重建形状和连接线，原图尺寸为 ${imageAttachments[0]?.width}×${imageAttachments[0]?.height}。` }] : []),
          ...conversation.slice(imageReconstruction ? conversation.length : -20),
          { role: "user", content: payload.content },
        ];
        const runtime = globalThis.UniPptAiRuntime;
        if (!runtime?.run) throw new Error("U AI 运行时未加载，请刷新页面");
        const detail = {
          prompt: payload.displayText, mode, toolNames: tools.map((tool) => tool.name),
          selection: state.documentHost.selection.get(),
          attachments: submittedAttachments.map(({ kind, name, size, format, slideCount }) => ({ kind, name, size, format, slideCount })),
        };
        globalThis.dispatchEvent(new CustomEvent("unippt:ai-request", { detail }));
        const callAiTool = async (name, args = {}, request = null) => {
            if (name === "slide.reconstructFromImage") {
              const requestedName = String(args.attachmentName || "");
              const attachment = imageAttachments.find((item) => item.name === requestedName)
                || (imageAttachments.length === 1 ? imageAttachments[0] : null);
              if (!attachment) throw new Error(`找不到图片附件 ${requestedName || "（未指定）"}`);
              return state.documentHost.ai.callTool(name, {
                ...args,
                attachmentName: attachment.name,
                sourceImage: attachment.dataUrl,
                sourceSize: { width: attachment.width, height: attachment.height },
                slide: { ...args.slide, mode: "native", preserveSourceImage: false },
                ocrDetection: ocrDetections.get(attachment.name),
                afterSlideId: args.afterSlideId || context?.activeSlide?.id,
              }, approvedCallOptions(request));
            }
            if (name === "presentation.importFreeHtml") {
              if (!freeHtmlImport) throw new Error("未获得把自由 HTML 转换为整份 PPT 的明确指令");
              const requestedName = String(args.attachmentName || "");
              const attachment = htmlAttachments.find((item) => item.name === requestedName)
                || (htmlAttachments.length === 1 ? htmlAttachments[0] : null);
              if (!attachment) throw new Error(`找不到自由 HTML 附件 ${requestedName || "（未指定）"}`);
              return state.documentHost.ai.callTool(name, {
                ...args, attachmentName: attachment.name, sourceHtml: attachment.html,
              }, approvedCallOptions(request));
            }
            if (["presentation.create", "presentation.compose", "presentation.importFreeHtml"].includes(name) && !explicitReplaceDeck) throw new Error("未获得替换整份演示文稿的明确指令");
            if (["slide.remove", "object.remove"].includes(name) && !explicitDelete) throw new Error("未获得删除页面或对象的明确指令");
            if (name === "presentation.applyChangeSet" && !explicitDelete) {
              const destructive = (args.operations || []).some((operation) => /^(?:removeSlide|removeObject|removeAnimation)$/.test(String(operation?.op || "")));
              if (destructive) throw new Error("本轮未获得删除内容的明确指令，已阻止破坏性事务");
            }
            return state.documentHost.ai.callTool(name, args, approvedCallOptions(request));
        };
        const result = await runtime.run({
            config: loadUniPptAiConfig(),
            messages: requestMessages,
            tools,
            callTool: callAiTool,
            signal: aiAbort.signal,
            maxRounds: imageReconstruction ? 3 : 5,
            maxTokens: imageReconstruction ? 24000 : undefined,
            thinking: imageReconstruction ? true : undefined,
            thinkingBudget: imageReconstruction ? 2048 : undefined,
            timeoutMs: 180000,
            terminalTools: imageReconstruction ? ["slide.reconstructFromImage"] : [],
            preflightTool(name, args) {
              if (name !== "slide.reconstructFromImage") return;
              const attachment = imageAttachments.find((item) => item.name === args.attachmentName) || imageAttachments[0];
              state.documentHost.ai.previewImageReconstruction({ ...args, sourceImage: attachment.dataUrl,
                sourceSize: { width: attachment.width, height: attachment.height }, ocrDetection: ocrDetections.get(attachment.name) });
            },
            onThinking(token, round) {
              const streamedThinking = appendProgressToken(thinkingByRound, token, round);
              setAiProgress(imageReconstruction ? "正在分析图片并规划可编辑图层" : "正在思考", streamedThinking);
            },
            onToken(token, round) {
              appendProgressToken(progressByRound, token, round);
              setAiProgress(imageReconstruction ? "正在生成结构化重建方案" : "正在组织回答");
            },
            onToolArguments(bytes) {
              setAiProgress("正在接收结构化对象", `${(bytes / 1024).toFixed(1)} KiB · 完整接收并校验后执行`);
            },
            beforeTools(requests) {
              const mutations = requests.filter((request) => !UNIPPT_AI_READ_ONLY_TOOLS.has(request.name));
              if (!mutations.length) return true;
              setAiProgress("等待确认", mutations.length === 1 ? uniPptAiToolUi(mutations[0].name).label : `${mutations.length} 项修改`);
              return approveAiToolBatch(reply, mutations);
            },
            onToolState(event) {
              updateToolCard(event);
              const toolUi = uniPptAiToolUi(event.name);
              if (event.phase === "requested") setAiProgress("准备执行", toolUi.label);
              else if (event.phase === "completed") setAiProgress("已完成", toolUi.completed);
              else if (event.phase === "failed") setAiProgress("执行失败", toolUi.label);
              else if (event.phase === "denied") setAiProgress("已拒绝", toolUi.label);
              if (event.phase === "completed" && !toolUi.readOnly) toast(`${toolUi.completed}，可 Ctrl+Z 撤销`);
              else if (event.phase === "failed") toast(`${toolUi.label}失败：${event.result?.error || "未知错误"}`);
            },
        });
        finishAiProcessing();
        reply.rawText = result.text || "模型已完成请求，但没有返回说明。";
        renderAiMessage(reply.body, reply.rawText);
        reply.copy.hidden = false;
        conversation.push({ role: "user", content: payload.conversationText }, { role: "assistant", content: reply.rawText });
      } catch (error) {
        finishAiProcessing();
        if (imagePrecisionStages.length) {
          const link = document.createElement('a'); link.textContent = '下载失败诊断'; link.download = '图片重建_失败诊断.json';
          link.href = URL.createObjectURL(new Blob([JSON.stringify({error:error.message,modelStages:imagePrecisionStages,ocr:imagePrecisionOcr},null,2)],{type:'application/json'}));
          reply.bubble.append(link);
        }
        if (error.name === "AbortError") reply.rawText += reply.rawText ? "\n（已停止）" : "已停止本次请求。";
        else {
          reply.rawText += `${reply.rawText ? "\n" : ""}执行失败：${error.message}`;
          reply.message.classList.add("ai-error");
        }
        renderAiMessage(reply.body, reply.rawText);
        reply.copy.hidden = false;
      } finally {
        finishAiProcessing();
        aiAbort = null;
        send.classList.remove("busy");
        scrollAiMessagesToEnd();
      }
    };
    send.onclick = sendRequest;
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        void sendRequest();
      }
    });
    input.addEventListener("input", () => {
      input.style.height = "auto";
      input.style.height = `${Math.min(200, input.scrollHeight)}px`;
    });
    panel.querySelector(".ai-chips").onclick = (event) => {
      const chip = event.target.closest(".ai-chip");
      if (!chip) return;
      if (chip.dataset.mode === "edit") panel.querySelector('input[value="edit"]').checked = true;
      input.value = chip.dataset.q || "";
      input.dispatchEvent(new Event("input"));
      input.focus();
    };
    panel.querySelector(".ai-close").onclick = () => panel.classList.add("hidden");
    panel.querySelector(".ai-max").onclick = () => panel.classList.toggle("ai-full");
    panel.querySelector(".ai-gear").onclick = () => panel.querySelector(".ai-cfg").classList.toggle("hidden");
    const config = loadUniPptAiConfig();
    panel.querySelector(".ai-base").value = config.base;
    panel.querySelector(".ai-model").value = config.model;
    panel.querySelector(".ai-key").value = config.key;
    panel.querySelector(".ai-save-config").onclick = async () => {
      const button = panel.querySelector(".ai-save-config");
      const next = {
        base: panel.querySelector(".ai-base").value.trim(),
        model: panel.querySelector(".ai-model").value.trim(),
        key: panel.querySelector(".ai-key").value.trim(),
      };
      button.disabled = true;
      try {
        if (next.key) {
          const response = await fetch("/api/ai/config", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(next),
          });
          if (!response.ok) throw new Error((await response.text()).slice(0, 500) || `HTTP ${response.status}`);
          try { localStorage.setItem(UNIPPT_AI_CONFIG_KEY, JSON.stringify({ ...next, key: "" })); } catch (_) {}
          panel.querySelector(".ai-key").value = "";
          toast("模型设置已保存到本机服务，重启和其他浏览器也可使用");
        } else {
          try { localStorage.setItem(UNIPPT_AI_CONFIG_KEY, JSON.stringify(next)); } catch (_) {}
          toast("已切换为服务端模型通道");
        }
        panel.querySelector(".ai-cfg").classList.add("hidden");
        await refreshProviderStatus();
      } catch (error) {
        try { localStorage.setItem(UNIPPT_AI_CONFIG_KEY, JSON.stringify(next)); } catch (_) {}
        toast(`服务端持久化失败，已仅保存到当前浏览器：${error.message}`);
      } finally { button.disabled = false; }
    };
    panel.querySelector(".ai-test-config").onclick = async () => {
      const button = panel.querySelector(".ai-test-config");
      button.disabled = true;
      try {
        const result = await globalThis.UniPptAiRuntime.run({
          config: loadUniPptAiConfig(),
          messages: [{ role: "user", content: "只回复：连接成功" }],
          tools: [],
          maxRounds: 1,
        });
        toast(result.text || "模型连接成功");
      } catch (error) { toast(`模型连接失败：${error.message}`); }
      finally { button.disabled = false; }
    };
    panel.querySelector(".ai-inspect").onclick = () => {
      const issues = state.documentHost.document.validate();
      appendMessage("assistant", issues.length
        ? `检查完成：发现 ${issues.length} 个排版或结构提示。已准备好通过“编排演示”逐项修复。`
        : "检查完成：当前演示文稿结构、动画目标与对象边界均通过校验。");
    };
    const attachmentInput = panel.querySelector(".ai-attachment-input");
    panel.querySelector(".ai-attach-button").onclick = () => attachmentInput.click();
    attachmentInput.onchange = () => {
      void addAttachmentFiles(attachmentInput.files);
      attachmentInput.value = "";
    };
    const composer = panel.querySelector(".ai-composer");
    composer.addEventListener("dragover", (event) => {
      if (![...(event.dataTransfer?.types || [])].includes("Files")) return;
      event.preventDefault();
      event.stopPropagation();
      composer.classList.add("dragging-files");
    });
    composer.addEventListener("dragleave", () => composer.classList.remove("dragging-files"));
    composer.addEventListener("drop", (event) => {
      const files = Array.from(event.dataTransfer?.files || []);
      if (!files.length) return;
      event.preventDefault();
      event.stopPropagation();
      composer.classList.remove("dragging-files");
      void addAttachmentFiles(files);
    });
    input.addEventListener("paste", (event) => {
      const files = Array.from(event.clipboardData?.files || []);
      if (!files.length) return;
      event.preventDefault();
      void addAttachmentFiles(files);
    });
    panel.querySelector(".ai-context-toggle").onclick = (event) => {
      const pressed = event.currentTarget.getAttribute("aria-pressed") !== "false";
      event.currentTarget.setAttribute("aria-pressed", String(!pressed));
      panel.querySelector(".ai-context").classList.toggle("disabled", pressed);
    };
    const resizer = panel.querySelector(".ai-resizer");
    resizer.onpointerdown = (event) => {
      event.preventDefault();
      const move = (next) => {
        panel.style.width = `${Math.max(320, Math.min(innerWidth - 12, innerWidth - next.clientX))}px`;
      };
      const up = () => {
        removeEventListener("pointermove", move);
        removeEventListener("pointerup", up);
        try { localStorage.setItem("unippt-ai-width", String(panel.offsetWidth)); } catch (_) {}
      };
      addEventListener("pointermove", move);
      addEventListener("pointerup", up);
    };
    try {
      const saved = Number(localStorage.getItem("unippt-ai-width"));
      if (saved >= 320) panel.style.width = `${Math.min(innerWidth - 12, saved)}px`;
    } catch (_) {}
    panel.querySelector(".ai-tool-count").textContent = String(state.documentHost.ai.tools.length);
    void refreshProviderStatus();
  }
  panel.classList.remove("hidden");
  panel.querySelector(".ai-tool-count").textContent = String(state.documentHost?.ai.tools.length || 0);
  const input = panel.querySelector(".ai-input");
  if (prefill) {
    input.value = prefill;
    input.dispatchEvent(new Event("input"));
  }
  requestAnimationFrame(() => input.focus());
  return panel;
}

function bindBrowserGuards() {
  document.addEventListener("contextmenu", (event) => event.preventDefault(), { capture: true });
  document.addEventListener("auxclick", (event) => event.preventDefault(), { capture: true });
  document.addEventListener("mousedown", (event) => {
    if (event.button > 1) event.preventDefault();
  }, { capture: true });
  for (const type of ["gesturestart", "gesturechange", "gestureend"]) {
    document.addEventListener(type, (event) => event.preventDefault(), { capture: true, passive: false });
  }
  document.addEventListener("wheel", (event) => {
    if (!(event.ctrlKey || event.metaKey)) return;
    event.preventDefault();
    if (document.querySelector("dialog[open]")) return;
    if (!state.deck || event.deltaY === 0) return;
    rememberCanvasPointer(event);
    adjustZoom(event.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP, {
      clientX: event.clientX,
      clientY: event.clientY,
    });
  }, { capture: true, passive: false });
  document.addEventListener("dragstart", (event) => {
    if (!isTextEditingTarget(event.target)) event.preventDefault();
  }, { capture: true });
  for (const type of ["dragenter", "dragover"]) {
    document.addEventListener(type, (event) => {
      if (isTextEditingTarget(event.target) && !transferContainsFiles(event.dataTransfer)) return;
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
    }, { capture: true });
  }
  document.addEventListener("drop", handleApplicationDrop, { capture: true });
  document.addEventListener("selectstart", (event) => {
    if (!isTextEditingTarget(event.target)) event.preventDefault();
  }, { capture: true });
}

function bindRibbonCommands() {
  $("#copyObject").onclick = copySelectedObject;
  $("#cutObject").onclick = cutSelectedObject;
  $("#pasteObject").onclick = () => void pasteObject();
  $("#formatPainter").onclick = activateFormatPainter;
  $("#applySlideLayout").onclick = applyBasicSlideLayout;
  $("#resetSlideLayout").onclick = resetSlideLayout;
  $("#paragraphBullets").onclick = () => toggleParagraphList("bullet");
  $("#paragraphNumbering").onclick = () => toggleParagraphList("number");
  $("#paragraphOutdent").onclick = () => adjustParagraphLevel(-1);
  $("#paragraphIndent").onclick = () => adjustParagraphLevel(1);
  $("#paragraphSpacing").onclick = cycleParagraphSpacing;
  $("#alignLeft").onclick = () => setParagraphAlignment("left");
  $("#alignCenter").onclick = () => setParagraphAlignment("center");
  $("#alignRight").onclick = () => setParagraphAlignment("right");
  $("#alignJustify").onclick = () => setParagraphAlignment("justify");
  $("#verticalAlign").onclick = cycleVerticalAlignment;
  $("#ribbonFontColor").onclick = () => openSceneColorPicker("#ribbonFontColor", {
    label: "字体颜色",
    current: () => selectedObject()?.textStyle?.color,
    autoLabel: "自动（黑色）",
    autoColor: "#000000",
    onPick: (color) => updateSelected((object) => applyTextStyle(object, "color", color)),
  });
  $("#ribbonFontColorInput").oninput = (event) => updateSelected((object) => applyTextStyle(object, "color", event.target.value));
  $("#shapeFill").onclick = () => openSceneColorPicker("#shapeFill", {
    label: "形状填充",
    current: () => selectedObject()?.style?.fill,
    autoLabel: "无填充色",
    autoColor: "transparent",
    onPick: (color) => updateSelected((object) => {
      object.style.fill = color;
      object.style.gradient = null;
      object.shapeFillAsset = null;
    }),
  });
  $("#shapeFillInput").oninput = (event) => updateSelected((object) => { object.style.fill = event.target.value; object.style.gradient = null; object.shapeFillAsset = null; });
  $("#shapeOutline").onclick = () => openSceneColorPicker("#shapeOutline", {
    label: "形状轮廓",
    current: () => selectedObject()?.style?.stroke,
    autoLabel: "无轮廓",
    autoColor: "transparent",
    onPick: (color) => updateSelected((object) => {
      object.style.stroke = color;
      object.style.strokeWidth = color === "transparent" ? 0 : Math.max(1, Number(object.style.strokeWidth) || 1);
    }),
  });
  $("#shapeOutlineInput").oninput = (event) => updateSelected((object) => { object.style.stroke = event.target.value; object.style.strokeWidth = Math.max(1, Number(object.style.strokeWidth) || 1); });
  $("#bringToFront").onclick = () => arrangeSelectedObject("front");
  $("#findText").onclick = () => openFindReplace(false);
  $("#replaceText").onclick = () => openFindReplace(true);
  $("#findNext").onclick = findNextText;
  $("#replaceAll").onclick = replaceAllText;
  $("#findQuery").oninput = () => { state.findCursor = -1; updateFindResultCount(); };
  $("#selectNextObject").onclick = selectNextObject;
  $("#cycleThemeColor").onclick = cycleThemeColor;
  $("#cycleThemeFont").onclick = cycleThemeFont;
  $("#toggleThemeEffects").onclick = toggleThemeEffects;
  $("#toggleSlideSize").onclick = toggleSlideSize;
  $("#slideBackground").onclick = () => openSceneColorPicker("#slideBackground", {
    label: "幻灯片背景",
    requiresObject: false,
    current: () => currentSlide()?.background,
    autoLabel: "默认（白色）",
    autoColor: "#FFFFFF",
    onPick: (color) => mutate(() => {
      currentSlide().background = color;
      currentSlide().backgroundAsset = null;
    }),
  });
  $("#slideBackgroundInput").oninput = (event) => mutate(() => { currentSlide().background = event.target.value; currentSlide().backgroundAsset = null; });
  $("#openHelp").onclick = () => openHelpDialog();
  $("#showFeatureStatus").onclick = showFeatureStatus;
  $("#sendFeedback").onclick = () => toast("反馈入口已启用：请在当前任务中直接描述问题并附截图");
  $("#spellCheck").onclick = startSpellCheck;
  $("#accessibilityCheck").onclick = runAccessibilityCheck;
  $("#accessibilityStatus").onclick = runAccessibilityCheck;
  $("#focusNotes").onclick = focusSlideNotes;
  $("#notesStatus").onclick = focusSlideNotes;
  $("#normalViewRibbon").onclick = showNormalView;
  $("#outlineViewRibbon").onclick = showOutlineView;
  $("#slideSorterView").onclick = showSlideSorterView;
  $("#notesView").onclick = focusSlideNotes;
  $("#toggleGridLines").onclick = toggleGridLines;
  $("#slidesPaneTab").onclick = showSlidesPane;
  $("#outlinePaneTab").onclick = showOutlinePane;
}

function openSceneColorPicker(anchorSelector, options) {
  if (options.requiresObject !== false && !selectedObject()) return toast("请先选择要设置格式的对象");
  const anchor = $(anchorSelector);
  const picker = globalThis.UniPptColorPicker;
  if (!anchor || !picker?.open) return;
  anchor.classList.add("color-command");
  picker.open(anchor, {
    label: options.label,
    current: options.current?.() || "#172033",
    autoLabel: options.autoLabel,
    autoColor: options.autoColor,
    onPick: options.onPick,
  });
}

function handleApplicationDrop(event) {
  if (isTextEditingTarget(event.target) && !transferContainsFiles(event.dataTransfer)) return;
  event.preventDefault();
  event.stopPropagation();
  const files = [...(event.dataTransfer?.files || [])];
  if (!files.length) return;
  const file = files[0];
  if (/\.(pptx|udoc|html?)$/i.test(file.name)) {
    void openFile(file);
    return;
  }
  if (file.type.startsWith("image/") && event.target.closest?.("#canvasViewport")) {
    addImageFile(file);
    return;
  }
  toast("仅支持打开 PPTX、UDOC、无损 HTML，或向画布插入图片");
}

function transferContainsFiles(dataTransfer) {
  return [...(dataTransfer?.types || [])].includes("Files");
}

function isTextEditingTarget(target) {
  const element = target instanceof Element ? target : target?.parentElement;
  return element instanceof Element && Boolean(element.closest(
    "input,textarea,select,[contenteditable=true],.editing,.editing-content",
  ));
}

function activateRibbon(name) {
  $$("[data-ribbon]").forEach((button) => button.classList.toggle("active", button.dataset.ribbon === name));
  $$("[data-panel]").forEach((panel) => panel.classList.toggle("active", panel.dataset.panel === name));
  if (name === "transitions") syncTransitionRibbon();
}

function bindInkTools() {
  $$('[data-draw-tool]').forEach((button) => {
    button.onclick = () => setInkTool(button.dataset.drawTool);
  });
  $("#drawColor").oninput = (event) => {
    state.inkColor = normalizeColor(event.target.value, "#202124");
  };
  $("#drawWidth").oninput = (event) => {
    const tool = state.inkTool === "highlighter" ? "highlighter" : state.inkTool === "eraser" ? "eraser" : "pen";
    state.inkWidths[tool] = clamp(Number(event.target.value) || 1, 1, 24);
    syncInkToolbar();
  };
  $("#convertInkShape").onclick = convertSelectedInkToShape;
  $("#clearSlideInk").onclick = clearSlideInk;
  globalThis.UniPptColorPicker?.attachInput($("#drawColor"), {
    label: "墨迹颜色",
    current: () => state.inkColor,
    onPick: (color) => {
      state.inkColor = color;
      $("#drawColor").value = color;
      syncInkToolbar();
    },
  });
  syncInkToolbar();
}

function setInkTool(tool) {
  if (!['select', 'pen', 'highlighter', 'eraser'].includes(tool)) return;
  cancelInkPointer();
  state.inkTool = tool;
  syncInkToolbar();
  if (tool !== "select") selectObject(null);
}

function syncInkToolbar() {
  $$('[data-draw-tool]').forEach((button) => button.classList.toggle("active", button.dataset.drawTool === state.inkTool));
  document.body.classList.toggle("ink-tool-active", state.inkTool !== "select");
  document.body.classList.toggle("ink-tool-eraser", state.inkTool === "eraser");
  const widthTool = state.inkTool === "highlighter" ? "highlighter" : state.inkTool === "eraser" ? "eraser" : "pen";
  const width = state.inkWidths[widthTool];
  if ($("#drawWidth")) $("#drawWidth").value = String(width);
  if ($("#drawWidthValue")) $("#drawWidthValue").value = `${width} px`;
  if ($("#drawColor")) $("#drawColor").disabled = state.inkTool === "eraser";
  $("#drawColor")?._syncColorSwatch?.();
}

function slidePointFromPointer(event) {
  const stage = $("#slideStage");
  const rect = stage.getBoundingClientRect();
  if (!rect.width || !rect.height) return null;
  return [
    clamp((event.clientX - rect.left) * state.deck.width / rect.width, 0, state.deck.width),
    clamp((event.clientY - rect.top) * state.deck.height / rect.height, 0, state.deck.height),
    Number.isFinite(event.pressure) && event.pressure > 0 ? event.pressure : 0.5,
  ];
}

function handleInkPointerDown(event) {
  if (state.inkTool === "select" || !state.deck || event.button !== 0) return;
  const point = slidePointFromPointer(event);
  if (!point) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  state.inkPointerId = event.pointerId;
  $("#slideStage").setPointerCapture?.(event.pointerId);
  if (state.inkTool === "eraser") {
    state.inkEraseCheckpointed = false;
    state.inkEraseChanged = false;
    eraseInkAt(point);
    return;
  }
  state.inkPoints = [point];
  state.inkPreview = createInkPreview();
  updateInkPreview();
}

function handleInkPointerMove(event) {
  if (state.inkPointerId !== event.pointerId) return;
  const point = slidePointFromPointer(event);
  if (!point) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  if (state.inkTool === "eraser") {
    eraseInkAt(point);
    return;
  }
  const previous = state.inkPoints.at(-1);
  if (previous && Math.hypot(point[0] - previous[0], point[1] - previous[1]) < 0.75) return;
  state.inkPoints.push(point);
  updateInkPreview();
}

function handleInkPointerUp(event) {
  if (state.inkPointerId !== event.pointerId) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  $("#slideStage").releasePointerCapture?.(event.pointerId);
  state.inkPointerId = null;
  if (state.inkTool === "eraser") {
    if (state.inkEraseChanged) {
      state.future = [];
      markDeckChanged();
      renderAll();
    }
    state.inkEraseCheckpointed = false;
    state.inkEraseChanged = false;
    return;
  }
  const points = globalThis.UniPptInk?.simplify?.(state.inkPoints, 1.25) || state.inkPoints;
  removeInkPreview();
  state.inkPoints = [];
  if (points.length < 2) return;
  addInkStroke(points, state.inkTool === "highlighter");
}

function cancelInkPointer() {
  if (state.inkPointerId != null) $("#slideStage")?.releasePointerCapture?.(state.inkPointerId);
  state.inkPointerId = null;
  state.inkPoints = [];
  state.inkEraseCheckpointed = false;
  state.inkEraseChanged = false;
  removeInkPreview();
}

function createInkPreview() {
  const namespace = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(namespace, "svg");
  const path = document.createElementNS(namespace, "path");
  svg.classList.add("ink-preview-layer");
  svg.setAttribute("viewBox", `0 0 ${state.deck.width} ${state.deck.height}`);
  path.setAttribute("stroke", state.inkColor);
  path.setAttribute("stroke-width", String(state.inkTool === "highlighter" ? state.inkWidths.highlighter : state.inkWidths.pen));
  path.setAttribute("opacity", state.inkTool === "highlighter" ? "0.36" : "1");
  svg.append(path);
  $("#slideStage").append(svg);
  return { svg, path };
}

function updateInkPreview() {
  state.inkPreview?.path.setAttribute("d", globalThis.UniPptInk?.pointsToPath?.(state.inkPoints) || "");
}

function removeInkPreview() {
  state.inkPreview?.svg.remove();
  state.inkPreview = null;
}

function addInkStroke(points, highlighter) {
  const width = highlighter ? state.inkWidths.highlighter : state.inkWidths.pen;
  const box = globalThis.UniPptInk.bounds(points);
  const padding = Math.max(2, width);
  const frame = {
    x: Math.max(0, box.x - padding), y: Math.max(0, box.y - padding),
    width: Math.max(1, box.width + padding * 2), height: Math.max(1, box.height + padding * 2), rotation: 0,
  };
  const local = points.map((point) => [point[0] - frame.x, point[1] - frame.y, point[2]]);
  const object = newSceneObject("shape", highlighter ? "墨迹·荧光笔" : "墨迹·钢笔", frame, "", 1);
  object.customGeometry = {
    width: Math.max(1, Math.round(frame.width)),
    height: Math.max(1, Math.round(frame.height)),
    pathData: globalThis.UniPptInk.pointsToPath(local),
  };
  object.style = {
    ...defaultStyle(), fill: "transparent", stroke: state.inkColor, strokeWidth: width,
    opacity: highlighter ? 0.36 : 1,
  };
  addObject(object);
}

function isInkObject(object) {
  return object?.kind === "shape" && object?.customGeometry?.pathData && String(object.name || "").startsWith("墨迹");
}

function eraseInkAt(point) {
  const radius = state.inkWidths.eraser / 2;
  const objects = currentSlide().objects;
  for (let index = objects.length - 1; index >= 0; index -= 1) {
    const object = objects[index];
    if (!isInkObject(object)) continue;
    const localPoint = [point[0] - object.frame.x, point[1] - object.frame.y];
    const points = globalThis.UniPptInk.pathToPoints(object.customGeometry.pathData);
    if (!globalThis.UniPptInk.hitTest(points, localPoint, radius + (Number(object.style?.strokeWidth) || 1) / 2)) continue;
    if (!state.inkEraseCheckpointed) {
      checkpoint();
      state.inkEraseCheckpointed = true;
    }
    objects.splice(index, 1);
    state.inkEraseChanged = true;
    $("#slideStage").querySelector(`.scene-object[data-id="${cssEscape(object.id)}"]`)?.remove();
  }
}

function convertSelectedInkToShape() {
  const object = selectedObject() || [...currentSlide().objects].reverse().find(isInkObject);
  if (!isInkObject(object)) return toast("请先选择一条墨迹");
  const points = globalThis.UniPptInk.pathToPoints(object.customGeometry.pathData);
  const recognized = globalThis.UniPptInk.recognizeShape(points);
  if (!recognized) return toast("未识别出直线、矩形、椭圆或三角形");
  mutate(() => {
    object.name = `墨迹形状·${recognized.kind}`;
    object.customGeometry = null;
    object.style.opacity = 1;
    if (recognized.kind === "ellipse") object.geometry = "ellipse";
    else if (recognized.kind === "rectangle") object.geometry = "rect";
    else if (recognized.kind === "triangle") object.geometry = "triangle";
    else {
      object.geometry = "line";
      const [start, end] = recognized.points;
      object.flipV = (end[0] - start[0]) * (end[1] - start[1]) < 0;
    }
  });
  toast(`已转换为${({ line: "直线", ellipse: "椭圆", rectangle: "矩形", triangle: "三角形" })[recognized.kind]}`);
}

function clearSlideInk() {
  const count = currentSlide().objects.filter(isInkObject).length;
  if (!count) return toast("当前幻灯片没有墨迹");
  mutate(() => {
    currentSlide().objects = currentSlide().objects.filter((object) => !isInkObject(object));
    state.selectedId = null;
  });
  toast(`已清除 ${count} 条墨迹`);
}

function bindInspectorInputs() {
  const fields = {
    propName: (o, v) => o.name = v,
    propX: (o, v) => o.frame.x = number(v, o.frame.x),
    propY: (o, v) => o.frame.y = number(v, o.frame.y),
    propW: (o, v) => o.frame.width = Math.max(8, number(v, o.frame.width)),
    propH: (o, v) => o.frame.height = Math.max(8, number(v, o.frame.height)),
    propRotation: (o, v) => o.frame.rotation = number(v, o.frame.rotation),
    propFill: (o, v) => { o.style.fill = v; o.style.gradient = null; },
    propColor: (o, v) => applyTextStyle(o, "color", v),
    propFontSize: (o, v) => applyTextStyle(
      o,
      "fontSize",
      Math.max(1, number(v, o.textStyle.fontSize * 72 / 96)) * 96 / 72,
    ),
  };
  for (const [id, setter] of Object.entries(fields)) {
    $("#" + id).addEventListener("change", (event) => updateSelected((object) => setter(object, event.target.value)));
  }
  globalThis.UniPptColorPicker?.attachInput($("#propFill"), {
    label: "形状填充",
    current: () => selectedObject()?.style?.fill || "transparent",
    autoLabel: "无填充色",
    autoColor: "transparent",
    onPick: (color) => updateSelected((object) => {
      object.style.fill = color;
      object.style.gradient = null;
      object.shapeFillAsset = null;
    }),
  });
  globalThis.UniPptColorPicker?.attachInput($("#propColor"), {
    label: "字体颜色",
    current: () => selectedObject()?.textStyle?.color || "#172033",
    autoLabel: "自动（黑色）",
    autoColor: "#000000",
    onPick: (color) => updateSelected((object) => applyTextStyle(object, "color", color)),
  });
}

/**
 * Turns a failed response into a message worth showing.
 *
 * A rejected request does not always carry a JSON body: if the server abandons
 * a request mid-flight the HTTP layer closes it with a bare status and no
 * payload, and `Response.json()` then throws "Unexpected end of JSON input",
 * which tells the user nothing about what actually went wrong.
 */
async function readResponseError(response) {
  const text = await response.text().catch(() => "");
  if (text.trim()) {
    try {
      const payload = JSON.parse(text);
      if (payload?.error) return String(payload.error);
    } catch (_) { /* not JSON; fall back to the raw body */ }
    return text.trim().slice(0, 200);
  }
  return `服务器没有返回内容（HTTP ${response.status}${response.statusText ? ` ${response.statusText}` : ""}）`;
}

async function readJsonResponse(response) {
  if (!response.ok) throw new Error(await readResponseError(response));
  const text = await response.text();
  try {
    return JSON.parse(text);
  } catch (_) {
    throw new Error(`服务器返回了无法解析的响应（HTTP ${response.status}，${text.length} 字节）`);
  }
}

async function openFile(file) {
  if (!file) return;
  $("#fileInput").value = "";
  if (/\.udoc$/i.test(file.name)) {
    return openPortableFile(file, "/api/import-udoc", "UDoc");
  }
  if (/\.html?$/i.test(file.name)) {
    return openPortableFile(file, "/api/import-html", "无损 HTML");
  }
  $("#loading").hidden = false;
  setStatus(`正在解析 ${file.name}…`);
  try {
    const response = await fetch("/api/import-pptx", {
      method: "POST",
      headers: {
        "Content-Type": "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        // Percent encoding keeps non-ASCII filenames legal in an HTTP header.
        "X-UniPPT-Filename": encodeURIComponent(file.name),
      },
      body: file,
    });
    const cacheStatus = response.headers.get("X-UniPPT-Cache");
    const cacheId = response.headers.get("X-UniPPT-Cache-Id");
    const payload = await readJsonResponse(response);
    const sourceTitle = payload.title;
    payload.title = file.name.replace(/\.pptx$/i, "");
    replaceDeck(
      normalizeDeck(payload),
      `已导入 ${file.name} · ${payload.slides.length} 张幻灯片`,
      {
        sourceFile: file,
        sourceKind: "pptx",
        cacheId: cacheId || payload.sourceImportId,
        // Old servers do not know the local filename. Do not use a cache-only
        // export if the client had to adjust the imported title.
        cacheOnlyEligible: Boolean(cacheId && cacheStatus !== "bypass" && sourceTitle === payload.title),
      },
    );
  } catch (error) {
    setStatus("导入失败");
    toast(`PPTX 导入失败：${error.message}`);
  } finally {
    $("#loading").hidden = true;
  }
}

async function openPortableFile(file, endpoint, label) {
  $("#loading").hidden = false;
  setStatus(`正在打开 ${label}：${file.name}…`);
  try {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: { "X-UniPPT-Filename": encodeURIComponent(file.name) },
      body: file,
    });
    const cacheStatus = response.headers.get("X-UniPPT-Cache");
    const cacheId = response.headers.get("X-UniPPT-Cache-Id");
    const payload = await readJsonResponse(response);
    replaceDeck(
      normalizeDeck(payload),
      `已打开 ${label} · ${payload.slides.length} 张幻灯片`,
      {
        sourceFile: file,
        sourceKind: label.toLowerCase(),
        cacheId: cacheId || payload.sourceImportId,
        cacheOnlyEligible: Boolean(cacheId && cacheStatus !== "bypass"),
      },
    );
  } catch (error) {
    setStatus(`${label} 打开失败`);
    toast(`${label} 打开失败：${error.message}`);
  } finally {
    $("#loading").hidden = true;
  }
}

function normalizeDeck(deck) {
  deck.extensions ||= {};
  deck.sourceImportId ??= null;
  deck.fonts ||= [];
  for (const font of deck.fonts) {
    font.family ||= "Embedded Font";
    font.weight = Math.max(1, Math.min(1000, Number(font.weight) || 400));
    font.style = font.style === "italic" ? "italic" : "normal";
    font.dataUri ||= "";
    font.mimeType ||= "font/ttf";
    font.format ||= font.mimeType === "font/otf" ? "opentype" : "truetype";
    font.sha256 = /^[a-f0-9]{64}$/i.test(String(font.sha256 || "")) ? String(font.sha256).toLowerCase() : "";
    font.md5 = /^[a-f0-9]{32}$/i.test(String(font.md5 || "")) ? String(font.md5).toLowerCase() : "";
    font.bytes = Math.max(0, Number(font.bytes) || 0);
    font.postscriptName = String(font.postscriptName || "");
    font.faceIndex = Number.isInteger(Number(font.faceIndex)) ? Number(font.faceIndex) : null;
  }
  for (const slide of deck.slides || []) {
    slide.sourcePartName ??= null;
    slide.backgroundAsset ??= null;
    slide.masterObjects ||= [];
    slide.layoutObjects ||= [];
    slide.inheritedAnimations ||= [];
    slide.animations ||= [];
    slide.transition ??= null;
    if (slide.transition) {
      slide.transition.kind ||= "fade";
      slide.transition.durationMs = Math.max(1, Number(slide.transition.durationMs) || 700);
      slide.transition.advanceOnClick = slide.transition.advanceOnClick !== false;
      slide.transition.advanceAfterMs = slide.transition.advanceAfterMs == null ? null : Math.max(0, Number(slide.transition.advanceAfterMs) || 0);
      slide.transition.direction ??= null;
    }
    slide.sourceTimingXml ??= null;
    slide.sourceTransitionXml ??= null;
    normalizeAnimations(slide.inheritedAnimations);
    normalizeAnimations(slide.animations);
    normalizeObjects(slide.masterObjects);
    normalizeObjects(slide.layoutObjects);
    normalizeObjects(slide.objects || []);
  }
  return globalThis.UniPptPresentationHost?.enrichSemantics?.(deck) || deck;
}

function normalizeAnimations(animations) {
  animations.forEach((animation, order) => {
      animation.id ||= uid("anim");
      animation.sourceTimingId ??= null;
      animation.targetObjectId ??= null;
      animation.targetShapeId ??= null;
      animation.effect ||= "custom";
      animation.class ||= animation.effect === "motionPath" ? "motionPath" : animation.effect === "media" ? "media" : "entrance";
      animation.trigger ||= "onClick";
      animation.durationMs = Math.max(1, Number(animation.durationMs) || 500);
      animation.delayMs = Math.max(0, Number(animation.delayMs) || 0);
      animation.acceleration = animation.acceleration == null ? null : Math.max(0, Math.min(100000, Number(animation.acceleration) || 0));
      animation.deceleration = animation.deceleration == null ? null : Math.max(0, Math.min(100000, Number(animation.deceleration) || 0));
      animation.speed = animation.speed == null ? null : Number(animation.speed) || 0;
      animation.timeFilter = typeof animation.timeFilter === "string" && animation.timeFilter.trim() ? animation.timeFilter : null;
      animation.repeatCount ??= null;
      animation.repeatDurationMs = animation.repeatDurationMs == null ? null : Math.max(1, Number(animation.repeatDurationMs) || 1);
      animation.autoReverse = Boolean(animation.autoReverse);
      animation.order = Number.isFinite(animation.order) ? animation.order : order;
      animation.presetId ??= null;
      animation.presetSubtype ??= null;
      animation.direction ??= null;
      animation.motionPath ??= null;
      animation.mediaAction ??= animation.effect === "media" ? "play" : null;
  });
}

function normalizeObjects(objects) {
  for (const object of objects) {
    object.style ||= defaultStyle();
    object.style.gradient ??= null;
    object.style.strokeDash ??= null;
    object.style.shadow ??= null;
    object.textStyle ||= defaultTextStyle();
    object.textStyle.nativeFontFamily ??= null;
    object.textStyle.nativeFonts ||= {};
    object.imageCrop ||= { left: 0, top: 0, right: 0, bottom: 0 };
    object.imageEffects ||= {};
    object.imageEffects.duotone ??= null;
    object.imageEffects.softEdgeRadius = Math.max(0, Number(object.imageEffects.softEdgeRadius) || 0);
    for (const edge of ["left", "top", "right", "bottom"]) {
      object.imageCrop[edge] = clamp(Number(object.imageCrop[edge]) || 0, 0, 1);
    }
    object.children ||= [];
    object.formula ??= null;
    object.media ??= null;
    object.chart ??= null;
    globalThis.UniPptMedia?.normalizeObject(object);
    object.hyperlinks ||= { click: null, hover: null };
    object.hyperlinks.click ??= null;
    object.hyperlinks.hover ??= null;
    object.textParagraphs ||= [];
    for (const paragraph of object.textParagraphs) {
      for (const run of paragraph.runs || []) {
        run.nativeFontFamily ??= null;
        run.nativeFonts ||= {};
        run.hyperlinks ||= { click: null, hover: null };
        run.hyperlinks.click ??= null;
        run.hyperlinks.hover ??= null;
      }
    }
    object.textFrame ||= { marginLeft: 8, marginRight: 8, marginTop: 5, marginBottom: 5, verticalAlign: "center", verticalType: "horz", wordWrap: true, autoSize: "none" };
    object.textFrame.verticalType ||= "horz";
    object.textFrame.autoSize ||= "none";
    if (object.textFrame.autoSize === "shrinkText") object.textFrame.autoSize = "textToFitShape";
    object.flipH = Boolean(object.flipH);
    object.flipV = Boolean(object.flipV);
    normalizeObjects(object.children);
  }
}

async function newDeck() {
  const response = await fetch("/api/demo");
  if (!response.ok) return toast("无法创建演示文稿");
  replaceDeck(normalizeDeck(await response.json()), "已新建演示文稿", { sourceKind: "new" });
}

function replaceDeck(deck, message, cacheOptions = {}) {
  const normalizedDeck = normalizeDeck(deck);
  const clone = globalThis.UniPptDocumentCache?.cloneForHistory || structuredClone;
  const previous = {
    deck: state.deck,
    documentCache: state.documentCache ? clone(state.documentCache) : state.documentCache,
    activeSlide: state.activeSlide,
    selectedId: state.selectedId,
    selectedAnimationId: state.selectedAnimationId,
    history: state.history.slice(),
    future: state.future.slice(),
    autoFit: state.autoFit,
    autoTextFitCache: new Map(autoTextFitCache),
  };
  const undoable = Boolean(cacheOptions.undoable && state.deck);
  try {
    if (undoable) checkpoint();
    state.deck = normalizedDeck;
    autoTextFitCache.clear();
    resetDocumentCache(cacheOptions);
    state.activeSlide = 0;
    state.selectedId = null;
    state.selectedAnimationId = null;
    if (!undoable) state.history = [];
    state.future = [];
    state.autoFit = true;
    renderAll();
  } catch (error) {
    state.deck = previous.deck;
    state.documentCache = previous.documentCache;
    state.activeSlide = previous.activeSlide;
    state.selectedId = previous.selectedId;
    state.selectedAnimationId = previous.selectedAnimationId;
    state.history = previous.history;
    state.future = previous.future;
    state.autoFit = previous.autoFit;
    autoTextFitCache.clear();
    for (const [key, value] of previous.autoTextFitCache) autoTextFitCache.set(key, value);
    try { renderAll(); } catch (_) {}
    throw error;
  }
  void installEmbeddedFonts(normalizedDeck);
  requestAnimationFrame(fitSlide);
  setStatus(message);
  toast(message);
  state.documentHost?._emit?.("load", { revision: state.documentCache?.revision || 0, source: cacheOptions.sourceKind || "load" });
}

function resetDocumentCache(options = {}) {
  void mcpBridge?.disconnect('文稿已切换，旧 MCP 会话已关闭',true);
  imageReconstructionSources.clear();
  const runtime = globalThis.UniPptDocumentCache;
  if (!runtime || !state.deck) return;
  state.documentCache ||= runtime.create();
  runtime.reset(state.documentCache, state.deck, options);
  updateUdocCacheSummary();
}

function markDeckChanged() {
  const runtime = globalThis.UniPptDocumentCache;
  if (!runtime || !state.documentCache) return;
  runtime.markChanged(state.documentCache);
  updateUdocCacheSummary();
  scheduleUdocStructureRefresh();
}

function exportRequestBody() {
  const runtime = globalThis.UniPptDocumentCache;
  if (!runtime || !state.documentCache) return JSON.stringify({ deck: globalThis.UniPptPresentationHost?.prepareNativeExport?.(state.deck) || state.deck });
  const body = runtime.exportBody(state.documentCache, state.deck);
  updateUdocCacheSummary();
  return body;
}

function createExportRequest() {
  const revision = state.documentCache?.revision ?? null;
  return { body: exportRequestBody(), revision };
}

function markExportRevisionSynced(response, sentRevision) {
  const runtime = globalThis.UniPptDocumentCache;
  const cache = state.documentCache;
  if (!runtime || !cache || !response?.ok) return;
  const responseCacheId = response.headers.get("X-UniPPT-Cache-Id");
  const responseCacheState = response.headers.get("X-UniPPT-Cache");
  const responseRevision = Number(response.headers.get("X-UniPPT-Cache-Revision"));
  if (responseCacheId
    && responseCacheId === cache.cacheId
    && responseCacheState !== "bypass"
    && Number.isInteger(sentRevision)
    && responseRevision === sentRevision
    && cache.revision === sentRevision) {
    runtime.markSynced(cache, sentRevision);
    updateUdocCacheSummary();
  }
}

function installEmbeddedFonts(deck) {
  const generation = ++fontInstallGeneration;
  bindFontLifecycleEvents();
  for (const face of embeddedFontFaces) document.fonts.delete(face);
  embeddedFontFaces = [];
  globalThis.UniPptFontRuntime?.cancel?.();
  document.getElementById("unippt-embedded-fonts")?.remove();
  refreshFontCatalog(deck);
  const definitions = (deck.fonts || []).map((font) => {
    const source = String(font.dataUri || "");
    if (!font.family || !(/^(?:data:(?:font\/|application\/)|\/api\/cache\/[^/]+\/asset\/)/i.test(source))) return null;
    const weight = Math.max(1, Math.min(1000, Number(font.weight) || 400));
    const fontStyle = font.style === "italic" ? "italic" : "normal";
    const format = String(font.format || "truetype");
    const verified = /^[a-f0-9]{64}$/i.test(String(font.sha256 || ""))
      && /^[a-f0-9]{32}$/i.test(String(font.md5 || ""))
      && !/^(?:embedded-opentype|collection)$/i.test(format)
      && !/^font\/collection$/i.test(String(font.mimeType || ""));
    return { family: font.family, source, format, weight, fontStyle, verified };
  }).filter(Boolean);
  if (!definitions.length) return Promise.resolve(false);

  // Fonts are a post-paint cold path. Imported CJK faces can exceed 50 MiB;
  // starting them before the first Deck render competes with JSON parsing and
  // makes the loading overlay appear stuck even though fonts are best effort.
  const work = new Promise((resolve) => scheduleFontPreparation(async () => {
    if (generation !== fontInstallGeneration || deck !== state.deck) return resolve([]);
    await ensureFeature("render").catch(() => undefined);
    if (generation !== fontInstallGeneration || deck !== state.deck) return resolve([]);
    verifiedFontBatchActive++;
    globalThis.UniPptStartup?.metrics && (globalThis.UniPptStartup.metrics.fontRequestedAt = performance.now());
    try {
      const loads = [];
      const actualFamilies = globalThis.UniPptFonts?.collectDeckFontFamilies(deck) || [];
      const usedFamilies = new Set(actualFamilies.map((family) => String(family).toLocaleLowerCase()));
      const legacy = definitions.filter((definition) => !definition.verified
        && usedFamilies.has(String(definition.family).toLocaleLowerCase()));

      if (definitions.some((definition) => definition.verified)) {
        const runtime = globalThis.UniPptFontRuntime;
        if (runtime?.prepareDeckFonts) loads.push(runtime.prepareDeckFonts(deck));
      }

      // Legacy files without fingerprints retain the existing trusted same-origin
      // FontFace path. Newly imported fonts always use the verified runtime above.
      if (globalThis.FontFace && document.fonts) {
        for (const definition of legacy) {
          try {
            const face = new FontFace(
              definition.family,
              `url(${JSON.stringify(definition.source)}) format(${cssString(definition.format)})`,
              { weight: String(definition.weight), style: definition.fontStyle, display: "swap" },
            );
            document.fonts.add(face);
            embeddedFontFaces.push(face);
            loads.push(face.load().catch(() => {
              document.fonts.delete(face);
              return null;
            }));
          } catch (_) { /* invalid or browser-rejected font: use the system fallback */ }
        }
      } else if (legacy.length) {
        const host = document.createElement("div");
        host.id = "unippt-embedded-fonts";
        host.hidden = true;
        for (const definition of legacy) {
          const style = document.createElement("style");
          style.textContent = `@font-face{font-family:${cssString(definition.family)};src:url(${JSON.stringify(definition.source)}) format(${cssString(definition.format)});font-weight:${definition.weight};font-style:${definition.fontStyle};font-display:swap;}`;
          host.append(style);
        }
        document.head.append(host);
      }
      if (document.fonts?.load) {
        for (const family of actualFamilies) {
          try { loads.push(document.fonts.load(`16px ${cssString(family)}`).catch(() => [])); }
          catch (_) { /* malformed local font name: keep its PowerPoint fallback */ }
        }
      }
      resolve(await Promise.allSettled(loads));
    } catch (error) {
      console.warn("UniPPT deferred font preparation failed", error);
      resolve([]);
    } finally {
      verifiedFontBatchActive--;
      globalThis.UniPptStartup?.metrics && (globalThis.UniPptStartup.metrics.fontReadyAt = performance.now());
    }
  }));
  return work
    .then(() => document.fonts?.ready || undefined)
    .then(() => {
      if (generation !== fontInstallGeneration || deck !== state.deck) return false;
      scheduleFontMetricRefresh(generation);
      return true;
    });
}

function scheduleFontPreparation(run) {
  if (typeof requestIdleCallback === "function") requestIdleCallback(run, { timeout: 800 });
  else setTimeout(run, 120);
}

function bindFontLifecycleEvents() {
  if (fontEventsBound || !document.fonts?.addEventListener) return;
  fontEventsBound = true;
  document.fonts.addEventListener("loadingdone", () => {
    if (verifiedFontBatchActive) return;
    scheduleFontMetricRefresh(fontInstallGeneration);
  });
}

function scheduleFontMetricRefresh(generation) {
  clearTimeout(fontMetricRefreshTimer);
  fontMetricRefreshTimer = setTimeout(() => {
    if (generation !== fontInstallGeneration || !state.deck) return;
    autoTextFitCache.clear();
    renderSlideList();
    renderCanvas();
    if (!$("#presenter")?.hidden) {
      renderPresentation(false, {
        restoreCursor: state.presenterAnimationCursor,
        runInitialAutomatic: false,
      });
      fitPresentationStage();
    }
    if (state.autoFit) scheduleFitSlide();
    else updateCanvasScale();
  }, 80);
}

function refreshFontCatalog(deck) {
  const select = $("#ribbonFontFamily");
  if (!select) return;
  select.querySelectorAll("option[data-document-font]").forEach((option) => option.remove());
  const embedded = new Map((deck.fonts || [])
    .filter((font) => font?.family)
    .map((font) => [String(font.family).toLocaleLowerCase(), String(font.family)]));
  const actual = globalThis.UniPptFonts?.collectDeckFontFamilies(deck) || [];
  const families = [...actual];
  for (const family of embedded.values()) {
    if (!families.some((value) => value.toLocaleLowerCase() === family.toLocaleLowerCase())) families.push(family);
  }
  for (const family of families) {
    if ([...select.options].some((option) => option.value.toLocaleLowerCase() === family.toLocaleLowerCase())) continue;
    const option = new Option(family, family);
    option.dataset.documentFont = "1";
    option.dataset.fontSource = embedded.has(family.toLocaleLowerCase()) ? "embedded" : "document";
    option.title = option.dataset.fontSource === "embedded" ? "文档内嵌字体" : "文档使用的字体";
    select.add(option);
  }
}

function cssString(value) {
  return `"${String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/[\r\n]/g, " ")}"`;
}

function toggleExportMenu(anchor = $("#exportMenuButton"), forceOpen = false) {
  const popup = $("#exportMenuPopup");
  if (!popup || !anchor) return;
  const shouldOpen = forceOpen || popup.hidden;
  if (!shouldOpen) {
    closeExportMenu();
    return;
  }
  popup.hidden = false;
  const rect = anchor.getBoundingClientRect();
  const margin = 8;
  const width = popup.offsetWidth || 318;
  const height = popup.offsetHeight || 240;
  const left = Math.max(margin, Math.min(innerWidth - width - margin, rect.left));
  const below = rect.bottom + 6;
  const top = below + height <= innerHeight - margin
    ? below
    : Math.max(margin, rect.top - height - 6);
  popup.style.left = `${left}px`;
  popup.style.top = `${top}px`;
  $("#exportMenuButton")?.setAttribute("aria-expanded", "true");
  popup.querySelector("button")?.focus({ preventScroll: true });
}

function closeExportMenu() {
  const popup = $("#exportMenuPopup");
  if (!popup || popup.hidden) return;
  popup.hidden = true;
  $("#exportMenuButton")?.setAttribute("aria-expanded", "false");
}

async function savePptx() {
  if (!state.deck) return;
  $("#loading").hidden = false;
  setStatus("正在生成原生 PPTX…");
  try {
    const exportRequest = createExportRequest();
    const response = await fetch("/api/export-pptx", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: exportRequest.body,
    });
    if (!response.ok) throw new Error(await readResponseError(response));
    markExportRevisionSynced(response, exportRequest.revision);
    downloadBlob(`${safeFilename(state.deck.title)}.pptx`, await response.blob());
    setStatus("PPTX 已生成 · 未编辑部件已原样保留");
    toast("原生 PPTX 已保存");
  } catch (error) {
    setStatus("PPTX 保存失败");
    toast(`PPTX 保存失败：${error.message}`);
  } finally {
    $("#loading").hidden = true;
  }
}

async function saveUdoc() {
  if (!state.deck) return;
  await exportPortable("/api/export-udoc", "udoc", "application/vnd.unidoc", "正在打包 Brotli + ZIP UDoc…", "UDoc 已保存 · unidoc_type=pptx");
}

async function exportLosslessHtml() {
  if (!state.deck) return;
  await exportPortable("/api/export-html", "html", "text/html", "正在生成无损 HTML 放映…", "无损 HTML 已导出 · 可重新打开编辑");
}

async function exportPdf() {
  if (!state.deck) return;
  $("#loading").hidden = false;
  setLoadingVisual("正在准备浏览器原生打印…", null, "正在装配全部幻灯片与字体");
  try {
    const host = setupNativePrintPages();
    await settleNativePrintPages(host);
    $("#loading").hidden = true;
    setStatus("PDF 打印版式已就绪 · 请在浏览器打印窗口中选择“另存为 PDF”");
    window.print();
    toast("已打开浏览器原生打印 · 可选择“另存为 PDF”");
    // Chromium keeps window.print() blocked until its preview closes.  The
    // timer is a fallback for embedded browsers that omit `afterprint`.
    setTimeout(teardownNativePrintPages, 1000);
  } catch (error) {
    teardownNativePrintPages();
    setStatus("PDF 打印准备失败");
    toast(`PDF 打印失败：${error.message}`);
  } finally {
    $("#loading").hidden = true;
    setLoadingVisual("正在处理…");
  }
}

const NATIVE_PRINT_HOST_ID = "unipptNativePrintPages";
const NATIVE_PRINT_STYLE_ID = "unipptNativePrintStyle";

function setupNativePrintPages() {
  if (!state.deck) return null;
  const existing = document.getElementById(NATIVE_PRINT_HOST_ID);
  if (existing) return existing;

  const width = Math.max(1, Number(state.deck.width) || 1280);
  const height = Math.max(1, Number(state.deck.height) || 720);
  const style = document.createElement("style");
  style.id = NATIVE_PRINT_STYLE_ID;
  style.textContent = `
    @media screen { #${NATIVE_PRINT_HOST_ID} { display: none !important; } }
    @media print {
      @page { size: ${width}px ${height}px; margin: 0; }
      html, body {
        width: auto !important;
        height: auto !important;
        margin: 0 !important;
        padding: 0 !important;
        overflow: visible !important;
        background: #fff !important;
        print-color-adjust: exact !important;
        -webkit-print-color-adjust: exact !important;
      }
      body > :not(#${NATIVE_PRINT_HOST_ID}) { display: none !important; }
      #${NATIVE_PRINT_HOST_ID} {
        display: block !important;
        position: static !important;
        width: ${width}px !important;
        margin: 0 !important;
        padding: 0 !important;
        background: #fff !important;
      }
      #${NATIVE_PRINT_HOST_ID} > .unippt-native-print-slide {
        position: relative !important;
        display: block !important;
        width: ${width}px !important;
        height: ${height}px !important;
        margin: 0 !important;
        padding: 0 !important;
        overflow: hidden !important;
        box-sizing: border-box !important;
        break-after: page;
        page-break-after: always;
      }
      #${NATIVE_PRINT_HOST_ID} > .unippt-native-print-slide:last-child {
        break-after: auto;
        page-break-after: auto;
      }
    }
  `;
  document.head.append(style);

  const host = document.createElement("main");
  host.id = NATIVE_PRINT_HOST_ID;
  host.setAttribute("aria-hidden", "true");
  state.deck.slides.forEach((slide, index) => {
    const page = document.createElement("section");
    page.className = "unippt-native-print-slide";
    page.dataset.slideIndex = String(index);
    page.style.width = `${width}px`;
    page.style.height = `${height}px`;
    applySlideBackground(page, slide);
    renderReadOnlySlide(slide, page, {
      scope: "presentation-print",
      slideKey: slide.sourcePartName || slide.id || String(index),
    });
    host.append(page);
  });
  document.body.append(host);
  return host;
}

async function settleNativePrintPages(host) {
  if (!host) throw new Error("没有可打印的幻灯片");
  try { await document.fonts?.ready; } catch (_) {}
  await Promise.all([...host.querySelectorAll("img")].map(async (image) => {
    if (!image.complete) {
      await new Promise((resolve) => {
        image.addEventListener("load", resolve, { once: true });
        image.addEventListener("error", resolve, { once: true });
      });
    }
    try { await image.decode?.(); } catch (_) {}
  }));
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
}

function teardownNativePrintPages() {
  const host = document.getElementById(NATIVE_PRINT_HOST_ID);
  if (host) globalThis.UniPptMedia?.stopAll(host, true, true);
  host?.remove();
  document.getElementById(NATIVE_PRINT_STYLE_ID)?.remove();
}

async function exportSlideImages() {
  if (!state.deck) return;
  await exportPortable(
    "/api/export-images",
    "zip",
    "application/zip",
    "正在渲染逐页 PNG 并打包…",
    "图片压缩包已导出 · 含逐页 PNG 与清单",
  );
}

function setLoadingVisual(message, progress = null, detail = "") {
  const loading = $("#loading");
  const text = $("#loadingText") || loading?.querySelector("p");
  const track = $("#loadingProgress");
  const bar = $("#loadingProgressBar");
  const detailNode = $("#loadingDetail");
  if (text && message) text.textContent = message;
  if (track && bar) {
    const numeric = Number(progress);
    const visible = Number.isFinite(numeric);
    track.hidden = !visible;
    if (visible) {
      const percent = clamp(numeric, 0, 100);
      bar.style.width = `${percent}%`;
      track.setAttribute("aria-valuemin", "0");
      track.setAttribute("aria-valuemax", "100");
      track.setAttribute("aria-valuenow", String(Math.round(percent)));
    }
  }
  if (detailNode) {
    detailNode.hidden = !detail;
    detailNode.textContent = detail;
  }
}

function exportProgressDetail(progress) {
  const parts = [];
  if (progress.totalSlides > 0) parts.push(`${progress.completedSlides}/${progress.totalSlides} 页`);
  if (progress.cachedSlides > 0) parts.push(`缓存 ${progress.cachedSlides} 页`);
  if (progress.frames > 0) parts.push(`${progress.frames} 帧`);
  const elapsed = Math.max(0, Number(progress.elapsedMs) || 0) / 1000;
  if (elapsed >= 1) parts.push(`已用 ${Math.round(elapsed)} 秒`);
  if (progress.percent >= 12 && progress.percent < 97 && elapsed >= 2) {
    const remaining = elapsed * (100 - progress.percent) / progress.percent;
    if (Number.isFinite(remaining) && remaining < 60 * 60) {
      parts.push(`预计剩余约 ${Math.max(1, Math.round(remaining))} 秒`);
    }
  }
  return parts.join(" · ");
}

function exportJobId() {
  if (globalThis.crypto?.randomUUID) return globalThis.crypto.randomUUID();
  return `export-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}

function pollVideoExportProgress(jobId) {
  let stopped = false;
  const completed = (async () => {
    while (!stopped) {
      try {
        const response = await fetch(`/api/export-progress/${encodeURIComponent(jobId)}`, {
          cache: "no-store",
        });
        if (response.ok) {
          const progress = await response.json();
          setLoadingVisual(
            progress.error ? `导出失败：${progress.error}` : progress.message,
            progress.percent,
            exportProgressDetail(progress),
          );
          if (progress.done) break;
        }
      } catch (_) {
        // A temporary progress polling failure must not abort the actual render.
      }
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  })();
  return async () => {
    stopped = true;
    await completed;
  };
}

async function exportVideoPortable(endpoint, pending, success, options) {
  const jobId = exportJobId();
  $("#loading").hidden = false;
  setLoadingVisual(pending, 1, "正在创建导出任务…");
  setStatus(pending);
  const stopPolling = pollVideoExportProgress(jobId);
  try {
    const exportRequest = createExportRequest();
    const response = await fetch(endpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-UniPPT-Export-Job": jobId,
        "X-UniPPT-Video-Width": String(options.width),
        "X-UniPPT-Video-Height": String(options.height),
        "X-UniPPT-Video-Fps": String(options.fps),
        "X-UniPPT-Video-Profile": options.profile,
        "X-UniPPT-Video-Muted": options.muted ? "1" : "0",
      },
      body: exportRequest.body,
    });
    if (!response.ok) throw new Error(await readResponseError(response));
    markExportRevisionSynced(response, exportRequest.revision);
    setLoadingVisual("视频已生成，正在接收文件…", 100, "即将开始下载");
    const blob = await response.blob();
    downloadBlob(`${safeFilename(state.deck.title)}.mp4`,
      blob.type ? blob : new Blob([blob], { type: "video/mp4" }));
    setStatus(success);
    toast(success);
  } catch (error) {
    setStatus("MP4 导出失败");
    toast(`导出失败：${error.message}`);
  } finally {
    await stopPolling();
    $("#loading").hidden = true;
    setLoadingVisual("正在处理…", null, "");
  }
}

function videoExportOptions() {
  const [width, height] = String($("#videoExportResolution").value || "1920x1080")
    .split("x")
    .map((value) => Number(value));
  return {
    width: Number.isFinite(width) ? width : 1920,
    height: Number.isFinite(height) ? height : 1080,
    fps: Number($("#videoExportFps").value) || 30,
    profile: $("#videoExportProfile").value || "quality",
    muted: Boolean($("#videoExportMuted").checked),
    engine: $("#videoExportEngine")?.value === "browser" ? "browser" : "server",
  };
}

/**
 * Records the live presenter with the browser's own screen capture.
 *
 * Needs no Node, Chromium, or FFmpeg on the server, at the cost of capturing in
 * real time at whatever the viewer's display can show. The deterministic
 * server pipeline remains the choice when the output must be frame-exact.
 */
async function exportVideoByRecording(options) {
  await Promise.all([ensureFeature("presentation"), ensureFeature("export")]);
  const recorder = globalThis.UniPptScreenRecorder;
  if (!recorder?.supported()) {
    toast("当前浏览器不支持屏幕录制，请改用服务端渲染");
    return;
  }
  const presenter = $("#presenter");
  let closed = null;
  try {
    setStatus("请选择要录制的画面…");
    const result = await recorder.record({
      width: options.width,
      height: options.height,
      fps: options.fps,
      audio: !options.muted,
      onShareEnded: () => {
        toast("共享已停止，正在保存已录制的部分");
        closePresentation();
      },
      onStart: ({ settings }) => {
        const width = settings.width || options.width;
        const height = settings.height || options.height;
        setStatus(`正在录制放映…${width}×${height}`);
      },
      async prepare() {
        // Open the show idle so the capture starts on a fully painted first
        // slide instead of a half-built one, then hand control to autoplay.
        state.slideShowSettings.autoPlay = false;
        startPresentation("beginning", {
          autoPlay: false,
          ignoreRange: true,
          runInitialAutomatic: false,
        });
        presenter.classList.add("recording");
        hidePresentationControls();
        await settleRecordingFrame();
      },
      async play() {
        const finished = new Promise((resolve) => {
          closed = resolve;
          document.addEventListener("unippt:presentation-closed", resolve, { once: true });
        });
        state.slideShowSettings.autoPlay = true;
        updatePresentationAutoPlayControl();
        renderPresentation(false);
        await finished;
        // Let the final slide's last frame reach the encoder before stopping.
        await new Promise((resolve) => setTimeout(resolve, 400));
      },
    });

    const seconds = Math.max(1, Math.round(result.durationMs / 1000));
    downloadBlob(`${safeFilename(state.deck.title)}.${result.format.extension}`, result.blob);
    const geometry = result.width && result.height ? `${result.width}×${result.height}` : "共享画面原始像素";
    const rate = result.frameRate ? `最高 ${Math.round(result.frameRate)} FPS` : "可变帧率";
    const summary = `${result.format.label} · ${geometry} · ${rate} · ${seconds} 秒 · ${(result.blob.size / 1048576).toFixed(1)} MB`;
    setStatus(`录屏视频已导出 · ${summary}`);
    toast(result.endedByUser ? `已保存中断前的录制 · ${summary}` : `录屏视频已导出 · ${summary}`);
  } catch (error) {
    const aborted = error?.name === "NotAllowedError" || error?.name === "AbortError";
    setStatus(aborted ? "已取消录屏导出" : "录屏导出失败");
    if (!aborted) toast(`录屏导出失败：${error.message}`);
  } finally {
    if (closed) document.removeEventListener("unippt:presentation-closed", closed);
    presenter.classList.remove("recording");
    if (!presenter.hidden) closePresentation();
  }
}

/** Waits for fonts, images, and two frames so the first captured frame is complete. */
async function settleRecordingFrame() {
  try { await document.fonts?.ready; } catch (_) { /* fonts are best effort */ }
  const stage = $("#presentStage");
  await Promise.all([...stage.querySelectorAll("img")].map(async (image) => {
    try { await image.decode?.(); } catch (_) { /* a broken image must not stall the take */ }
  }));
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  await new Promise((resolve) => setTimeout(resolve, 350));
}

function videoProfileLabel(profile) {
  return { fast: "极速预览", balanced: "均衡", quality: "高保真" }[profile] || "高保真";
}

function updateVideoExportSummary() {
  const options = videoExportOptions();
  const resolution = options.height === 2160 ? "4K" : options.height === 1440 ? "2K" : `${options.height}p`;
  const browser = options.engine === "browser";
  const recorder = globalThis.UniPptScreenRecorder;
  const format = browser ? recorder?.preferredFormat() : null;
  $("#videoExportSummary").textContent = browser
    ? `${format?.label || "浏览器录屏"} · 跟随共享画面原始像素 · 可变帧率 · ${options.muted ? "静音" : "保留音频"}`
    : `${resolution} · ${options.fps} FPS · ${videoProfileLabel(options.profile)} · ${options.muted ? "静音" : "保留音频"}`;
  $("#videoExportEngineHint").textContent = browser
    ? (recorder?.supported()
      ? "调用浏览器 getDisplayMedia + MediaRecorder 实时录制；分辨率和实际帧率由所选共享画面决定"
      : "当前浏览器不支持 MediaRecorder 屏幕录制，请改用服务端渲染")
    : "服务端用无头 Chromium 逐帧渲染并交给 FFmpeg 编码，画面与时间轴逐帧精确";
  $("#videoExportProfileField").hidden = browser;
  $("#videoExportResolutionField").hidden = browser;
  $("#videoExportFpsField").hidden = browser;
  $("#videoExportResolution").disabled = browser;
  $("#videoExportFps").disabled = browser;
  $("#videoExportResolutionHint").textContent = browser
    ? "浏览器不能强制共享画面输出 4K；真 4K 请改用服务端逐帧渲染"
    : "清晰度越高，逐帧捕获和编码耗时越长";
  $("#videoExportFpsHint").textContent = browser
    ? "屏幕录制为可变帧率，忙碌时可能低于显示器刷新率"
    : "分页转场和对象动画均按此帧率采样";
  $("#videoExportSummaryNote").textContent = browser
    ? "请选择当前 UniPPT 标签页；录制期间不要切换窗口，真 4K / 固定帧率请使用服务端导出。"
    : "将自动播放每页完整动画，最后一页结束后完成视频。";
  $("#videoProfileHint").textContent = options.profile === "quality"
    ? "完整分辨率高质量 JPEG 帧直传 FFmpeg，避免逐帧 PNG 压缩瓶颈"
    : options.profile === "balanced"
      ? "以 75% 分辨率采集后高质量缩放，兼顾速度与清晰度"
      : "以 50% 分辨率采集并并行编码，适合快速预览";
}

async function openVideoExportDialog(profile = "quality") {
  if (!state.deck) return;
  if (profile === "browser") await ensureFeature("export");
  const engine = $("#videoExportEngine");
  if (engine) {
    if (profile === "browser") engine.value = "browser";
    else {
      engine.value = "server";
      $("#videoExportProfile").value = profile;
    }
    // Offer recording only where the browser can actually encode.
    engine.querySelector('option[value="browser"]').disabled = !globalThis.UniPptScreenRecorder?.supported();
  }
  updateVideoExportSummary();
  const dialog = $("#videoExportDialog");
  if (!dialog.open) dialog.showModal();
}

async function confirmVideoExport() {
  if (!state.deck) return;
  const options = videoExportOptions();
  const dialog = $("#videoExportDialog");
  if (dialog.open) dialog.close();
  if (options.engine === "browser") {
    await ensureFeature("export");
    await exportVideoByRecording(options);
    return;
  }
  const endpoint = options.profile === "fast" ? "/api/export-video-fast" : "/api/export-video";
  const resolution = options.height === 2160 ? "4K" : options.height === 1440 ? "2K" : `${options.height}p`;
  await exportVideoPortable(
    endpoint,
    `正在以 ${resolution} / ${options.fps} FPS 渲染完整动画和分页转场…`,
    `MP4 已导出 · ${resolution} / ${options.fps} FPS · 动画与转场已连续拼接`,
    options,
  );
}

async function exportVideo() {
  await openVideoExportDialog("quality");
}

async function exportVideoFast() {
  await openVideoExportDialog("fast");
}

async function exportVideoRecording() {
  await openVideoExportDialog("browser");
}

function showUdocStructure() {
  if (!state.deck) return;
  const dialog = $("#udocStructureDialog");
  dialog.showModal();
  $$(".status-view").forEach((button) => button.classList.toggle("active", button.id === "viewUdocStructure"));
  renderUdocStructure(true);
}

function closeUdocStructure() {
  const dialog = $("#udocStructureDialog");
  if (dialog.open) dialog.close();
  syncUdocStructureClosedState();
}

function syncUdocStructureClosedState() {
  cancelAnimationFrame(udocStructureRefreshFrame);
  udocStructureRefreshFrame = 0;
  $$(".status-view").forEach((button) => button.classList.toggle("active", button.id === "normalView"));
}

async function copyUdocStructure() {
  const runtime = globalThis.UniPptDocumentCache;
  if (!runtime || !state.documentCache || !state.deck) return;
  const button = $("#copyUdocStructure");
  const original = button.textContent;
  button.disabled = true;
  button.textContent = "正在生成…";
  // Opening and browsing never stringify media. Full JSON generation is kept
  // behind this explicit action and yielded one frame so the UI can repaint.
  await new Promise((resolve) => requestAnimationFrame(resolve));
  try {
    const snapshot = runtime.structure(state.documentCache, state.deck);
    await navigator.clipboard.writeText(JSON.stringify(snapshot, null, 2));
    toast("UDoc 完整结构 JSON 已复制");
  } finally {
    button.disabled = false;
    button.textContent = original;
  }
}

function scheduleUdocStructureRefresh() {
  if (!$("#udocStructureDialog")?.open || udocStructureRefreshFrame) return;
  udocStructureRefreshFrame = requestAnimationFrame(() => {
    udocStructureRefreshFrame = 0;
    renderUdocStructure();
  });
}

function renderUdocStructure(force = false) {
  const runtime = globalThis.UniPptDocumentCache;
  const dialog = $("#udocStructureDialog");
  const content = $("#udocStructureContent");
  if (!runtime || !state.documentCache || !state.deck || !dialog?.open || !content) return;
  const revision = state.documentCache.revision;
  if (!force && Number(content.dataset.revision) === revision) return;
  const expanded = new Set([...content.querySelectorAll(".udoc-tree-row[aria-expanded=true]")]
    .map((row) => row.dataset.path));
  const scrollTop = content.scrollTop;
  const snapshot = runtime.structure(state.documentCache, state.deck);
  const fragment = document.createDocumentFragment();
  for (const [key, value] of Object.entries(snapshot)) {
    fragment.appendChild(createUdocTreeNode(key, value, `/${key}`, expanded));
  }
  content.replaceChildren(fragment);
  content.dataset.revision = String(revision);
  content.scrollTop = scrollTop;
  updateUdocCacheSummary();
}

function createUdocTreeNode(key, value, path, expandedPaths) {
  const wrapper = element("div", "udoc-tree-node");
  const row = element("div", "udoc-tree-row");
  row.setAttribute("role", "treeitem");
  row.dataset.path = path;
  const toggle = element("button", "udoc-tree-toggle", "");
  toggle.type = "button";
  const label = element("span", "udoc-tree-key", String(key));
  row.append(toggle, label);
  wrapper.appendChild(row);

  if (value && typeof value === "object") {
    const entries = Array.isArray(value)
      ? value.map((entry, index) => [index, entry])
      : Object.entries(value);
    row.appendChild(element("span", "udoc-tree-summary", Array.isArray(value)
      ? `Array(${entries.length})`
      : `{${entries.length} 项}`));
    const children = element("div", "udoc-tree-children");
    children.setAttribute("role", "group");
    children.hidden = true;
    wrapper.appendChild(children);
    let rendered = 0;
    const appendBatch = () => {
      const end = Math.min(entries.length, rendered + 100);
      const batch = document.createDocumentFragment();
      for (; rendered < end; rendered += 1) {
        const [childKey, childValue] = entries[rendered];
        batch.appendChild(createUdocTreeNode(
          childKey,
          childValue,
          `${path}/${String(childKey).replaceAll("~", "~0").replaceAll("/", "~1")}`,
          expandedPaths,
        ));
      }
      children.querySelector(".udoc-tree-more")?.remove();
      children.appendChild(batch);
      if (rendered < entries.length) {
        const more = element("button", "udoc-tree-more", `再显示 ${Math.min(100, entries.length - rendered)} 项`);
        more.type = "button";
        more.onclick = appendBatch;
        children.appendChild(more);
      }
    };
    const setOpen = (open) => {
      row.setAttribute("aria-expanded", String(open));
      toggle.textContent = open ? "▼" : "▶";
      children.hidden = !open;
      if (open && rendered === 0) appendBatch();
    };
    toggle.onclick = () => setOpen(row.getAttribute("aria-expanded") !== "true");
    setOpen(expandedPaths.has(path));
    if (!entries.length) toggle.disabled = true;
    return wrapper;
  }

  toggle.disabled = true;
  const type = value === null ? "null" : typeof value;
  const largeSummary = type === "string" ? globalThis.UniPptDocumentCache.stringSummary(value) : null;
  if (largeSummary) {
    row.appendChild(element("span", "udoc-tree-summary", `‹${largeSummary}›`));
    const inspect = element("button", "udoc-tree-action", "分段查看");
    inspect.type = "button";
    inspect.onclick = () => toggleLargeUdocValue(wrapper, inspect, value);
    row.appendChild(inspect);
  } else {
    const shown = type === "string" ? JSON.stringify(value) : String(value);
    row.appendChild(element("span", `udoc-tree-${type}`, shown));
  }
  return wrapper;
}

function toggleLargeUdocValue(wrapper, button, value) {
  const existing = wrapper.querySelector(":scope > .udoc-tree-large-detail");
  if (existing) {
    existing.remove();
    button.textContent = "分段查看";
    return;
  }
  const chunkSize = 16 * 1024;
  let offset = 0;
  const detail = element("div", "udoc-tree-large-detail");
  const toolbar = element("div", "udoc-tree-large-toolbar");
  const previous = element("button", "udoc-tree-action", "上一段");
  const next = element("button", "udoc-tree-action", "下一段");
  const position = element("span", "", "");
  const copy = element("button", "udoc-tree-action", "复制完整值");
  const chunk = element("pre", "udoc-tree-large-chunk", "");
  previous.type = next.type = copy.type = "button";
  const draw = () => {
    const end = Math.min(value.length, offset + chunkSize);
    chunk.textContent = value.slice(offset, end);
    position.textContent = `${offset.toLocaleString()}–${end.toLocaleString()} / ${value.length.toLocaleString()}`;
    previous.disabled = offset === 0;
    next.disabled = end >= value.length;
  };
  previous.onclick = () => { offset = Math.max(0, offset - chunkSize); draw(); };
  next.onclick = () => { offset = Math.min(value.length - 1, offset + chunkSize); draw(); };
  copy.onclick = async () => {
    await navigator.clipboard.writeText(value);
    toast("完整字段值已复制");
  };
  toolbar.append(previous, next, position, copy);
  detail.append(toolbar, chunk);
  wrapper.appendChild(detail);
  button.textContent = "收起";
  draw();
}

function updateUdocCacheSummary() {
  const cache = state.documentCache;
  if (!cache) return;
  const scene = $("#udocSceneCacheState");
  const source = $("#udocSourceCacheState");
  const output = $("#udocExportCacheState");
  const revision = $("#udocStructureRevision");
  if (scene) {
    scene.textContent = `完整场景已缓存 · r${cache.revision}`;
    scene.className = "ready";
  }
  if (source) {
    const ready = Boolean(cache.cacheId || cache.sourceFile);
    source.textContent = ready ? `原生 PPTX 已缓存${cache.cacheId ? ` · ${String(cache.cacheId).slice(0, 12)}` : ""}` : "无原生源文件";
    source.className = ready ? "ready" : "pending";
  }
  if (output) {
    const ready = cache.exportBodyRevision === cache.revision;
    const cacheOnly = cache.cacheOnlyEligible && cache.cacheId && cache.syncedRevision === cache.revision;
    output.textContent = ready ? "导出载荷已复用" : cacheOnly ? "导出复用服务端缓存" : "导出直接使用当前场景";
    output.className = ready ? "ready" : "pending";
  }
  if (revision) revision.textContent = `实时版本 r${cache.revision} · 大型媒体按需展开`;
}

async function exportPortable(endpoint, extension, mime, pending, success) {
  $("#loading").hidden = false;
  setStatus(pending);
  try {
    const exportRequest = createExportRequest();
    const response = await fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: exportRequest.body,
    });
    if (!response.ok) throw new Error(await readResponseError(response));
    markExportRevisionSynced(response, exportRequest.revision);
    const blob = await response.blob();
    downloadBlob(`${safeFilename(state.deck.title)}.${extension}`, blob.type ? blob : new Blob([blob], { type: mime }));
    setStatus(success);
    toast(success);
  } catch (error) {
    setStatus(`${extension.toUpperCase()} 导出失败`);
    toast(`导出失败：${error.message}`);
  } finally {
    $("#loading").hidden = true;
  }
}

function applySlideBackground(node, slide) {
  node.style.background = slide.background || "#ffffff";
  if (slide.backgroundAsset) {
    node.style.backgroundImage = `url(${JSON.stringify(slide.backgroundAsset)})`;
    node.style.backgroundSize = "100% 100%";
    node.style.backgroundPosition = "center";
    node.style.backgroundRepeat = "no-repeat";
  }
}

function renderAll() {
  if(state.deck)showStartupSurface('ready');
  if (!state.deck) return;
  state.activeSlide = clamp(state.activeSlide, 0, state.deck.slides.length - 1);
  $("#deckTitle").value = state.deck.title;
  document.documentElement.style.setProperty("--deck-ratio", `${state.deck.width}/${state.deck.height}`);
  renderSlideList();
  renderCanvas();
  renderInspector();
  renderAnimationPane();
  syncAnimationRibbon();
  syncTransitionRibbon();
  updateCanvasScale();
  updateHistoryButtons();
}

function renderSlideList() {
  const list = $("#slideList");
  thumbnailResizeObserver?.disconnect();
  thumbnailResizeObserver = null;
  list.replaceChildren();

  if (state.outlineView) {
    state.deck.slides.forEach((slide, index) => {
      const item = element("button", `slide-outline-item${index === state.activeSlide ? " active" : ""}`);
      item.type = "button";
      const text = flattenObjects(slide.objects || []).map((object) => String(object.text || "").trim()).filter(Boolean).join(" · ") || "（无文字）";
      item.append(element("b", "", String(index + 1)), element("span", "", text));
      item.onclick = () => { state.activeSlide = index; state.selectedId = null; renderAll(); };
      item.addEventListener("contextmenu", (event) => openSlideContextMenu(event, index));
      list.append(item);
    });
    return;
  }

  const scaleThumbnail = (thumb) => {
    const scene = thumb.querySelector(".slide-thumb-stage");
    if (!scene) return;
    const scale = Math.min(
      thumb.clientWidth / Math.max(1, state.deck.width),
      thumb.clientHeight / Math.max(1, state.deck.height),
    );
    scene.style.transform = `scale(${scale})`;
  };

  if (globalThis.ResizeObserver) {
    thumbnailResizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) scaleThumbnail(entry.target);
    });
  }

  state.deck.slides.forEach((slide, index) => {
    const item = element("div", `slide-item${index === state.activeSlide ? " active" : ""}`);
    const numberEl = element("span", "slide-number", String(index + 1));
    const thumb = element("div", "slide-thumb");
    const scene = element("div", "slide-thumb-stage");
    scene.style.width = `${state.deck.width}px`;
    scene.style.height = `${state.deck.height}px`;
    applySlideBackground(scene, slide);
    renderSlideObjects(slide, scene, false, true, null);
    thumb.append(scene);
    item.append(numberEl, thumb);
    item.onclick = () => {
      state.activeSlide = index;
      state.selectedId = null;
      renderAll();
    };
    item.addEventListener("contextmenu", (event) => openSlideContextMenu(event, index));
    list.append(item);
    if (thumbnailResizeObserver) thumbnailResizeObserver.observe(thumb);
    else requestAnimationFrame(() => scaleThumbnail(thumb));
  });
}

function openSlideContextMenu(event, index) {
  event.preventDefault();
  event.stopPropagation();
  if (!state.deck?.slides[index]) return;
  closeExportMenu();
  if (state.activeSlide !== index || state.selectedId !== null) {
    state.activeSlide = index;
    state.selectedId = null;
    renderAll();
  }
  const menu = $("#slideContextMenu");
  if (!menu) return;
  $("#slideContextMoveUp").disabled = index === 0;
  $("#slideContextMoveDown").disabled = index === state.deck.slides.length - 1;
  $("#slideContextDelete").disabled = state.deck.slides.length === 1;
  menu.hidden = false;
  menu.style.left = "0px";
  menu.style.top = "0px";
  const margin = 8;
  const rect = menu.getBoundingClientRect();
  menu.style.left = `${clamp(event.clientX, margin, Math.max(margin, innerWidth - rect.width - margin))}px`;
  menu.style.top = `${clamp(event.clientY, margin, Math.max(margin, innerHeight - rect.height - margin))}px`;
  menu.querySelector("button:not(:disabled)")?.focus({ preventScroll: true });
}

function closeSlideContextMenu() {
  const menu = $("#slideContextMenu");
  if (!menu || menu.hidden) return;
  menu.hidden = true;
}

function runSlideContextAction(action) {
  closeSlideContextMenu();
  action();
}

function openCurrentSlideTransitionSettings() {
  closeSlideContextMenu();
  activateRibbon("transitions");
  requestAnimationFrame(() => {
    $("[data-panel='transitions'] [data-transition].active")?.focus({ preventScroll: true });
  });
}

function renderCanvas() {
  const stage = $("#slideStage");
  const slide = currentSlide();
  globalThis.UniPptMedia?.stopAll(stage, true);
  stage.replaceChildren();
  stage.style.width = `${state.deck.width}px`;
  stage.style.height = `${state.deck.height}px`;
  applySlideBackground(stage, slide);
  const mediaContext = { scope: "editor", slideKey: slide.sourcePartName || slide.id };
  renderSlideObjects(slide, stage, true, false, mediaContext);
  syncCanvasSelection();
  $("#slideNotes").value = slide.notes || "";
  setStatus(`幻灯片 ${state.activeSlide + 1} / ${state.deck.slides.length} · ${slide.objects.length} 个对象`);
}

function renderSlideObjects(slide, container, interactive, thumbnail, mediaContext) {
  let z = 0;
  for (const object of slide.masterObjects || []) {
    renderObject(object, container, false, z, thumbnail, mediaContext);
    z += 1;
  }
  for (const object of slide.layoutObjects || []) {
    renderObject(object, container, false, z, thumbnail, mediaContext);
    z += 1;
  }
  for (const object of slide.objects || []) {
    renderObject(object, container, interactive, z, thumbnail, mediaContext);
    z += 1;
  }
  // Thumbnail stages are assembled off-DOM.  Defer measurement until their
  // parent is attached so normAutofit uses the same geometry as the editor.
  queueMicrotask(() => fitAutoTextInContainer(container));
}

function renderReadOnlySlide(slide, container, mediaContext) {
  const runtime = globalThis.UniPptPresentationScene;
  if (!runtime?.renderSlide) {
    renderSlideObjects(slide, container, false, false, mediaContext);
    return;
  }
  runtime.renderSlide(slide, container, {
    clear: false,
    applyBackground: false,
    deckWidth: state.deck.width,
    deckHeight: state.deck.height,
    dynamicObjects: state.deck.extensions?.["org.unippt.dynamic"]?.objects || {},
    mediaContext,
    onHyperlink: (link) => activateObjectHyperlink(link, true),
  });
}

function nextGroupedInteractionId(objectId, groupPath, selectedId, selectedPathIds = []) {
  if (!groupPath?.length) return objectId;
  const selectedGroupIndex = groupPath.indexOf(selectedId);
  if (selectedGroupIndex >= 0) return groupPath[selectedGroupIndex + 1] || objectId;
  let sharedGroupIndex = -1;
  for (let index = 0; index < groupPath.length; index += 1) {
    if (!selectedPathIds.includes(groupPath[index])) break;
    sharedGroupIndex = index;
  }
  return sharedGroupIndex >= 0
    ? (groupPath[sharedGroupIndex + 1] || objectId)
    : groupPath[0];
}

function groupedInteractionTargetId(objectId, groupPath) {
  const selectedPathIds = state.selectedId
    ? (findObjectPath(state.selectedId)?.map((item) => item.id) || [])
    : [];
  return nextGroupedInteractionId(objectId, groupPath, state.selectedId, selectedPathIds);
}

function renderObject(object, container, interactive, z = 0, thumbnail = false, mediaContext = null, selectionGroupPath = []) {
  if (object.kind === "group") {
    if (!object.children?.length) {
      renderPlaceholder(object, container, interactive, z, "组合对象", thumbnail, mediaContext);
      return;
    }
    const group = element("div", "scene-object kind-group");
    group.dataset.id = object.id;
    group.style.zIndex = String(z + 1);
    applyObjectStyle(group, object);
    // Match PowerPoint's selection drill-down: the first click selects the
    // outer group, then another click on a descendant selects the next nested
    // group or the actual child. Keep the complete group path so nested groups
    // can advance one level at a time without losing local coordinates.
    const childSelectionGroupPath = [...selectionGroupPath, object.id];
    object.children.forEach((child, index) => renderObject(child, group, interactive, index, thumbnail, mediaContext, childSelectionGroupPath));
    container.append(group);
    return;
  }
  const node = element("div", `scene-object kind-${object.kind}`);
  node.dataset.id = object.id;
  if (isInkObject(object)) node.classList.add("is-ink-object");
  node.style.zIndex = String(z + 1);
  applyObjectStyle(node, object);
  if (!thumbnail) bindObjectHyperlinks(node, object, interactive);
  const hasCustomGeometry = renderCustomGeometry(node, object) || renderLinearGeometry(node, object);
  if (interactive) {
    const slide = currentSlide();
    const count = [...(slide.inheritedAnimations || []), ...(slide.animations || [])]
      .filter((animation) => animation.targetObjectId === object.id).length;
    if (count) {
      node.classList.add("animation-target");
      node.dataset.animationCount = String(count);
    }
  }
  if (renderDynamicObject(node, object, interactive, thumbnail)) {
    // The isolated sandbox is the visual body; selection stays on the parent.
  } else if (object.kind === "image" && object.asset) {
    const image = new Image();
    image.src = object.asset;
    image.alt = object.name;
    applyImageCrop(image, object.imageCrop);
    node.append(image);
    applyImageEffects(node, image, object.imageEffects);
  } else if (object.kind === "table" && object.table?.rows?.length) {
    renderTable(node, object, interactive);
  } else if (object.kind === "chart" && object.chart) {
    if (!globalThis.UniPPTChartRuntime?.render(node, object.chart)) {
      node.append(element("div", "object-content", object.chart.title || object.name || "Chart"));
    }
  } else if (object.kind === "math") {
    renderMath(node, object.formula);
  } else if (object.textParagraphs?.length) {
    renderRichText(node, object, interactive);
  } else if (!hasCustomGeometry || object.text || placeholderLabel(object)) {
    node.append(element("div", "object-content", object.text || placeholderLabel(object)));
  }
  renderShapeFillImage(node, object);
  if (!thumbnail) globalThis.UniPptMedia?.attach(node, object, interactive, mediaContext);
  if (interactive) {
    node.addEventListener("pointerdown", (event) => {
      const interactionId = groupedInteractionTargetId(object.id, selectionGroupPath);
      const textBody = node.querySelector(":scope > .object-content, :scope > .rich-text");
      const hitResizeHandle = Boolean(event.target.closest?.(".resize-handle, .rotate-handle"));
      const canEditText = interactionId === object.id && Boolean(textBody) && !["connector", "math", "table"].includes(object.kind);
      // A formula is rendered by KaTeX for visual fidelity, but its glyphs
      // are still selectable when the object is already selected.  The first
      // click keeps the normal object-selection gesture; a subsequent drag
      // inside the formula uses the browser Selection API instead of moving
      // the whole object.  Double-click remains the explicit LaTeX/OMML
      // editor entry point below.
      if (interactionId === object.id && object.kind === "math" && state.selectedId === object.id && !hitResizeHandle) {
        event.stopPropagation();
        beginTextDragSelect(node, event);
        return;
      }
      // Editing, or a second press inside an already-selected text box, must
      // extend the native Selection from the press point — including a drag
      // that starts on a mid-line glyph, not only at the end of the run.
      if (canEditText && !hitResizeHandle && (node.classList.contains("editing") || state.selectedId === object.id)) {
        if (!node.classList.contains("editing")) editText(event, object.id);
        else beginTextDragSelect(textBody, event);
        return;
      }
      beginDrag(event, interactionId);
    });
    node.addEventListener("contextmenu", (event) => {
      if (isTextEditingTarget(event.target)) return;
      event.preventDefault();
      event.stopPropagation();
      const interactionId = groupedInteractionTargetId(object.id, selectionGroupPath);
      openFormatPane(interactionId);
    });
    node.addEventListener("dblclick", (event) => {
      const interactionId = groupedInteractionTargetId(object.id, selectionGroupPath);
      if (interactionId !== object.id) {
        event.stopPropagation();
        selectObject(interactionId);
        return;
      }
      if (object.kind === "math") {
        event.stopPropagation();
        selectObject(object.id);
        openFormulaDialog(object.id);
      } else if (object.kind === "chart") {
        event.stopPropagation();
        selectObject(object.id);
        openInsertObjectDialog("chart", object.id);
      } else {
        editText(event, object.id);
      }
    });
  }
  container.append(node);
}

function imageCropMetrics(crop, width = 100, height = 100) {
  const left = clamp(Number(crop?.left) || 0, 0, 0.9999);
  const top = clamp(Number(crop?.top) || 0, 0, 0.9999);
  const right = clamp(Number(crop?.right) || 0, 0, 0.9999);
  const bottom = clamp(Number(crop?.bottom) || 0, 0, 0.9999);
  const visibleWidth = Math.max(0.0001, 1 - Math.min(0.9999, left + right));
  const visibleHeight = Math.max(0.0001, 1 - Math.min(0.9999, top + bottom));
  return {
    x: -left / visibleWidth * width,
    y: -top / visibleHeight * height,
    width: width / visibleWidth,
    height: height / visibleHeight,
  };
}

function applyImageCrop(image, crop) {
  const metrics = imageCropMetrics(crop);
  Object.assign(image.style, {
    left: `${metrics.x}%`,
    top: `${metrics.y}%`,
    width: `${metrics.width}%`,
    height: `${metrics.height}%`,
  });
}

function applyImageEffects(host, image, effects) {
  const radius = Math.max(0, Number(effects?.softEdgeRadius) || 0);
  if (radius > 0) {
    const width = Math.max(1, parseFloat(host.style.width) || host.offsetWidth || 1);
    const height = Math.max(1, parseFloat(host.style.height) || host.offsetHeight || 1);
    const x = Math.min(50, radius / width * 100);
    const y = Math.min(50, radius / height * 100);
    const mask = `linear-gradient(to right, transparent 0%, #000 ${x}%, #000 ${100 - x}%, transparent 100%), linear-gradient(to bottom, transparent 0%, #000 ${y}%, #000 ${100 - y}%, transparent 100%)`;
    image.style.maskImage = mask;
    image.style.maskComposite = "intersect";
    image.style.webkitMaskImage = mask;
    image.style.webkitMaskComposite = "source-in";
  }
  const duotone = effects?.duotone;
  const color = (value) => /^#[0-9a-f]{6}$/i.test(value || "")
    ? [1, 3, 5].map((offset) => parseInt(value.slice(offset, offset + 2), 16) / 255)
    : null;
  const shadow = color(duotone?.shadowColor);
  const highlight = color(duotone?.highlightColor);
  if (!shadow || !highlight) return;
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  const filter = document.createElementNS(ns, "filter");
  const grayscale = document.createElementNS(ns, "feColorMatrix");
  const transfer = document.createElementNS(ns, "feComponentTransfer");
  const id = `unippt-duotone-${String(host.dataset.id || "image").replace(/[^a-z0-9_-]/gi, "-")}-${Math.random().toString(36).slice(2)}`;
  filter.id = id;
  filter.setAttribute("color-interpolation-filters", "sRGB");
  grayscale.setAttribute("type", "saturate");
  grayscale.setAttribute("values", "0");
  transfer.append(...[0, 1, 2].map((channel) => {
    const fn = document.createElementNS(ns, `feFunc${"RGB"[channel]}`);
    fn.setAttribute("type", "table");
    fn.setAttribute("tableValues", `${shadow[channel]} ${highlight[channel]}`);
    return fn;
  }));
  filter.append(grayscale, transfer);
  const defs = document.createElementNS(ns, "defs");
  defs.append(filter);
  svg.append(defs);
  Object.assign(svg.style, { position: "absolute", width: "0", height: "0", overflow: "hidden" });
  svg.setAttribute("aria-hidden", "true");
  host.append(svg);
  image.style.filter = `url(#${id})`;
}

function imageFillMetrics(crop, fillRect, width = 100, height = 100) {
  const left = Number(fillRect?.left) || 0;
  const top = Number(fillRect?.top) || 0;
  const right = Number(fillRect?.right) || 0;
  const bottom = Number(fillRect?.bottom) || 0;
  const destination = {
    x: left * width,
    y: top * height,
    width: Math.max(0.0001, (1 - left - right) * width),
    height: Math.max(0.0001, (1 - top - bottom) * height),
  };
  const source = imageCropMetrics(crop, destination.width, destination.height);
  return {
    x: destination.x + source.x,
    y: destination.y + source.y,
    width: source.width,
    height: source.height,
  };
}

function renderShapeFillImage(node, object) {
  if (!object.shapeFillAsset || object.customGeometry || object.kind === "image" || /\brepeat\b/i.test(object.style?.fill || "")) return false;
  const frame = object.frame || {};
  const metrics = imageFillMetrics(
    object.imageCrop,
    object.imageFillRect,
    Math.max(1, Number(frame.width) || 1),
    Math.max(1, Number(frame.height) || 1),
  );
  const image = new Image();
  image.className = "shape-fill-image";
  image.src = object.shapeFillAsset;
  image.alt = "";
  Object.assign(image.style, {
    left: `${metrics.x}px`,
    top: `${metrics.y}px`,
    width: `${metrics.width}px`,
    height: `${metrics.height}px`,
  });
  node.prepend(image);
  return true;
}

function isLinearGeometry(object) {
  const frame = object?.frame || {};
  const style = object?.style || {};
  const width = Math.abs(Number(frame.width) || 0);
  const height = Math.abs(Number(frame.height) || 0);
  const strokeWidth = Number(style.strokeWidth) || 0;
  const stroke = String(style.stroke || "").trim().toLowerCase();
  const fill = String(style.fill || "").trim().toLowerCase();
  const hasVisibleStroke = strokeWidth > 0 && stroke && stroke !== "transparent" && stroke !== "none";
  const hasNoFill = !fill || fill === "transparent" || fill === "none";
  const hasText = Boolean(String(object?.text || "").trim()) || (object?.textParagraphs || []).some(
    (paragraph) => (paragraph.runs || []).some((run) => Boolean(String(run.text || "").trim())),
  );
  const degenerateFrame = width <= 0.5 || height <= 0.5;
  const linePreset = /(?:^|[^a-z])(?:line|straightconnector)/i.test(String(object?.geometry || ""));
  return hasVisibleStroke && !hasText && hasNoFill && (degenerateFrame || linePreset);
}

function renderLinearGeometry(node, object) {
  if (!isLinearGeometry(object)) return false;
  const svgNamespace = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(svgNamespace, "svg");
  const line = document.createElementNS(svgNamespace, "line");
  const frame = object.frame || {};
  const horizontal = Math.abs(Number(frame.height) || 0) <= 0.5;
  const vertical = Math.abs(Number(frame.width) || 0) <= 0.5;
  svg.classList.add("linear-geometry-svg");
  svg.setAttribute("viewBox", "0 0 1000 1000");
  svg.setAttribute("preserveAspectRatio", "none");
  svg.setAttribute("aria-hidden", "true");
  line.setAttribute("x1", vertical ? "500" : "0");
  line.setAttribute("y1", horizontal ? "500" : "0");
  line.setAttribute("x2", vertical ? "500" : "1000");
  line.setAttribute("y2", horizontal ? "500" : "1000");
  line.setAttribute("stroke", object.style.stroke);
  line.setAttribute("stroke-width", String(Math.max(0.25, Number(object.style.strokeWidth) || 1)));
  line.setAttribute("stroke-linecap", "butt");
  line.setAttribute("vector-effect", "non-scaling-stroke");
  line.setAttribute("stroke-dasharray", /dot/i.test(object.style.strokeDash || "") ? "1 2" : /dash/i.test(object.style.strokeDash || "") ? "6 4" : "none");
  svg.append(line);
  node.classList.add("has-linear-geometry");
  node.style.background = "transparent";
  node.style.border = "0";
  node.style.padding = "0";
  node.style.overflow = "visible";
  node.append(svg);
  return true;
}

function renderCustomGeometry(node, object) {
  const geometry = object.customGeometry;
  const geometryWidth = Number(geometry?.width) || 0;
  const geometryHeight = Number(geometry?.height) || 0;
  if (!geometry?.pathData || geometryWidth < 0 || geometryHeight < 0 || !(geometryWidth > 0 || geometryHeight > 0)) return false;
  const svgNamespace = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(svgNamespace, "svg");
  svg.classList.add("custom-geometry-svg");
  const viewBoxX = geometryWidth > 0 ? 0 : -0.5;
  const viewBoxY = geometryHeight > 0 ? 0 : -0.5;
  svg.setAttribute("viewBox", `${viewBoxX} ${viewBoxY} ${Math.max(1, geometryWidth)} ${Math.max(1, geometryHeight)}`);
  svg.setAttribute("preserveAspectRatio", "none");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(svgNamespace, "path");
  path.setAttribute("d", geometry.pathData);
  if (object.shapeFillAsset) {
    const defs = document.createElementNS(svgNamespace, "defs");
    const pattern = document.createElementNS(svgNamespace, "pattern");
    const patternId = `shape-fill-${String(object.id || "shape").replace(/[^a-zA-Z0-9_-]/g, "-")}-${crypto.randomUUID?.() || Math.random().toString(36).slice(2)}`;
    pattern.id = patternId;
    pattern.setAttribute("patternUnits", "userSpaceOnUse");
    pattern.setAttribute("width", String(Math.max(1, geometryWidth)));
    pattern.setAttribute("height", String(Math.max(1, geometryHeight)));
    const image = document.createElementNS(svgNamespace, "image");
    image.setAttribute("href", object.shapeFillAsset);
    const crop = imageFillMetrics(
      object.imageCrop,
      object.imageFillRect,
      Math.max(1, geometryWidth),
      Math.max(1, geometryHeight),
    );
    image.setAttribute("x", String(crop.x));
    image.setAttribute("y", String(crop.y));
    image.setAttribute("width", String(crop.width));
    image.setAttribute("height", String(crop.height));
    image.setAttribute("preserveAspectRatio", "none");
    pattern.append(image);
    defs.append(pattern);
    svg.append(defs);
    path.setAttribute("fill", `url(#${patternId})`);
  } else {
    path.setAttribute("fill", object.style.fill || "transparent");
  }
  path.setAttribute("fill-rule", "evenodd");
  path.setAttribute("stroke", object.style.stroke || "transparent");
  path.setAttribute("stroke-width", String(Math.max(0, object.style.strokeWidth || 0)));
  path.setAttribute("stroke-dasharray", /dot/i.test(object.style.strokeDash || "") ? "1 2" : /dash/i.test(object.style.strokeDash || "") ? "6 4" : "none");
  path.setAttribute("vector-effect", "non-scaling-stroke");
  if (isInkObject(object)) {
    // A freehand stroke can have a very large, mostly empty bounding box.
    // Targeting that box makes dragging an apparently unrelated object move
    // the ink instead.  Keep a forgiving invisible stroke as the hit target,
    // while the visible path remains purely visual.
    const hitPath = document.createElementNS(svgNamespace, "path");
    hitPath.classList.add("ink-hit-path");
    hitPath.setAttribute("d", geometry.pathData);
    hitPath.setAttribute("fill", "none");
    hitPath.setAttribute("stroke", "transparent");
    hitPath.setAttribute("stroke-width", String(Math.max(12, Number(object.style.strokeWidth || 0) + 8)));
    hitPath.setAttribute("stroke-linecap", "round");
    hitPath.setAttribute("stroke-linejoin", "round");
    hitPath.setAttribute("vector-effect", "non-scaling-stroke");
    svg.append(hitPath);
  }
  svg.append(path);
  node.classList.add("has-custom-geometry");
  node.style.background = "transparent";
  node.style.border = "0";
  node.style.padding = "0";
  node.style.overflow = "visible";
  node.append(svg);
  return true;
}

function renderTable(node, object, interactive) {
  const table = object.table;
  const grid = element("div", "ppt-table-grid");
  const columnWeights = table.columns?.length
    ? table.columns
    : Array.from({ length: Math.max(1, ...table.rows.map((row) => row.cells?.length || 0)) }, () => 1);
  const rowWeights = table.rows.map((row) => Math.max(1, Number(row.height) || 1));
  grid.style.gridTemplateColumns = columnWeights.map((value) => `minmax(0,${Math.max(1, Number(value) || 1)}fr)`).join(" ");
  grid.style.gridTemplateRows = rowWeights.map((value) => `minmax(0,${value}fr)`).join(" ");

  table.rows.forEach((row, rowIndex) => {
    (row.cells || []).forEach((cell, columnIndex) => {
      // DrawingML keeps covered merge cells in the XML. Only the merge origin
      // is painted, matching PowerPoint and preventing duplicate text/borders.
      if (cell.hMerge || cell.vMerge) return;
      const cellNode = element("div", "ppt-table-cell");
      cellNode.dataset.row = String(rowIndex);
      cellNode.dataset.column = String(columnIndex);
      cellNode.style.gridColumn = `${columnIndex + 1} / span ${Math.max(1, cell.gridSpan || 1)}`;
      cellNode.style.gridRow = `${rowIndex + 1} / span ${Math.max(1, cell.rowSpan || 1)}`;
      cellNode.style.background = cell.fill || "transparent";
      cellNode.style.color = cell.textStyle?.color || object.textStyle?.color || "#172033";
      cellNode.style.fontFamily = cell.textStyle?.fontFamily || object.textStyle?.fontFamily || "Aptos,sans-serif";
      cellNode.style.fontSize = `${cell.textStyle?.fontSize || object.textStyle?.fontSize || 18}px`;
      cellNode.style.fontWeight = cell.textStyle?.bold ? "700" : "400";
      cellNode.style.fontStyle = cell.textStyle?.italic ? "italic" : "normal";
      cellNode.style.textAlign = cell.textStyle?.align || "left";
      const textFrame = cell.textFrame || {};
      cellNode.dataset.textFlow = textFlowMode(textFrame.verticalType);
      cellNode.style.padding = `${textFrame.marginTop ?? 5}px ${textFrame.marginRight ?? 8}px ${textFrame.marginBottom ?? 5}px ${textFrame.marginLeft ?? 8}px`;
      cellNode.style.alignItems = textFrame.verticalAlign === "top" ? "flex-start" : textFrame.verticalAlign === "bottom" ? "flex-end" : "center";
      applyTableCellBorders(cellNode, cell.borders || {});
      if (cell.textParagraphs?.length) {
        renderRichText(cellNode, cell, interactive);
      } else {
        cellNode.append(element("div", "object-content", cell.text || ""));
      }
      if (interactive) {
        cellNode.addEventListener("dblclick", (event) => editTableCell(event, object.id, rowIndex, columnIndex));
      }
      grid.append(cellNode);
    });
  });
  node.style.padding = "0";
  node.append(grid);
}

function editTableCell(event, objectId, rowIndex, columnIndex) {
  const object = findObject(objectId);
  const cell = object?.table?.rows?.[rowIndex]?.cells?.[columnIndex];
  const content = event.currentTarget.querySelector(".object-content, .rich-text");
  if (!cell || !content) return;
  event.preventDefault();
  event.stopPropagation();
  selectObject(objectId);
  checkpoint();
  const previous = cell.text || "";
  content.contentEditable = "true";
  content.classList.add("editing-content");
  content.focus();
  placeTextCaret(content, event.clientX, event.clientY);
  let cancelled = false;
  content.addEventListener("keydown", (keyEvent) => {
    if (keyEvent.key !== "Escape") return;
    keyEvent.preventDefault();
    cancelled = true;
    content.blur();
  });
  content.addEventListener("blur", () => {
    content.contentEditable = "false";
    content.classList.remove("editing-content");
    const value = normalizeEditedText(content.innerText);
    if (!cancelled && value !== previous) {
      cell.text = value;
      cell.textParagraphs = [];
      state.future = [];
      markDeckChanged();
    } else {
      state.history.pop();
    }
    renderAll();
  }, { once: true });
}

function applyTableCellBorders(node, borders) {
  for (const [side, cssName] of [["left", "borderLeft"], ["right", "borderRight"], ["top", "borderTop"], ["bottom", "borderBottom"]]) {
    const border = borders[side];
    node.style[cssName] = border
      ? `${Math.max(0, border.width || 0)}px ${dashCss(border.dash)} ${border.color || "transparent"}`
      : "0 solid transparent";
  }
}

function bindObjectHyperlinks(node, object, interactive) {
  bindHyperlinks(node, object.hyperlinks, interactive);
}

function bindHyperlinks(node, hyperlinks, interactive) {
  const click = hyperlinks?.click;
  const hover = hyperlinks?.hover;
  if (!click && !hover) return;
  node.dataset.hyperlink = "true";
  const tooltip = click?.tooltip || hover?.tooltip;
  if (tooltip) node.title = tooltip;
  if (click) {
    node.addEventListener("click", (event) => {
      if (interactive && !(event.ctrlKey || event.metaKey)) return;
      if (activateObjectHyperlink(click, !interactive)) {
        event.preventDefault();
        event.stopPropagation();
      }
    });
  }
  if (hover && !interactive) {
    node.addEventListener("pointerenter", (event) => {
      if (node.dataset.hoverLinkActivated === "1") return;
      if (activateObjectHyperlink(hover, true)) {
        node.dataset.hoverLinkActivated = "1";
        event.stopPropagation();
      }
    });
  }
}

function activateObjectHyperlink(link, presenting) {
  if (!link) return false;
  const action = String(link.action || "").toLowerCase();
  const setSlide = (index) => {
    if (index < 0 || index >= state.deck.slides.length) return false;
    if (presenting) {
      state.presenterSlide = index;
      state.presenterAnimationCursor = 0;
      renderPresentation(true);
    } else {
      state.activeSlide = index;
      state.selectedId = null;
      renderAll();
    }
    return true;
  };
  if (action.includes("jump=nextslide")) return setSlide((presenting ? state.presenterSlide : state.activeSlide) + 1);
  if (action.includes("jump=previousslide")) return setSlide((presenting ? state.presenterSlide : state.activeSlide) - 1);
  if (action.includes("jump=firstslide")) return setSlide(0);
  if (action.includes("jump=lastslide")) return setSlide(state.deck.slides.length - 1);
  if (action.includes("jump=endshow")) {
    if (presenting) closePresentation();
    return presenting;
  }
  if (link.target && !link.external) {
    const index = state.deck.slides.findIndex((slide) => slide.sourcePartName === link.target);
    if (index >= 0) return setSlide(index);
  }
  if (isSafeHyperlink(link.target)) {
    window.open(link.target, "_blank", "noopener,noreferrer");
    return true;
  }
  return false;
}

function isSafeHyperlink(target) {
  return /^(https?:|mailto:|tel:)/i.test(String(target || "").trim());
}

function renderRichText(node, object, interactive) {
  const flow = element("div", "object-content rich-text");
  populateRichText(flow, object, interactive);
  node.append(flow);
}

function populateRichText(flow, object, interactive) {
  let textOffset = 0;
  for (const [paragraphIndex, paragraph] of (object.textParagraphs || []).entries()) {
    const line = element("div", "ppt-paragraph");
    line.dataset.paragraphIndex = String(paragraphIndex);
    line.dataset.textStart = String(textOffset);
    line.style.whiteSpace = object.textFrame?.wordWrap === false ? "pre" : "pre-wrap";
    const paragraphFontSize = Math.max(
      1,
      ...(paragraph.runs || []).map((run) => Number(run.fontSize) || Number(object.textStyle.fontSize) || 24),
    );
    line.style.fontSize = `${paragraphFontSize}px`;
    const paragraphAlign = paragraph.align || object.textStyle.align || "left";
    line.style.textAlign = cssTextAlign(paragraphAlign);
    line.style.textAlignLast = isDistributedAlign(paragraphAlign) ? "justify" : "auto";
    line.style.paddingLeft = `${Math.max(0, paragraph.level || 0) * 24}px`;
    line.style.lineHeight = String(paragraph.lineSpacing || 1.18);
    if (paragraph.spaceBefore != null) line.style.marginTop = `${paragraph.spaceBefore}px`;
    if (paragraph.spaceAfter != null) line.style.marginBottom = `${paragraph.spaceAfter}px`;
    if (paragraph.bullet) {
      const bullet = element("span", "ppt-bullet", paragraph.bullet);
      line.append(bullet);
    }
    for (const [runIndex, run] of (paragraph.runs || []).entries()) {
      const span = element("span", "ppt-run", run.text || "");
      span.dataset.paragraphIndex = String(paragraphIndex);
      span.dataset.runIndex = String(runIndex);
      span.dataset.textStart = String(textOffset);
      textOffset += String(run.text || "").length;
      span.dataset.textEnd = String(textOffset);
      Object.assign(span.style, {
        fontFamily: run.fontFamily || object.textStyle.fontFamily,
        fontSize: `${run.fontSize || object.textStyle.fontSize || 24}px`,
        color: run.color || object.textStyle.color,
        fontWeight: run.bold ? "700" : "400",
        fontStyle: run.italic ? "italic" : "normal",
        textDecoration: `${run.underline ? "underline" : ""}${run.strikethrough ? " line-through" : ""}`.trim() || "none",
        textDecorationStyle: /wavy/i.test(run.underlineStyle || "") ? "wavy" : /dbl/i.test(run.underlineStyle || "") ? "double" : "solid",
        verticalAlign: Number.isFinite(run.baselineOffset) ? `${run.baselineOffset * (run.fontSize || object.textStyle.fontSize || 24) / 100}px` : run.baseline === "super" ? "super" : run.baseline === "sub" ? "sub" : "baseline",
        opacity: run.alpha ?? 1,
      });
      applyTextGradient(span, run.gradient);
      bindHyperlinks(span, run.hyperlinks, interactive);
      line.append(span);
    }
    line.dataset.textEnd = String(textOffset);
    flow.append(line);
    if (paragraphIndex < object.textParagraphs.length - 1) textOffset += 1;
  }
}

function renderMath(node, formula) {
  const source = formula?.latex || "Office 公式";
  if (window.katex && formula?.latex) {
    katex.render(source, node, { displayMode: Boolean(formula.display), throwOnError: false });
  } else {
    node.textContent = source;
  }
}

function renderPlaceholder(object, container, interactive, z, label, thumbnail = false, mediaContext = null) {
  const copy = structuredClone(object);
  copy.kind = "placeholder";
  copy.text = label;
  renderObject(copy, container, interactive, z, thumbnail, mediaContext);
}

function applyObjectStyle(node, object) {
  const f = object.frame;
  const textFrame = object.textFrame || {};
  const textAlign = object.textStyle.align || "left";
  const radius = /round|ellipse|arc|cloud/i.test(object.geometry || "") ? (/ellipse/i.test(object.geometry || "") ? "50%" : "16px") : "0";
  Object.assign(node.style, {
    left: `${f.x}px`, top: `${f.y}px`, width: `${Math.max(1, f.width)}px`, height: `${Math.max(1, f.height)}px`,
    // DrawingML xfrm/ext describes the outside of the shape. Text insets and
    // strokes therefore belong inside that exact box, just as they do in
    // PowerPoint; never let a surrounding CSS reset turn them into extra width.
    boxSizing: "border-box", minWidth: "0", minHeight: "0",
    transform: objectTransform(object),
    justifyContent: object.textStyle.align === "center" ? "center" : object.textStyle.align === "right" ? "flex-end" : "flex-start",
    alignItems: textFrame.verticalAlign === "top" ? "flex-start" : textFrame.verticalAlign === "bottom" ? "flex-end" : "center",
    padding: `${textFrame.marginTop ?? 5}px ${textFrame.marginRight ?? 8}px ${textFrame.marginBottom ?? 5}px ${textFrame.marginLeft ?? 8}px`,
    whiteSpace: textFrame.wordWrap === false ? "pre" : "pre-wrap",
    boxShadow: shadowCss(object.style.shadow),
  });
  node.dataset.textAutoSize = textFrame.autoSize || "none";
  node.dataset.textFlow = textFlowMode(textFrame.verticalType);
  node.dataset.textWrap = textFrame.wordWrap === false ? "none" : "wrap";
  const tiledImageFill = Boolean(object.shapeFillAsset && /\brepeat\b/i.test(object.style?.fill || ""));
  if (tiledImageFill && !object.customGeometry) {
    node.style.backgroundImage = `url(${JSON.stringify(object.shapeFillAsset)})`;
    node.style.backgroundRepeat = "repeat";
  }
  node.dataset.animationBase = objectTransform(object);
  node.style.setProperty("--object-fill", object.shapeFillAsset && !tiledImageFill ? "transparent" : object.style.fill || "transparent");
  node.style.setProperty("--object-stroke", object.style.stroke || "transparent");
  node.style.setProperty("--object-border", `${object.style.strokeWidth || 0}px`);
  node.style.setProperty("--object-border-style", dashCss(object.style.strokeDash));
  node.style.setProperty("--object-opacity", object.style.opacity ?? 1);
  node.style.setProperty("--text-color", object.textStyle.color || "#172033");
  node.style.setProperty("--font-family", object.textStyle.fontFamily || "Aptos, sans-serif");
  node.style.setProperty("--font-size", `${object.textStyle.fontSize || 24}px`);
  node.style.setProperty("--font-weight", object.textStyle.bold ? "700" : "400");
  node.style.setProperty("--font-style", object.textStyle.italic ? "italic" : "normal");
  node.style.setProperty("--text-align", cssTextAlign(textAlign));
  node.style.textAlignLast = isDistributedAlign(textAlign) ? "justify" : "auto";
  node.style.setProperty("--shape-radius", radius);
}

function textFlowMode(verticalType) {
  switch (String(verticalType || "horz")) {
    case "eaVert": return "vertical-rl-upright";
    case "vert":
    case "wordArtVert": return "vertical-rl-mixed";
    case "vert270":
    case "wordArtVertRtl": return "vertical-lr-mixed";
    case "mongolianVert": return "vertical-lr-upright";
    default: return "horizontal";
  }
}

function fitAutoTextInContainer(container, attachmentRetry = false) {
  if (!container?.querySelectorAll) return;
  if (!container.isConnected) {
    // A thumbnail may attach on the next frame, but an obsolete stage removed
    // by another render must not retain a perpetual animation-frame loop.
    if (!attachmentRetry) requestAnimationFrame(() => fitAutoTextInContainer(container, true));
    return;
  }
  for (const node of container.querySelectorAll('[data-text-auto-size="textToFitShape"]')) {
    fitTextToShape(node);
  }
  for (const node of container.querySelectorAll('[data-text-wrap="none"][data-text-flow^="vertical-"]')) {
    fitVerticalNoWrapText(node);
  }
}

function fitVerticalNoWrapText(node) {
  const flow = node.querySelector(":scope > .rich-text");
  if (!flow) return;
  const availableHeight = Math.max(1, flow.clientHeight);
  for (const paragraph of flow.querySelectorAll(":scope > .ppt-paragraph")) {
    paragraph.style.letterSpacing = "";
    if (paragraph.scrollHeight <= availableHeight + .5) continue;

    // Chromium uses the embedded font's vertical advance metrics, while
    // PowerPoint lays East-Asian upright glyphs on the font-size grid. Some
    // CJK fonts therefore exceed the saved text-box height even though Office
    // fits the same no-wrap paragraph on one column. Preserve the glyph size
    // and tighten only inter-glyph advance until the native paragraph fits.
    const fontSize = Math.max(1, parseFloat(getComputedStyle(paragraph).fontSize) || 1);
    let fitting = -fontSize * .5;
    let overflowing = 0;
    paragraph.style.letterSpacing = `${fitting}px`;
    if (paragraph.scrollHeight > availableHeight + .5) continue;
    for (let index = 0; index < 12; index += 1) {
      const middle = (fitting + overflowing) / 2;
      paragraph.style.letterSpacing = `${middle}px`;
      if (paragraph.scrollHeight <= availableHeight + .5) fitting = middle;
      else overflowing = middle;
    }
    paragraph.style.letterSpacing = `${fitting - .05}px`;
  }
}

function fitTextToShape(node) {
  const flow = node.querySelector(":scope > .rich-text");
  if (!flow) return 1;
  const runs = [...flow.querySelectorAll(".ppt-run")];
  const paragraphs = [...flow.querySelectorAll(".ppt-paragraph")];
  if (!runs.length || !paragraphs.length) return 1;

  for (const run of runs) {
    run.dataset.baseFontSize ||= String(parseFloat(run.style.fontSize) || 1);
  }
  for (const paragraph of paragraphs) {
    paragraph.dataset.baseFontSize ||= String(parseFloat(paragraph.style.fontSize) || 1);
  }

  const computed = getComputedStyle(node);
  const availableWidth = Math.max(1, node.clientWidth
    - (parseFloat(computed.paddingLeft) || 0)
    - (parseFloat(computed.paddingRight) || 0));
  const availableHeight = Math.max(1, node.clientHeight
    - (parseFloat(computed.paddingTop) || 0)
    - (parseFloat(computed.paddingBottom) || 0));
  const cacheKey = [
    Math.round(availableWidth * 10), Math.round(availableHeight * 10),
    node.dataset.id || "", flow.textContent || "",
    runs.map((run) => `${run.dataset.baseFontSize}:${run.style.fontFamily}`).join("|"),
  ].join(";");

  const applyScale = (scale) => {
    for (const run of runs) run.style.fontSize = `${Math.max(.5, Number(run.dataset.baseFontSize) * scale)}px`;
    for (const paragraph of paragraphs) paragraph.style.fontSize = `${Math.max(.5, Number(paragraph.dataset.baseFontSize) * scale)}px`;
  };
  const fits = () => flow.scrollWidth <= availableWidth + .5 && flow.scrollHeight <= availableHeight + .5;

  const cached = autoTextFitCache.get(cacheKey);
  if (Number.isFinite(cached)) {
    applyScale(cached);
    if (fits()) return cached;
  }
  applyScale(1);
  if (fits()) {
    autoTextFitCache.set(cacheKey, 1);
    return 1;
  }
  let low = .12;
  let high = 1;
  for (let index = 0; index < 12; index += 1) {
    const middle = (low + high) / 2;
    applyScale(middle);
    if (fits()) low = middle;
    else high = middle;
  }
  // A tiny safety margin matches PowerPoint's conservative glyph bounds and
  // avoids clipping the final digit on narrow year labels.
  const result = Math.max(.1, low * .985);
  applyScale(result);
  autoTextFitCache.set(cacheKey, result);
  return result;
}

function isDistributedAlign(value) {
  return value === "distributed" || value === "thaiDistributed";
}

function cssTextAlign(value) {
  return isDistributedAlign(value) ? "justify" : value === "justifyLow" ? "justify" : value;
}

function dashCss(value) {
  if (/dot/i.test(value || "")) return "dotted";
  if (/dash/i.test(value || "")) return "dashed";
  return "solid";
}

function shadowCss(shadow) {
  if (!shadow) return "none";
  const color = cssColorWithOpacity(shadow.color || "#000000", shadow.opacity ?? 0.35);
  const inset = shadow.inset ? "inset " : "";
  return `${inset}${shadow.offsetX || 0}px ${shadow.offsetY || 0}px ${Math.max(0, shadow.blur || 0)}px ${color}`;
}

function cssColorWithOpacity(color, opacity) {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color || "");
  if (!match) return color;
  const rgb = match.slice(1).map((value) => parseInt(value, 16));
  return `rgba(${rgb[0]}, ${rgb[1]}, ${rgb[2]}, ${Math.max(0, Math.min(1, opacity))})`;
}

function textGradientCss(gradient) {
  if (!gradient?.stops?.length) return null;
  const stops = gradient.stops.map((stop) =>
    `${cssColorWithOpacity(stop.color || "#000000", stop.opacity ?? 1)} ${clamp(Number(stop.position) || 0, 0, 1) * 100}%`
  );
  return `linear-gradient(${Number(gradient.angle) || 0}deg, ${stops.join(", ")})`;
}

function applyTextGradient(node, gradient) {
  const fill = textGradientCss(gradient);
  if (!fill) return;
  node.style.backgroundImage = fill;
  node.style.backgroundClip = "text";
  node.style.webkitBackgroundClip = "text";
  node.style.color = "transparent";
  node.style.webkitTextFillColor = "transparent";
}

function objectTransform(object) {
  const frame = object.frame || {};
  return `rotate(${frame.rotation || 0}deg) scaleX(${object.flipH ? -1 : 1}) scaleY(${object.flipV ? -1 : 1})`;
}

function multiplyAffine(parent, local) {
  return {
    a: parent.a * local.a + parent.c * local.b,
    b: parent.b * local.a + parent.d * local.b,
    c: parent.a * local.c + parent.c * local.d,
    d: parent.b * local.c + parent.d * local.d,
    e: parent.a * local.e + parent.c * local.f + parent.e,
    f: parent.b * local.e + parent.d * local.f + parent.f,
  };
}

function objectAffine(object) {
  const frame = object?.frame || {};
  const x = Number(frame.x) || 0;
  const y = Number(frame.y) || 0;
  const width = Math.max(1, Number(frame.width) || 1);
  const height = Math.max(1, Number(frame.height) || 1);
  const radians = (Number(frame.rotation) || 0) * Math.PI / 180;
  const cosine = Math.cos(radians);
  const sine = Math.sin(radians);
  const scaleX = object?.flipH ? -1 : 1;
  const scaleY = object?.flipV ? -1 : 1;
  const a = cosine * scaleX;
  const b = sine * scaleX;
  const c = -sine * scaleY;
  const d = cosine * scaleY;
  const centerX = width / 2;
  const centerY = height / 2;
  return {
    a, b, c, d,
    e: x + centerX - a * centerX - c * centerY,
    f: y + centerY - b * centerX - d * centerY,
  };
}

function selectionGeometryForPath(path) {
  if (!path?.length) return null;
  let matrix = { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
  for (const object of path) matrix = multiplyAffine(matrix, objectAffine(object));
  const object = path.at(-1);
  const width = Math.max(1, Number(object.frame?.width) || 1);
  const height = Math.max(1, Number(object.frame?.height) || 1);
  const centerX = matrix.a * width / 2 + matrix.c * height / 2 + matrix.e;
  const centerY = matrix.b * width / 2 + matrix.d * height / 2 + matrix.f;
  return {
    left: centerX - width / 2,
    top: centerY - height / 2,
    width,
    height,
    rotation: Math.atan2(matrix.b, matrix.a) * 180 / Math.PI,
  };
}

function findObjectPath(id, objects = currentSlide().objects, ancestors = []) {
  for (const object of objects || []) {
    const path = [...ancestors, object];
    if (object.id === id) return path;
    const childPath = object.children?.length ? findObjectPath(id, object.children, path) : null;
    if (childPath) return childPath;
  }
  return null;
}

function selectionGeometry(id) {
  return selectionGeometryForPath(findObjectPath(id));
}

function applySelectionGeometry(selection, id) {
  const geometry = selectionGeometry(id);
  if (!selection || !geometry) return;
  Object.assign(selection.style, {
    left: `${geometry.left}px`,
    top: `${geometry.top}px`,
    width: `${geometry.width}px`,
    height: `${geometry.height}px`,
    transform: `rotate(${geometry.rotation}deg)`,
  });
}

function placeholderLabel(object) {
  return ({ chart: "图表", table: "表格", smartArt: "SmartArt", ole: "OLE 嵌入对象", unknown: object.name || "未识别对象", connector: "" })[object.kind] || "";
}

function beginDrag(event, id) {
  if (event.button !== 0 || event.target.closest?.(".resize-handle, .rotate-handle")) return;
  const object = findObject(id);
  if (!object || event.target.closest(".editing")) return;
  event.stopPropagation();
  if (state.selectedId !== id) selectObject(id);
  const pointerId = event.pointerId;
  const captureTarget = event.currentTarget;
  try { captureTarget.setPointerCapture?.(pointerId); } catch (_) { /* pointer already ended */ }
  const start = { x: event.clientX, y: event.clientY, left: object.frame.x, top: object.frame.y };
  let dragging = false;
  let active = true;
  const move = (e) => {
    if (!active || e.pointerId !== pointerId) return;
    if (!dragging && Math.hypot(e.clientX - start.x, e.clientY - start.y) < 3) return;
    if (!dragging) {
      checkpoint();
      dragging = true;
    }
    object.frame.x = clamp(start.left + (e.clientX - start.x) / state.zoom, -object.frame.width + 12, state.deck.width - 12);
    object.frame.y = clamp(start.top + (e.clientY - start.y) / state.zoom, -object.frame.height + 12, state.deck.height - 12);
    const node = $("#slideStage")?.querySelector(`.scene-object[data-id="${cssEscape(id)}"]`);
    if (node) { node.style.left = `${object.frame.x}px`; node.style.top = `${object.frame.y}px`; }
    applySelectionGeometry(selectionOverlayFor(id), id);
    renderInspector();
  };
  const finish = (finishEvent) => {
    if (!active || (finishEvent?.pointerId != null && finishEvent.pointerId !== pointerId)) return;
    active = false;
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", finish);
    window.removeEventListener("pointercancel", finish);
    window.removeEventListener("blur", finish);
    try {
      if (captureTarget.hasPointerCapture?.(pointerId)) captureTarget.releasePointerCapture(pointerId);
    } catch (_) { /* detached node or completed pointer */ }
    if (!dragging) return;
    state.future = [];
    markDeckChanged();
    renderSlideList();
    updateHistoryButtons();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", finish);
  window.addEventListener("pointercancel", finish);
  window.addEventListener("blur", finish);
}

function beginResize(event, id, dir) {
  event.preventDefault();
  event.stopPropagation();
  const object = findObject(id);
  if (!object) return;
  checkpoint();
  const start = { x: event.clientX, y: event.clientY, frame: { ...object.frame } };
  const move = (e) => {
    const dx = (e.clientX - start.x) / state.zoom;
    const dy = (e.clientY - start.y) / state.zoom;
    const f = object.frame, s = start.frame;
    if (dir.includes("e")) f.width = Math.max(12, s.width + dx);
    if (dir.includes("s")) f.height = Math.max(12, s.height + dy);
    if (dir.includes("w")) { f.x = Math.min(s.x + dx, s.x + s.width - 12); f.width = Math.max(12, s.width - dx); }
    if (dir.includes("n")) { f.y = Math.min(s.y + dy, s.y + s.height - 12); f.height = Math.max(12, s.height - dy); }
    renderCanvas();
    renderInspector();
  };
  const up = () => {
    window.removeEventListener("pointermove", move);
    state.future = [];
    markDeckChanged();
    renderSlideList();
    updateHistoryButtons();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up, { once: true });
}

function pointerAngleDegrees(centerX, centerY, pointerX, pointerY) {
  return Math.atan2(pointerY - centerY, pointerX - centerX) * 180 / Math.PI;
}

function rotationFromPointerAngles(startRotation, startAngle, currentAngle, snap = false) {
  const shortestDelta = ((currentAngle - startAngle + 540) % 360) - 180;
  let rotation = Number(startRotation || 0) + shortestDelta;
  if (snap) rotation = Math.round(rotation / 15) * 15;
  rotation = ((rotation % 360) + 360) % 360;
  return Math.round(rotation * 10) / 10;
}

function beginRotate(event, id) {
  if (event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  const object = findObject(id);
  const selection = selectionOverlayFor(id);
  if (!object || !selection) return;
  const bounds = selection.getBoundingClientRect();
  const centerX = bounds.left + bounds.width / 2;
  const centerY = bounds.top + bounds.height / 2;
  const startAngle = pointerAngleDegrees(centerX, centerY, event.clientX, event.clientY);
  const startRotation = Number(object.frame.rotation) || 0;
  const pointerId = event.pointerId;
  const captureTarget = event.currentTarget;
  const historyLength = state.history.length;
  checkpoint();
  let changed = false;
  let active = true;
  try { captureTarget.setPointerCapture?.(pointerId); } catch (_) { /* pointer already ended */ }
  const move = (moveEvent) => {
    if (!active || moveEvent.pointerId !== pointerId) return;
    const angle = pointerAngleDegrees(centerX, centerY, moveEvent.clientX, moveEvent.clientY);
    const rotation = rotationFromPointerAngles(startRotation, startAngle, angle, moveEvent.shiftKey);
    if (rotation === object.frame.rotation) return;
    object.frame.rotation = rotation;
    changed = true;
    const node = $("#slideStage")?.querySelector(`.scene-object[data-id="${cssEscape(id)}"]`);
    const transform = objectTransform(object);
    if (node) {
      node.style.transform = transform;
      node.dataset.animationBase = transform;
    }
    applySelectionGeometry(selection, id);
    renderInspector();
  };
  const finish = (finishEvent) => {
    if (!active || (finishEvent?.pointerId != null && finishEvent.pointerId !== pointerId)) return;
    active = false;
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", finish);
    window.removeEventListener("pointercancel", finish);
    window.removeEventListener("blur", finish);
    try {
      if (captureTarget.hasPointerCapture?.(pointerId)) captureTarget.releasePointerCapture(pointerId);
    } catch (_) { /* detached node or completed pointer */ }
    if (!changed) {
      if (state.history.length === historyLength + 1) state.history.pop();
      return;
    }
    state.future = [];
    markDeckChanged();
    renderSlideList();
    updateHistoryButtons();
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", finish);
  window.addEventListener("pointercancel", finish);
  window.addEventListener("blur", finish);
}

function editText(event, id) {
  const object = findObject(id);
  const node = event.currentTarget;
  const content = node.querySelector(".object-content, .rich-text");
  if (!object || !content || ["connector", "math", "table"].includes(object.kind)) return;
  event.stopPropagation();
  selectObject(id);
  const previousText = normalizeEditedText(object.text);
  const historyLength = state.history.length;
  checkpoint();
  node.classList.add("editing");
  content.classList.add("editing-content");
  content.contentEditable = "true";
  content.querySelectorAll(".ppt-bullet").forEach((bullet) => {
    bullet.contentEditable = "false";
  });
  content.focus({ preventScroll: true });
  const pointerGesture = event.type === "pointerdown" || event.type === "mousedown";
  if (pointerGesture) beginTextDragSelect(content, event);
  else if (event.type !== "dblclick") placeTextCaret(content, event.clientX, event.clientY);

  let cancelled = false;
  const finish = () => {
    const modelChanged = content.dataset.richTextModelChanged === "true";
    content.contentEditable = "false";
    content.classList.remove("editing-content");
    node.classList.remove("editing");
    const nextText = readEditableText(content);
    const changed = !cancelled && nextText !== previousText;
    if (changed) {
      replaceRichTextContent(object, nextText);
      state.future = [];
      markDeckChanged();
    } else if (!modelChanged && state.history.length === historyLength + 1) {
      state.history.pop();
    }
    if (state.textSelection?.objectId === id) state.textSelection = null;
    renderAll();
  };
  content.addEventListener("keydown", (keyEvent) => {
    if ((keyEvent.ctrlKey || keyEvent.metaKey) && !keyEvent.altKey) {
      const command = keyEvent.key.toLowerCase();
      if (["b", "i", "u"].includes(command)) {
        keyEvent.preventDefault();
        if (command === "b") toggleSelectedStyle("bold");
        if (command === "i") toggleSelectedStyle("italic");
        if (command === "u") toggleSelectedRunStyle("underline");
        return;
      }
    }
    if (keyEvent.key !== "Escape") return;
    keyEvent.preventDefault();
    cancelled = true;
    content.blur();
  });
  // Moving focus to the mini toolbar is part of the same text-editing
  // gesture. Do not commit/re-render the editor on that intermediate blur:
  // native selects fire selectionchange and would otherwise destroy the live
  // range before onchange can apply the requested character formatting.
  const finishOnBlur = () => {
    if (document.activeElement?.closest?.("#miniFormatToolbar")) return;
    content.removeEventListener("blur", finishOnBlur);
    finish();
  };
  content.addEventListener("blur", finishOnBlur);
}

function editingContentForObject(objectId) {
  const node = $("#slideStage")?.querySelector(`.scene-object[data-id="${cssEscape(objectId)}"].editing`);
  return node?.querySelector(":scope > .object-content, :scope > .rich-text") || null;
}

function preserveTextSelectionForCommand(event) {
  if (rememberActiveTextSelection()) event.preventDefault();
}

function scheduleMiniFormatToolbar() {
  if (miniFormatContext.frame) return;
  miniFormatContext.frame = requestAnimationFrame(() => { miniFormatContext.frame = 0; syncMiniFormatToolbar(); });
}

function miniFormatRange(object) {
  return globalThis.UniPptMiniFormat?.rangeFor(state.textSelection, object)
    || globalThis.UniPptMiniFormat?.rangeFor(miniFormatContext.range, object);
}

function miniFormatRuns(object) {
  const range = miniFormatRange(object), paragraphs = object?.textParagraphs || [];
  return range ? richTextRunsInRange(paragraphs, range.start, range.end) : paragraphs.flatMap(p => p.runs || []);
}

function applyMiniFormatProperty(property, resolveValue) {
  const object = selectedObject();
  if (!globalThis.UniPptMiniFormat?.canFormat(object)) return;
  if (applySelectedTextProperty(property, resolveValue)) { scheduleMiniFormatToolbar(); return; }
  const range = miniFormatRange(object), runs = miniFormatRuns(object);
  const value = resolveValue(runs.length ? runs : [object.textStyle], object);
  updateSelected(target => {
    if (range) updateRichTextRange(ensureRichTextParagraphs(target), range.start, range.end, property, value);
    else if (property === 'fontFamily') applyNativeFontFamily(target, value);
    else if (typeof value === 'function') {
      for (const p of ensureRichTextParagraphs(target)) for (const run of p.runs) run[property] = value(run);
      const first = representativeTextRun(target); if (first) target.textStyle[property] = first[property];
    } else applyTextStyle(target, property, value);
  });
}

function setMiniParagraphAlignment(align) {
  const object = selectedObject(); if (!object) return;
  const range = miniFormatRange(object);
  // Leaving the live editor commits any newly typed text before an object
  // transaction. The captured character range survives focus in the toolbar.
  editingContentForObject(object.id)?.blur();
  updateSelected(target => {
    let cursor = 0;
    for (const p of ensureRichTextParagraphs(target)) {
      const end = cursor + p.runs.reduce((n,r) => n + String(r.text || '').length, 0);
      if (!range || range.end > cursor && range.start <= end) p.align = align;
      cursor = end + 1;
    }
    if (!range) target.textStyle.align = align;
  });
}

function bindMiniFormatToolbar() {
  const toolbar = $('#miniFormatToolbar'); if (!toolbar) return;
  toolbar.addEventListener('pointerdown', event => {
    if (rememberActiveTextSelection()) miniFormatContext.range = { ...state.textSelection };
    if (event.target.closest('button')) event.preventDefault();
    event.stopPropagation();
  });
  toolbar.addEventListener('click', event => {
    const command = event.target.closest('[data-mini]')?.dataset.mini;
    if (['bold','italic','underline'].includes(command)) applyMiniFormatProperty(command, runs => !runs.every(r => Boolean(r[command])));
    else if (['grow','shrink'].includes(command)) applyMiniFormatProperty('fontSize', (_runs,object) => run => clamp((Number(run.fontSize) || object.textStyle.fontSize) * 72 / 96 + (command === 'grow' ? 2 : -2), 1, 400) * 96 / 72);
    else if (['left','center','right'].includes(command)) setMiniParagraphAlignment(command);
    else if (command === 'more') openFormatPane();
    else if (command === 'close') { miniFormatContext.dismissedId = state.selectedId; toolbar.hidden = true; }
    scheduleMiniFormatToolbar();
  });
  $('#miniFontFamily').onchange = event => applyMiniFormatProperty('fontFamily', () => event.target.value);
  const commitSize = event => {
    const value = Number(event.target.value), object = selectedObject();
    if (!Number.isFinite(value) || value <= 0 || !object) { scheduleMiniFormatToolbar(); return; }
    const pixels = clamp(value,1,400) * 96 / 72, runs = miniFormatRuns(object);
    if ((runs.length ? runs : [object.textStyle]).every(run => Math.abs(Number(run.fontSize) - pixels) < .01)) return;
    applyMiniFormatProperty('fontSize', () => pixels);
  };
  $('#miniFontSize').onchange = commitSize;
  $('#miniFontSize').onblur = commitSize;
  $('#miniFontSize').onkeydown = event => { if (event.key === 'Enter') { event.preventDefault(); event.stopPropagation(); commitSize(event); } };
  $('#miniFontColor').onchange = event => applyMiniFormatProperty('color', () => event.target.value);
  document.addEventListener('selectionchange', scheduleMiniFormatToolbar);
  document.addEventListener('pointerdown', event => {
    if (toolbar.contains(event.target)) return;
    miniFormatContext.range = null; miniFormatContext.dismissedId = null;
    if (event.target.closest('#slideStage')) { miniFormatContext.dragging = true; toolbar.hidden = true; }
    else if (!event.target.closest('#miniFormatToolbar')) { toolbar.hidden = true; miniFormatContext.dismissedId = state.selectedId; }
  }, true);
  document.addEventListener('pointerup', () => { miniFormatContext.dragging = false; scheduleMiniFormatToolbar(); });
  document.addEventListener('pointercancel', () => { miniFormatContext.dragging = false; scheduleMiniFormatToolbar(); });
  document.addEventListener('keydown', event => {
    if (event.key === 'Escape' && !toolbar.hidden) { miniFormatContext.dismissedId = state.selectedId; toolbar.hidden = true; }
  });
  $('#canvasViewport').addEventListener('scroll', scheduleMiniFormatToolbar, {passive:true});
  window.addEventListener('resize', scheduleMiniFormatToolbar);
}

function syncMiniFormatToolbar() {
  const toolbar = $('#miniFormatToolbar'), object = selectedObject(), runtime = globalThis.UniPptMiniFormat;
  if (!toolbar) return;
  // PowerPoint exposes the mini toolbar for a non-empty character
  // selection, not merely because a text box is selected.  Requiring the
  // captured range here also prevents a single click on a text box from
  // flashing the toolbar before the user has dragged across any glyphs.
  const selectedRange = miniFormatRange(object);
  const visible = runtime?.canFormat(object) && Boolean(selectedRange) && !miniFormatContext.dragging && miniFormatContext.dismissedId !== object.id
    && $('#presenter').hidden && !document.querySelector('dialog[open]');
  if (!visible) { toolbar.hidden = true; return; }
  const node = $('#slideStage')?.querySelector(`.scene-object[data-id="${cssEscape(object.id)}"]`);
  if (!node) { toolbar.hidden = true; return; }
  let anchor = node.getBoundingClientRect();
  const selection = window.getSelection();
  if (selection?.rangeCount && !selection.isCollapsed && node.contains(selection.anchorNode) && node.contains(selection.focusNode)) anchor = selection.getRangeAt(0).getBoundingClientRect();
  const viewport = $('#canvasViewport').getBoundingClientRect();
  if (anchor.bottom < viewport.top || anchor.top > viewport.bottom || anchor.right < viewport.left || anchor.left > viewport.right) { toolbar.hidden = true; return; }
  toolbar.hidden = false;
  const focused = document.activeElement, runs = miniFormatRuns(object), run = runs[0] || representativeTextRun(object) || object.textStyle;
  if (focused !== $('#miniFontFamily')) {
    const family = run.nativeFontFamily || primaryCssFont(run.fontFamily || object.textStyle.fontFamily), select = $('#miniFontFamily');
    const families = new Set([...$('#ribbonFontFamily').options].map(o => o.value));
    for (const value of ['Arial','Times New Roman','Cambria Math','微软雅黑','等线',family]) families.add(value);
    for (const value of families) ensureSelectOption(select,value);
    select.value = family;
  }
  if (focused !== $('#miniFontSize')) $('#miniFontSize').value = String(Math.round(Number(run.fontSize || object.textStyle.fontSize) * 72 / 96 * 10) / 10);
  if (focused !== $('#miniFontColor')) $('#miniFontColor').value = normalizeColor(run.color || object.textStyle.color, '#172033');
  for (const property of ['bold','italic','underline']) toolbar.querySelector(`[data-mini="${property}"]`).setAttribute('aria-pressed', String(runs.length ? runs.every(r => Boolean(r[property])) : Boolean(object.textStyle[property])));
  const align = object.textParagraphs?.find(p => p.align)?.align || object.textStyle.align || 'left';
  for (const value of ['left','center','right']) toolbar.querySelector(`[data-mini="${value}"]`).setAttribute('aria-pressed', String(align === value));
  const position = runtime.placement(anchor, toolbar.getBoundingClientRect(), {left:Math.max(0,viewport.left),top:Math.max(0,viewport.top),right:Math.min(innerWidth,viewport.right),bottom:Math.min(innerHeight,viewport.bottom)});
  toolbar.style.left = position.x + 'px'; toolbar.style.top = position.y + 'px';
}

function rememberActiveTextSelection() {
  const object = selectedObject();
  const content = object ? editingContentForObject(object.id) : null;
  if (!object || !content) return false;
  const browserSelection = window.getSelection?.() || document.getSelection?.();
  if (!browserSelection?.rangeCount) return false;
  const range = browserSelection.getRangeAt(0);
  if (!content.contains(range.startContainer) || !content.contains(range.endContainer)) return false;
  const totalLength = normalizeEditedText(object.text || "").length;
  const start = textSelectionBoundaryOffset(content, range.startContainer, range.startOffset, totalLength);
  const end = textSelectionBoundaryOffset(content, range.endContainer, range.endOffset, totalLength);
  if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
    state.textSelection = null;
    return false;
  }
  state.textSelection = { objectId: object.id, start, end };
  return true;
}

function activeTextSelection(object) {
  const content = object ? editingContentForObject(object.id) : null;
  if (!content) return null;
  rememberActiveTextSelection();
  // Focusing a native select/color control fires selectionchange and can
  // collapse the browser range before the toolbar command runs. The
  // pointerdown handler captures the range in miniFormatContext first; use
  // that immutable character interval as the command fallback and rebuild
  // the live Selection after the rich-text DOM refresh.
  const saved = state.textSelection?.objectId === object.id && state.textSelection.end > state.textSelection.start
    ? state.textSelection
    : miniFormatContext.range;
  if (!saved || saved.objectId !== object.id || saved.end <= saved.start) return null;
  return { ...saved, content };
}

function textSelectionBoundaryOffset(content, node, offset, totalLength) {
  const owner = node?.nodeType === 3 ? node.parentElement : node;
  const run = owner?.closest?.(".ppt-run");
  if (run && content.contains(run)) {
    const runStart = Number(run.dataset.textStart);
    if (!Number.isFinite(runStart)) return null;
    if (node.nodeType === 3) return clamp(runStart + clamp(offset, 0, node.data.length), 0, totalLength);
    try {
      const localRange = document.createRange();
      localRange.setStart(run, 0);
      localRange.setEnd(node, clamp(offset, 0, node.childNodes.length));
      return clamp(runStart + localRange.toString().length, 0, totalLength);
    } catch (_) { return clamp(runStart, 0, totalLength); }
  }
  if (node?.nodeType === 3 && node.parentElement === content) {
    return clamp(offset, 0, totalLength);
  }
  const paragraph = owner?.closest?.(".ppt-paragraph");
  if (paragraph && content.contains(paragraph)) {
    const paragraphStart = Number(paragraph.dataset.textStart) || 0;
    if (node === paragraph) {
      const children = [...paragraph.childNodes].slice(0, clamp(offset, 0, paragraph.childNodes.length));
      const previousRun = children.reverse().find((child) => child.nodeType === 1 && child.matches?.(".ppt-run"));
      return previousRun ? Number(previousRun.dataset.textEnd) : paragraphStart;
    }
    return paragraphStart;
  }
  if (node === content) {
    const nextParagraph = content.childNodes[clamp(offset, 0, content.childNodes.length)];
    return nextParagraph?.dataset?.textStart != null ? Number(nextParagraph.dataset.textStart) : totalLength;
  }
  return null;
}

function richTextRunsInRange(paragraphs, selectionStart, selectionEnd) {
  const selected = [];
  let cursor = 0;
  for (const [paragraphIndex, paragraph] of (paragraphs || []).entries()) {
    for (const run of paragraph.runs || []) {
      const runEnd = cursor + String(run.text || "").length;
      if (selectionStart < runEnd && selectionEnd > cursor) selected.push(run);
      cursor = runEnd;
    }
    if (paragraphIndex < paragraphs.length - 1) cursor += 1;
  }
  return selected;
}

function updateRichTextRange(paragraphs, selectionStart, selectionEnd, property, value) {
  let cursor = 0;
  let changed = false;
  for (const [paragraphIndex, paragraph] of (paragraphs || []).entries()) {
    const nextRuns = [];
    for (const run of paragraph.runs || []) {
      const text = String(run.text || "");
      const runStart = cursor;
      const runEnd = runStart + text.length;
      const selectedStart = Math.max(runStart, selectionStart);
      const selectedEnd = Math.min(runEnd, selectionEnd);
      if (selectedEnd <= selectedStart) {
        nextRuns.push(run);
        cursor = runEnd;
        continue;
      }
      const localStart = selectedStart - runStart;
      const localEnd = selectedEnd - runStart;
      let sourceRetained = false;
      const appendFragment = (fragmentText, selected) => {
        if (!fragmentText) return;
        const fragment = { ...run, text: fragmentText };
        if (sourceRetained) fragment.sourceIndex = null;
        else sourceRetained = true;
        if (selected) {
          const nextValue = typeof value === "function" ? value(run) : value;
          if (fragment[property] !== nextValue) changed = true;
          fragment[property] = nextValue;
          if (property === "color") fragment.gradient = null;
          if (property === "fontFamily") fragment.nativeFontFamily = primaryCssFont(nextValue);
        }
        nextRuns.push(fragment);
      };
      appendFragment(text.slice(0, localStart), false);
      appendFragment(text.slice(localStart, localEnd), true);
      appendFragment(text.slice(localEnd), false);
      cursor = runEnd;
    }
    paragraph.runs = nextRuns;
    if (paragraphIndex < paragraphs.length - 1) cursor += 1;
  }
  return changed;
}

function textBoundaryAtOffset(content, offset) {
  const runs = [...content.querySelectorAll(".ppt-run")];
  for (const run of runs) {
    const start = Number(run.dataset.textStart);
    const end = Number(run.dataset.textEnd);
    if (!Number.isFinite(start) || !Number.isFinite(end) || offset < start || offset > end) continue;
    const textNode = run.firstChild || run.appendChild(document.createTextNode(""));
    return { node: textNode, offset: clamp(offset - start, 0, textNode.data.length) };
  }
  const last = runs.at(-1);
  if (last) {
    const textNode = last.firstChild || last.appendChild(document.createTextNode(""));
    return { node: textNode, offset: textNode.data.length };
  }
  const textNode = content.firstChild || content.appendChild(document.createTextNode(""));
  return { node: textNode, offset: clamp(offset, 0, textNode.data?.length || 0) };
}

function restoreActiveTextSelection(content, start, end) {
  const from = textBoundaryAtOffset(content, start);
  const to = textBoundaryAtOffset(content, end);
  const browserSelection = window.getSelection?.() || document.getSelection?.();
  if (!from || !to || !browserSelection) return;
  content.focus({ preventScroll: true });
  if (typeof browserSelection.setBaseAndExtent === "function") {
    browserSelection.setBaseAndExtent(from.node, from.offset, to.node, to.offset);
  } else {
    const range = document.createRange();
    range.setStart(from.node, from.offset);
    range.setEnd(to.node, to.offset);
    browserSelection.removeAllRanges();
    browserSelection.addRange(range);
  }
}

function refreshActiveRichTextEditor(object, content, start, end) {
  content.replaceChildren();
  content.classList.add("rich-text", "editing-content");
  populateRichText(content, object, true);
  content.querySelectorAll(".ppt-bullet").forEach((bullet) => { bullet.contentEditable = "false"; });
  restoreActiveTextSelection(content, start, end);
  fitVerticalNoWrapText(content.closest(".scene-object"));
}

function applySelectedTextProperty(property, resolveValue) {
  const object = selectedObject();
  const selectedRange = object ? activeTextSelection(object) : null;
  if (!object || !selectedRange) return false;
  const paragraphs = ensureRichTextParagraphs(object);
  const selectedRuns = richTextRunsInRange(paragraphs, selectedRange.start, selectedRange.end);
  if (!selectedRuns.length) return false;
  const value = resolveValue(selectedRuns, object);
  if (!updateRichTextRange(paragraphs, selectedRange.start, selectedRange.end, property, value)) return true;
  selectedRange.content.dataset.richTextModelChanged = "true";
  state.future = [];
  globalThis.UniPptPresentationHost?.enrichSemantics?.(state.deck);
  markDeckChanged();
  refreshActiveRichTextEditor(object, selectedRange.content, selectedRange.start, selectedRange.end);
  state.textSelection = { objectId: object.id, start: selectedRange.start, end: selectedRange.end };
  renderSlideList();
  renderInspector();
  updateHistoryButtons();
  if (!state.hostCommitDepth) state.documentHost?._emit?.("change", { revision: state.documentCache?.revision || 0, source: "editor" });
  return true;
}

function readEditableText(content) {
  const bullets = [...content.querySelectorAll(".ppt-bullet")];
  const hiddenStates = bullets.map((bullet) => bullet.hidden);
  bullets.forEach((bullet) => { bullet.hidden = true; });
  const value = normalizeEditedText(content.innerText).replace(/\n{3,}/g, "\n\n");
  bullets.forEach((bullet, index) => { bullet.hidden = hiddenStates[index]; });
  return value;
}

function caretRangeAt(content, clientX, clientY) {
  if (!content || !Number.isFinite(clientX) || !Number.isFinite(clientY)) return null;
  let range = null;
  if (document.caretPositionFromPoint) {
    const position = document.caretPositionFromPoint(clientX, clientY);
    if (position?.offsetNode) {
      range = document.createRange();
      range.setStart(position.offsetNode, position.offset);
      range.collapse(true);
    }
  } else if (document.caretRangeFromPoint) {
    range = document.caretRangeFromPoint(clientX, clientY);
  }
  if (!range || !content.contains(range.startContainer)) return null;
  return range;
}

function setDirectionalTextSelection(selection, anchor, focus) {
  const anchorNode = anchor.startContainer;
  const anchorOffset = anchor.startOffset;
  const focusNode = focus.startContainer;
  const focusOffset = focus.startOffset;
  if (typeof selection.setBaseAndExtent === "function") {
    selection.setBaseAndExtent(anchorNode, anchorOffset, focusNode, focusOffset);
    return;
  }
  selection.removeAllRanges();
  const range = document.createRange();
  range.setStart(anchorNode, anchorOffset);
  range.collapse(true);
  selection.addRange(range);
  if (typeof selection.extend === "function") {
    selection.extend(focusNode, focusOffset);
    return;
  }
  const normalized = document.createRange();
  if (anchor.compareBoundaryPoints(Range.START_TO_START, focus) <= 0) {
    normalized.setStart(anchorNode, anchorOffset);
    normalized.setEnd(focusNode, focusOffset);
  } else {
    normalized.setStart(focusNode, focusOffset);
    normalized.setEnd(anchorNode, anchorOffset);
  }
  selection.removeAllRanges();
  selection.addRange(normalized);
}

function beginTextDragSelect(content, event) {
  if (!content || event.button !== 0) return;
  const selection = window.getSelection?.() || document.getSelection?.();
  if (!selection) return;
  const pointerId = event.pointerId;
  let origin = caretRangeAt(content, event.clientX, event.clientY);
  if (!origin) {
    placeTextCaret(content, event.clientX, event.clientY);
    origin = selection.rangeCount ? selection.getRangeAt(0).cloneRange() : null;
  } else {
    setDirectionalTextSelection(selection, origin, origin);
  }
  if (!origin) return;
  const move = (moveEvent) => {
    if (moveEvent.pointerId !== pointerId) return;
    const focus = caretRangeAt(content, moveEvent.clientX, moveEvent.clientY);
    if (!focus) return;
    try {
      setDirectionalTextSelection(selection, origin, focus);
    } catch (_) { /* hit-tested node left the tree mid-gesture */ }
  };
  const stop = () => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", stop);
    window.removeEventListener("pointercancel", stop);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", stop);
  window.addEventListener("pointercancel", stop);
}

function placeTextCaret(content, clientX, clientY) {
  const selection = window.getSelection?.() || document.getSelection?.();
  if (!selection) return;
  const range = caretRangeAt(content, clientX, clientY) || (() => {
    const fallback = document.createRange();
    fallback.selectNodeContents(content);
    fallback.collapse(false);
    return fallback;
  })();
  selection.removeAllRanges();
  selection.addRange(range);
}

function replaceRichTextContent(object, value) {
  const nextText = normalizeEditedText(value);
  const previousText = normalizeEditedText(object.text);
  object.text = nextText;

  const paragraphs = Array.isArray(object.textParagraphs) ? object.textParagraphs : [];
  if (!paragraphs.length || nextText === previousText) return;

  object.textParagraphs = reconcileRichTextParagraphs(
    paragraphs,
    nextText.split("\n"),
    object.textStyle || defaultTextStyle(),
  );
}

function normalizeEditedText(value) {
  return String(value ?? "").replace(/\r\n?/g, "\n");
}

function paragraphText(paragraph) {
  return (paragraph?.runs || []).map((run) => run.text || "").join("");
}

function reconcileRichTextParagraphs(paragraphs, nextLines, textStyle) {
  const previousLines = paragraphs.map(paragraphText);
  const anchors = unchangedParagraphAnchors(previousLines, nextLines);
  const result = [];
  let previousIndex = 0;
  let nextIndex = 0;

  for (const [anchorPrevious, anchorNext] of [...anchors, [paragraphs.length, nextLines.length]]) {
    const previousCount = anchorPrevious - previousIndex;
    const nextCount = anchorNext - nextIndex;
    const pairedCount = Math.min(previousCount, nextCount);

    for (let offset = 0; offset < pairedCount; offset++) {
      result.push(updateRichTextParagraph(paragraphs[previousIndex + offset], nextLines[nextIndex + offset], textStyle));
    }
    for (let offset = pairedCount; offset < nextCount; offset++) {
      const template = result.at(-1) || paragraphs[anchorPrevious] || paragraphs.at(-1);
      result.push(createRichTextParagraph(template, nextLines[nextIndex + offset], textStyle));
    }

    if (anchorPrevious < paragraphs.length && anchorNext < nextLines.length) {
      // An unchanged paragraph keeps every Run and its source identity byte-for-byte.
      result.push(paragraphs[anchorPrevious]);
    }
    previousIndex = anchorPrevious + 1;
    nextIndex = anchorNext + 1;
  }
  return result;
}

function unchangedParagraphAnchors(previousLines, nextLines) {
  const rows = previousLines.length + 1;
  const columns = nextLines.length + 1;
  const lengths = Array.from({ length: rows }, () => new Uint32Array(columns));
  for (let i = previousLines.length - 1; i >= 0; i--) {
    for (let j = nextLines.length - 1; j >= 0; j--) {
      lengths[i][j] = previousLines[i] === nextLines[j]
        ? lengths[i + 1][j + 1] + 1
        : Math.max(lengths[i + 1][j], lengths[i][j + 1]);
    }
  }

  const anchors = [];
  for (let i = 0, j = 0; i < previousLines.length && j < nextLines.length;) {
    if (previousLines[i] === nextLines[j]) {
      anchors.push([i++, j++]);
    } else if (lengths[i + 1][j] >= lengths[i][j + 1]) {
      i++;
    } else {
      j++;
    }
  }
  return anchors;
}

function updateRichTextParagraph(paragraph, text, textStyle) {
  if (paragraphText(paragraph) === text) return paragraph;
  const updated = { ...paragraph };
  const firstRun = paragraph?.runs?.[0];
  updated.runs = [{ ...(firstRun || richTextRunFromStyle(textStyle)), text }];
  return updated;
}

function createRichTextParagraph(template, text, textStyle) {
  const paragraph = template
    ? { ...template, runs: undefined, sourceIndex: null }
    : { align: textStyle.align || "left", level: 0, bullet: null, lineSpacing: null, spaceBefore: null, spaceAfter: null, sourceIndex: null };
  const templateRun = template?.runs?.[0];
  paragraph.runs = [{ ...(templateRun || richTextRunFromStyle(textStyle)), text, sourceIndex: null }];
  return paragraph;
}

function richTextRunFromStyle(textStyle) {
  return {
    text: "",
    fontFamily: textStyle.fontFamily || "Aptos, sans-serif",
    nativeFontFamily: textStyle.nativeFontFamily || primaryCssFont(textStyle.fontFamily),
    fontSize: textStyle.fontSize || 24,
    color: textStyle.color || "#172033",
    bold: Boolean(textStyle.bold),
    italic: Boolean(textStyle.italic),
    underline: false,
    strikethrough: false,
    baseline: "normal",
    hyperlinks: { click: null, hover: null },
    sourceIndex: null,
  };
}

function selectObject(id) {
  if (id !== state.selectedId) { miniFormatContext.range = null; miniFormatContext.dismissedId = null; }
  if (id !== state.selectedId) state.textSelection = null;
  state.selectedId = id;
  if (id && state.formatPainter && id !== state.formatPainter.sourceId) {
    const format = state.formatPainter;
    state.formatPainter = null;
    $("#formatPainter")?.classList.remove("active");
    mutate(() => {
      const target = findObject(id);
      if (!target) return;
      target.style = structuredClone(format.style);
      target.textStyle = structuredClone(format.textStyle);
      target.textFrame = structuredClone(format.textFrame);
      for (const paragraph of target.textParagraphs || []) {
        paragraph.align = target.textStyle.align || paragraph.align;
        for (const run of paragraph.runs || []) {
          for (const property of ["fontFamily", "nativeFontFamily", "fontSize", "color", "bold", "italic"]) run[property] = target.textStyle[property];
        }
      }
    });
    toast("格式已应用");
    return;
  }
  if (!id) state.formatPaneOpen = false;
  syncCanvasSelection();
  renderInspector();
  state.documentHost?._emit?.("selection", state.documentHost.selection.get());
}

function syncCanvasSelection() {
  const stage = $("#slideStage");
  if (!stage) return;
  stage.querySelectorAll(".scene-object.selected").forEach((node) => node.classList.remove("selected"));
  stage.querySelectorAll(".selection-frame").forEach((selection) => selection.remove());
  if (!state.selectedId) return;
  const node = stage.querySelector(`.scene-object[data-id="${cssEscape(state.selectedId)}"]`);
  const object = findObject(state.selectedId);
  if (!node || !object) return;
  node.classList.add("selected");
  const selection = element("div", "selection-frame");
  selection.dataset.selectionFor = state.selectedId;
  selection.setAttribute("aria-label", `${displayObjectName(object)} 选择框`);
  selection.style.boxSizing = "border-box";
  applySelectionGeometry(selection, state.selectedId);
  for (const edge of ["n", "e", "s", "w"]) {
    const moveEdge = element("i", "selection-move-edge");
    moveEdge.dataset.edge = edge;
    moveEdge.title = "拖动移动对象";
    moveEdge.addEventListener("pointerdown", (event) => beginDrag(event, state.selectedId));
    selection.append(moveEdge);
  }
  handles.forEach((dir) => {
    const handle = element("i", "resize-handle");
    handle.dataset.dir = dir;
    handle.title = `拖动缩放（${dir.toUpperCase()}）`;
    handle.addEventListener("pointerdown", (event) => beginResize(event, state.selectedId, dir));
    selection.append(handle);
  });
  const stem = element("i", "rotation-stem");
  const rotateHandle = element("button", "rotate-handle material-symbols-rounded", "rotate_right");
  rotateHandle.type = "button";
  rotateHandle.title = "拖动旋转；按住 Shift 以 15° 吸附";
  rotateHandle.setAttribute("aria-label", rotateHandle.title);
  rotateHandle.addEventListener("pointerdown", (event) => beginRotate(event, state.selectedId));
  selection.append(stem, rotateHandle);
  stage.append(selection);
}

function selectionOverlayFor(id) {
  return $("#slideStage")?.querySelector(`.selection-frame[data-selection-for="${cssEscape(id)}"]`) || null;
}

function renderInspector() {
  scheduleMiniFormatToolbar();
  const object = selectedObject();
  const pane = $(".format-pane");
  if (!object) state.formatPaneOpen = false;
  pane.hidden = !object || !state.formatPaneOpen || !$("#animationPane").hidden;
  $("#emptyInspector").hidden = Boolean(object);
  $("#objectInspector").hidden = !object;
  syncTextRibbon(object);
  syncWorkspaceLayout();
  if (!object) return;
  $("#propName").value = displayObjectName(object);
  $("#propX").value = round(object.frame.x);
  $("#propY").value = round(object.frame.y);
  $("#propW").value = round(object.frame.width);
  $("#propH").value = round(object.frame.height);
  $("#propRotation").value = round(object.frame.rotation || 0);
  $("#propFill").value = normalizeColor(object.style.fill, "#ffffff");
  $("#propColor").value = normalizeColor(object.textStyle.color, "#172033");
  $("#propFill")._syncColorSwatch?.();
  $("#propColor")._syncColorSwatch?.();
  $("#propFontSize").value = round((object.textStyle.fontSize || 24) * 72 / 96);
  $("#propBold").classList.toggle("active", object.textStyle.bold);
  $("#propItalic").classList.toggle("active", object.textStyle.italic);
  $("#editFormula").hidden = object.kind !== "math";
}

function representativeTextRun(object) {
  const runs = (object?.textParagraphs || []).flatMap((paragraph) => paragraph.runs || []);
  return runs.find((run) => String(run.text || "").trim()) || runs[0] || null;
}

function syncTextRibbon(object) {
  const familySelect = $("#ribbonFontFamily");
  const sizeSelect = $("#ribbonFontSize");
  if (!familySelect || !sizeSelect) return;
  const run = representativeTextRun(object);
  const style = object?.textStyle || {};
  const family = run?.nativeFontFamily || style.nativeFontFamily
    || primaryCssFont(run?.fontFamily || style.fontFamily || "Aptos");
  const points = (Number(run?.fontSize || style.fontSize) || 24) * 72 / 96;
  ensureSelectOption(familySelect, family);
  familySelect.value = family;
  const sizeLabel = String(Math.round(points * 10) / 10).replace(/\.0$/, "");
  ensureSelectOption(sizeSelect, sizeLabel);
  sizeSelect.value = sizeLabel;
  const enabled = Boolean(object && (object.text || object.textParagraphs?.length || object.kind === "text"));
  familySelect.disabled = !enabled;
  sizeSelect.disabled = !enabled;
  const runs = (object?.textParagraphs || []).flatMap((paragraph) => paragraph.runs || []);
  const commandState = {
    ribbonBold: Boolean(style.bold || runs.some((item) => item.bold)),
    ribbonItalic: Boolean(style.italic || runs.some((item) => item.italic)),
    ribbonUnderline: runs.some((item) => item.underline),
    ribbonStrike: runs.some((item) => item.strikethrough),
    ribbonSubscript: runs.some((item) => item.baseline === "sub"),
    ribbonSuperscript: runs.some((item) => item.baseline === "super"),
  };
  const colorPreview = $("#ribbonFontColor span");
  if (colorPreview) colorPreview.style.background = normalizeColor(run?.color || style.color, "#172033");
  for (const [id, active] of Object.entries(commandState)) {
    const button = $(`#${id}`);
    button.disabled = !enabled;
    button.classList.toggle("active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  for (const id of ["growFont", "shrinkFont", "ribbonFontColor", "paragraphBullets", "paragraphNumbering", "paragraphOutdent", "paragraphIndent", "paragraphSpacing", "alignLeft", "alignCenter", "alignRight", "alignJustify", "verticalAlign"]) {
    $(`#${id}`).disabled = !enabled;
  }
  // The ribbon must echo the selected object paragraph alignment so the
  // active button always matches what the canvas renders.
  const paragraphAlign = (object?.textParagraphs || []).find((paragraph) => paragraph.align)?.align
    || style.align || "left";
  for (const [id, value] of [
    ["alignLeft", "left"], ["alignCenter", "center"], ["alignRight", "right"], ["alignJustify", "justify"],
  ]) {
    const button = $(`#${id}`);
    const active = enabled && paragraphAlign === value;
    button.classList.toggle("active", active);
    button.setAttribute("aria-pressed", String(active));
  }
  for (const id of ["copyObject", "cutObject", "formatPainter", "bringToFront", "shapeFill", "shapeOutline", "openFormatPane"]) {
    $(`#${id}`).disabled = !object;
  }
}

function ensureSelectOption(select, value) {
  if (!value) return;
  if (![...select.options].some((option) => option.value === value)) {
    const option = new Option(value, value);
    if (select.id === "ribbonFontFamily") {
      option.dataset.documentFont = "1";
      option.dataset.fontSource = "document";
    }
    select.add(option);
  }
}

function primaryCssFont(stack) {
  const first = String(stack || "Aptos").split(",")[0].trim();
  return first.replace(/^['"]|['"]$/g, "") || "Aptos";
}

function applyNativeFontFamily(object, family) {
  const value = String(family || "").trim();
  if (!value) return;
  const cssFamily = `${JSON.stringify(value)}, "Microsoft YaHei", sans-serif`;
  object.textStyle ||= defaultTextStyle();
  object.textStyle.nativeFontFamily = value;
  object.textStyle.fontFamily = cssFamily;
  for (const paragraph of object.textParagraphs || []) {
    for (const run of paragraph.runs || []) {
      run.nativeFontFamily = value;
      run.fontFamily = cssFamily;
    }
  }
}

function displayObjectName(object) {
  const raw = String(object?.name || "").trim();
  const suspicious = raw.match(/[\u00c0-\u024f\u1e00-\u1eff\ufffd]/g)?.length || 0;
  if (raw && !(suspicious >= 3 && suspicious / raw.length >= 0.2)) return raw;
  const label = ({
    group: "组合", text: "文本框", shape: "形状", image: "图片", connector: "连接符",
    table: "表格", chart: "图表", math: "公式", media: "媒体",
  })[object?.kind] || "对象";
  return object?.sourceShapeId == null ? label : `${label} ${object.sourceShapeId}`;
}

function closeFormatPane() {
  state.formatPaneOpen = false;
  $(".format-pane").hidden = true;
  syncWorkspaceLayout();
}

function openFormatPane(id = state.selectedId) {
  if (typeof id === "string" && id) selectObject(id);
  if (!selectedObject()) {
    toast("请先选择要设置格式的对象");
    return;
  }
  $("#animationPane").hidden = true;
  state.formatPaneOpen = true;
  renderInspector();
}

function syncWorkspaceLayout() {
  const formatOpen = !$(".format-pane").hidden;
  const animationOpen = !$("#animationPane").hidden;
  const workspace = $(".office-workspace");
  const animationLayoutChanged = workspace.classList.contains("has-animation-pane") !== animationOpen;
  workspace.classList.toggle("has-format-pane", formatOpen);
  workspace.classList.toggle("has-animation-pane", animationOpen);
  workspace.classList.toggle("has-side-pane", animationOpen);
  if (animationLayoutChanged && state.deck && state.autoFit) scheduleFitSlide();
}

const directionalTransitions = new Set(["push", "pull", "cover", "wipe", "strips"]);

function bindTransitionInputs() {
  $$('[data-transition]').forEach((button) => {
    button.onclick = () => setSlideTransition(button.dataset.transition);
  });
  $("#previewTransition").onclick = previewTransition;
  $("#transitionDirection").onchange = (event) => updateSlideTransition((transition) => {
    transition.direction = event.target.value || null;
  });
  bindTransitionNumberInput("#transitionDuration", (transition, value) => {
    transition.durationMs = Math.max(50, Math.round(number(value, .7) * 1000));
  });
  $("#transitionAdvanceClick").onchange = (event) => updateSlideTransition((transition) => {
    transition.advanceOnClick = event.target.checked;
  });
  $("#transitionAdvanceAfter").onchange = (event) => updateSlideTransition((transition) => {
    transition.advanceAfterMs = event.target.checked
      ? Math.max(0, Math.round(number($("#transitionAfterSeconds").value, 5) * 1000))
      : null;
  });
  bindTransitionNumberInput("#transitionAfterSeconds", (transition, value) => {
    transition.advanceAfterMs = Math.max(0, Math.round(number(value, 5) * 1000));
  });
  $("#applyTransitionAll").onclick = applyTransitionToAll;
}

function bindTransitionNumberInput(selector, operation) {
  const input = $(selector);
  input.onfocus = () => input.dataset.historyArmed = "0";
  input.oninput = (event) => {
    if (!state.deck || !currentSlide().transition) return;
    if (input.dataset.historyArmed !== "1") {
      checkpoint();
      state.future = [];
      input.dataset.historyArmed = "1";
    }
    operation(currentSlide().transition, event.target.value);
    markDeckChanged();
    updateHistoryButtons();
  };
  input.onchange = () => {
    input.dataset.historyArmed = "0";
    renderAll();
  };
}

function setSlideTransition(kind) {
  if (!state.deck) return;
  mutate(() => {
    if (kind === "none") {
      currentSlide().transition = null;
      return;
    }
    const previous = currentSlide().transition;
    currentSlide().transition = {
      kind,
      durationMs: previous?.durationMs || 700,
      advanceOnClick: previous?.advanceOnClick !== false,
      advanceAfterMs: previous?.advanceAfterMs ?? null,
      direction: directionalTransitions.has(kind) ? previous?.direction || "l" : null,
    };
  });
  if (kind !== "none") requestAnimationFrame(previewTransition);
}

function updateSlideTransition(operation) {
  if (!state.deck || !currentSlide().transition) return;
  mutate(() => operation(currentSlide().transition));
}

function syncTransitionRibbon() {
  if (!state.deck || !$("#transitionDuration")) return;
  const transition = currentSlide().transition;
  const kind = transition?.kind || "none";
  $$('[data-transition]').forEach((button) => button.classList.toggle("active", button.dataset.transition === kind));
  $("#transitionDuration").disabled = !transition;
  $("#transitionDirection").disabled = !transition || !directionalTransitions.has(kind);
  $("#transitionAdvanceClick").disabled = !transition;
  $("#transitionAdvanceAfter").disabled = !transition;
  $("#transitionAfterSeconds").disabled = !transition || transition.advanceAfterMs == null;
  $("#previewTransition").disabled = !transition;
  if (!transition) {
    $("#transitionDuration").value = "0.70";
    $("#transitionDirection").value = "";
    $("#transitionAdvanceClick").checked = true;
    $("#transitionAdvanceAfter").checked = false;
    $("#transitionAfterSeconds").value = "5.00";
    return;
  }
  $("#transitionDuration").value = (transition.durationMs / 1000).toFixed(2);
  $("#transitionDirection").value = directionalTransitions.has(kind) ? transition.direction || "l" : "";
  $("#transitionAdvanceClick").checked = transition.advanceOnClick !== false;
  $("#transitionAdvanceAfter").checked = transition.advanceAfterMs != null;
  $("#transitionAfterSeconds").value = ((transition.advanceAfterMs ?? 5000) / 1000).toFixed(2);
}

function previewTransition() {
  if (!state.deck) return;
  const transition = currentSlide().transition;
  if (!transition) return toast("当前幻灯片没有切换效果");
  renderCanvas();
  const player = animateSlideTransition($("#slideStage"), transition);
  if (player) player.onfinish = () => player.cancel();
  setStatus(`正在预览“${transitionLabel(transition.kind)}”切换效果`);
}

function applyTransitionToAll() {
  if (!state.deck) return;
  const transition = currentSlide().transition;
  mutate(() => {
    for (const slide of state.deck.slides) {
      slide.transition = transition ? JSON.parse(JSON.stringify(transition)) : null;
    }
  });
  toast(transition ? `已将“${transitionLabel(transition.kind)}”应用到全部幻灯片` : "已清除全部幻灯片切换效果");
}

function transitionLabel(kind) {
  return ({
    fade: "淡出", push: "推进", wipe: "擦除", split: "分割", cover: "覆盖",
    dissolve: "溶解", pull: "拉出", cut: "切出", ripple: "波纹", wind: "波浪",
    pageCurl: "翻页", pageCurlDouble: "双页翻书", peelOff: "剥离", origami: "折纸",
    curtains: "帘式", drape: "悬垂", fallOver: "翻倒", glitter: "闪耀",
  })[kind] || kind;
}

function bindAnimationInputs() {
  const fields = {
    animationEffect: (animation, value) => {
      animation.effect = value;
      animation.class = value === "spin" || value === "growShrink" ? "emphasis" : value === "motionPath" ? "motionPath" : value === "media" ? "media" : animation.class === "exit" ? "exit" : "entrance";
      animation.presetId = null;
      animation.presetSubtype = null;
      animation.direction = defaultAnimationDirection(value);
    },
    animationClass: (animation, value) => animation.class = value,
    animationTrigger: (animation, value) => animation.trigger = value,
    animationDuration: (animation, value) => animation.durationMs = Math.max(10, Math.round(number(value, .5) * 1000)),
    animationDelay: (animation, value) => animation.delayMs = Math.max(0, Math.round(number(value, 0) * 1000)),
    animationDirection: (animation, value) => animation.direction = value || null,
  };
  for (const [id, setter] of Object.entries(fields)) {
    $("#" + id).addEventListener("change", (event) => updateSelectedAnimation((animation) => setter(animation, event.target.value)));
  }
  $("#ribbonAnimationTrigger").onchange = (event) => updateSelectedAnimation((animation) => animation.trigger = event.target.value);
  $("#ribbonAnimationDuration").onchange = (event) => updateSelectedAnimation((animation) => animation.durationMs = Math.max(10, Math.round(number(event.target.value, .5) * 1000)));
  $("#ribbonAnimationDelay").onchange = (event) => updateSelectedAnimation((animation) => animation.delayMs = Math.max(0, Math.round(number(event.target.value, 0) * 1000)));
}

function openAnimationPane() {
  state.formatPaneOpen = false;
  $(".format-pane").hidden = true;
  $("#animationPane").hidden = false;
  syncWorkspaceLayout();
  renderAnimationPane();
}

function closeAnimationPane() {
  $("#animationPane").hidden = true;
  syncWorkspaceLayout();
}

function addAnimation(effect) {
  const object = selectedObject();
  if (!object) {
    toast("请先选择要添加动画的对象");
    openAnimationPane();
    return;
  }
  const animationClass = effect === "spin" || effect === "growShrink" ? "emphasis" : effect === "motionPath" ? "motionPath" : effect === "media" ? "media" : "entrance";
  mutate(() => {
    const animation = {
      id: uid("anim"), sourceTimingId: null,
      targetObjectId: object.id, targetShapeId: object.sourceShapeId ?? null,
      effect, class: animationClass, trigger: "onClick", durationMs: 500, delayMs: 0,
      order: currentSlide().animations.length, presetId: null, presetSubtype: null,
      direction: defaultAnimationDirection(effect),
      motionPath: effect === "motionPath" ? "M 0 0 L 0.15 0" : null,
    };
    currentSlide().animations.push(animation);
    state.selectedAnimationId = animation.id;
  });
  openAnimationPane();
}

function defaultAnimationDirection(effect) {
  return ({
    flyIn: "left",
    wipe: "left",
    randomBars: "horizontal",
    wheel: "1",
    circle: "in",
    split: "outHorizontal",
  })[effect] || null;
}

function currentAnimation() {
  return (currentSlide().animations || []).find((animation) => animation.id === state.selectedAnimationId) || null;
}

function updateSelectedAnimation(operation) {
  if (!currentAnimation()) return;
  mutate(() => operation(currentAnimation()));
}

function deleteSelectedAnimation() {
  const index = (currentSlide().animations || []).findIndex((animation) => animation.id === state.selectedAnimationId);
  if (index < 0) return;
  mutate(() => {
    currentSlide().animations.splice(index, 1);
    currentSlide().animations.forEach((animation, order) => animation.order = order);
    state.selectedAnimationId = currentSlide().animations[index]?.id || currentSlide().animations[index - 1]?.id || null;
  });
}

function moveAnimation(delta) {
  const animations = currentSlide().animations || [];
  const index = animations.findIndex((animation) => animation.id === state.selectedAnimationId);
  const target = index + delta;
  if (index < 0 || target < 0 || target >= animations.length) return;
  mutate(() => {
    [animations[index], animations[target]] = [animations[target], animations[index]];
    animations.forEach((animation, order) => animation.order = order);
  });
}

function renderAnimationPane() {
  if (!state.deck) return;
  const animations = [...(currentSlide().animations || [])].sort((a, b) => a.order - b.order);
  if (state.selectedAnimationId && !animations.some((animation) => animation.id === state.selectedAnimationId)) state.selectedAnimationId = null;
  const list = $("#animationList");
  list.replaceChildren();
  $("#animationEmpty").hidden = animations.length > 0;
  const schedule = animationSchedule(animations);
  const total = Math.max(1000, ...schedule.map((item) => item.end));
  $("#animationTotal").textContent = `${(Math.max(0, ...schedule.map((item) => item.end)) / 1000).toFixed(2)} 秒`;
  animations.forEach((animation, index) => {
    const timing = schedule[index];
    const item = element("div", `animation-item ${animation.class}${animation.id === state.selectedAnimationId ? " active" : ""}`);
    item.dataset.animationId = animation.id;
    const target = animation.targetObjectId ? findObject(animation.targetObjectId) : null;
    const indexNode = element("span", "animation-index", String(index + 1));
    const copy = element("span", "animation-copy", `${animationEffectLabel(animation.effect)} · ${target?.name || `形状 ${animation.targetShapeId || "?"}`}`);
    const trigger = element("span", "animation-trigger-icon", animation.trigger === "onClick" ? "🖱" : animation.trigger === "withPrevious" ? "◫" : "↳");
    const track = element("span", "animation-track");
    const bar = element("span", "");
    bar.style.left = `${timing.start / total * 100}%`;
    bar.style.width = `${Math.max(1.5, (timing.end - timing.start) / total * 100)}%`;
    track.append(bar);
    item.append(indexNode, copy, trigger, track);
    item.onclick = () => {
      state.selectedAnimationId = animation.id;
      if (animation.targetObjectId) state.selectedId = animation.targetObjectId;
      renderCanvas();
      renderInspector();
      renderAnimationPane();
      syncAnimationRibbon();
    };
    list.append(item);
  });
  const animation = currentAnimation();
  $("#animationInspector").hidden = !animation;
  if (animation) {
    $("#animationEffect").value = animation.effect;
    $("#animationClass").value = animation.class;
    $("#animationTrigger").value = animation.trigger;
    $("#animationDuration").value = (animation.durationMs / 1000).toFixed(2);
    $("#animationDelay").value = (animation.delayMs / 1000).toFixed(2);
    $("#animationDirection").value = animation.direction || "";
  }
}

function syncAnimationRibbon() {
  const animation = currentAnimation();
  $("#ribbonAnimationTrigger").disabled = !animation;
  $("#ribbonAnimationDuration").disabled = !animation;
  $("#ribbonAnimationDelay").disabled = !animation;
  $("#removeAnimation").disabled = !animation;
  if (!animation) return;
  $("#ribbonAnimationTrigger").value = animation.trigger;
  $("#ribbonAnimationDuration").value = (animation.durationMs / 1000).toFixed(2);
  $("#ribbonAnimationDelay").value = (animation.delayMs / 1000).toFixed(2);
}

function scheduleAnimationBatch(animations, startIndex = 0) {
  if (globalThis.UniPptPresentationFlow?.scheduleAnimationBatch) {
    return globalThis.UniPptPresentationFlow.scheduleAnimationBatch(animations, startIndex);
  }
  // OOXML timing delays inside one click batch are anchored to the current
  // timing group. `withPrevious` shares that group's start; an
  // `afterPrevious` effect starts a new group after the prior group ends.
  const entries = [];
  let cursor = startIndex;
  let groupStart = 0;
  let groupEnd = 0;
  let batchEnd = 0;
  while (cursor < animations.length) {
    const animation = animations[cursor];
    const trigger = animation.trigger || "onClick";
    if (cursor > startIndex && trigger === "onClick") break;
    const delay = Math.max(0, Number(animation.delayMs) || 0);
    let start;
    if (cursor === startIndex) {
      start = delay;
      groupStart = start;
    } else if (trigger === "withPrevious") {
      start = groupStart + delay;
    } else {
      start = groupEnd + delay;
      groupStart = start;
      groupEnd = start;
    }
    const end = start + Math.max(1, Number(animation.durationMs) || 500);
    entries.push({ animation, index: cursor, start, end });
    groupEnd = Math.max(groupEnd, end);
    batchEnd = Math.max(batchEnd, end);
    cursor++;
  }
  return { entries, nextIndex: cursor, end: batchEnd };
}

function navigationPositionsForEntries(entries) {
  if (globalThis.UniPptPresentationFlow?.navigationPositionsForEntries) {
    return globalThis.UniPptPresentationFlow.navigationPositionsForEntries(entries);
  }
  const positions = [];
  for (const entry of entries) {
    const previous = positions[positions.length - 1];
    const trigger = entry.animation.trigger || "onClick";
    const sharesStart = previous && Math.abs(entry.start - previous.start) < 0.5;
    if (previous && (trigger === "withPrevious" || sharesStart)) {
      previous.entries.push(entry);
      previous.endIndex = entry.index + 1;
      previous.end = Math.max(previous.end, entry.end);
      continue;
    }
    positions.push({
      startIndex: entry.index,
      endIndex: entry.index + 1,
      start: entry.start,
      end: entry.end,
      entries: [entry],
    });
  }
  return positions;
}

function scheduleAnimationNavigation(animations, startIndex = 0) {
  if (globalThis.UniPptPresentationFlow?.scheduleAnimationNavigation) {
    return globalThis.UniPptPresentationFlow.scheduleAnimationNavigation(animations, startIndex);
  }
  const batch = scheduleAnimationBatch(animations, startIndex);
  return { ...batch, positions: navigationPositionsForEntries(batch.entries) };
}

function animationNavigationRanges(animations) {
  if (globalThis.UniPptPresentationFlow?.animationNavigationRanges) {
    return globalThis.UniPptPresentationFlow.animationNavigationRanges(animations);
  }
  const ranges = [];
  let cursor = 0;
  while (cursor < animations.length) {
    const schedule = scheduleAnimationNavigation(animations, cursor);
    if (schedule.nextIndex <= cursor) break;
    for (const position of schedule.positions) {
      ranges.push({ startIndex: position.startIndex, endIndex: position.endIndex });
    }
    cursor = schedule.nextIndex;
  }
  return ranges;
}

function previousAnimationNavigationCursor(animations, cursor) {
  if (globalThis.UniPptPresentationFlow?.previousAnimationNavigationCursor) {
    return globalThis.UniPptPresentationFlow.previousAnimationNavigationCursor(animations, cursor);
  }
  const completed = Math.max(0, Math.min(animations.length, Math.trunc(Number(cursor) || 0)));
  if (completed === 0) return null;
  for (const range of animationNavigationRanges(animations)) {
    if (completed <= range.endIndex) return range.startIndex;
  }
  return 0;
}

function animationSchedule(animations, clickGap = 180) {
  if (globalThis.UniPptPresentationFlow?.animationSchedule) {
    return globalThis.UniPptPresentationFlow.animationSchedule(animations, clickGap);
  }
  const schedule = [];
  let cursor = 0;
  let batchOrigin = 0;
  while (cursor < animations.length) {
    const batch = scheduleAnimationBatch(animations, cursor);
    for (const entry of batch.entries) {
      schedule.push({
        ...entry,
        start: batchOrigin + entry.start,
        end: batchOrigin + entry.end,
      });
    }
    cursor = batch.nextIndex;
    batchOrigin += batch.end + (cursor < animations.length ? clickGap : 0);
  }
  return schedule;
}

function animationBatchRanges(animations) {
  if (globalThis.UniPptPresentationFlow?.animationBatchRanges) {
    return globalThis.UniPptPresentationFlow.animationBatchRanges(animations);
  }
  const ranges = [];
  let cursor = 0;
  while (cursor < animations.length) {
    const batch = scheduleAnimationBatch(animations, cursor);
    if (batch.nextIndex <= cursor) break;
    ranges.push({ startIndex: cursor, endIndex: batch.nextIndex });
    cursor = batch.nextIndex;
  }
  return ranges;
}

function previousAnimationBatchCursor(animations, cursor) {
  if (globalThis.UniPptPresentationFlow?.previousAnimationBatchCursor) {
    return globalThis.UniPptPresentationFlow.previousAnimationBatchCursor(animations, cursor);
  }
  const completed = Math.max(0, Math.min(animations.length, Math.trunc(Number(cursor) || 0)));
  if (completed === 0) return null;
  for (const range of animationBatchRanges(animations)) {
    if (completed <= range.endIndex) return range.startIndex;
  }
  return 0;
}

function animationEffectLabel(effect) {
  return ({ appear: "出现", fade: "淡化", flyIn: "飞入", wipe: "擦除", randomBars: "随机线条", dissolve: "溶解", wheel: "轮子", circle: "圆形", split: "劈裂", zoom: "缩放", spin: "陀螺旋", growShrink: "放大/缩小", motionPath: "动作路径", media: "媒体", custom: "自定义效果" })[effect] || effect;
}

function previewAnimations() {
  const animations = [...(currentSlide().animations || [])].sort((a, b) => a.order - b.order);
  if (!animations.length) return toast("当前幻灯片没有动画");
  state.animationPreviewTimers.forEach(clearTimeout);
  state.animationPreviewTimers = [];
  renderCanvas();
  const stage = $("#slideStage");
  resetAnimationTargets(stage, animations);
  const schedule = animationSchedule(animations);
  schedule.forEach((entry) => animateSceneEffect(stage, entry.animation, entry.start));
  setStatus(`正在预览 ${animations.length} 个动画效果`);
}

function resetAnimationTargets(stage, animations) {
  for (const animation of animations) {
    if (animation.class !== "entrance" || !animation.targetObjectId) continue;
    const node = stage.querySelector(`[data-id="${cssEscape(animation.targetObjectId)}"]`);
    if (node) { node.style.visibility = "hidden"; node.style.opacity = "0"; }
  }
}

function animateSceneEffect(stage, animation, offset = 0, timerStore = state.animationPreviewTimers, playerStore = null) {
  if (!animation.targetObjectId) return 0;
  const node = stage.querySelector(`[data-id="${cssEscape(animation.targetObjectId)}"]`);
  if (!node) return 0;
  const delay = Math.max(0, offset);
  const playbackTiming = globalThis.UniPptPresentationFlow?.animationPlaybackTiming?.(animation) || {
    duration: Math.max(1, animation.durationMs || 500),
    iterations: 1,
    direction: "normal",
    easing: "linear",
    fill: "forwards",
  };
  const activeDuration = globalThis.UniPptPresentationFlow?.animationActiveDuration?.(animation)
    || playbackTiming.duration;
  const run = () => {
    node.style.visibility = "visible";
    if (animation.effect === "media" || animation.class === "media") {
      globalThis.UniPptMedia?.controlNode(node, animation.mediaAction || "play");
      return;
    }
    const frames = presentationAnimationFrames(animation, node);
    const previousWillChange = node.style.willChange;
    const compositorProperties = new Set();
    for (const frame of frames) {
      if (frame.transform != null) compositorProperties.add("transform");
      if (frame.opacity != null) compositorProperties.add("opacity");
      if (frame.clipPath != null) compositorProperties.add("clip-path");
      if (frame.filter != null) compositorProperties.add("filter");
      if (frame.maskImage != null || frame.webkitMaskImage != null) compositorProperties.add("mask");
    }
    if (compositorProperties.size) node.style.willChange = [...compositorProperties].join(",");
    const player = node.animate(frames, playbackTiming);
    if (playerStore) playerStore.push(player);
    const restoreCompositorHint = () => { node.style.willChange = previousWillChange; };
    player.onfinish = () => {
      restoreCompositorHint();
      if (animation.class === "exit" && !animation.autoReverse && !(Number(animation.speed) < 0)) { node.style.visibility = "hidden"; node.style.opacity = "0"; }
      else node.style.opacity = "1";
    };
    player.oncancel = restoreCompositorHint;
  };
  if (delay === 0 && (animation.effect === "media" || animation.class === "media")) {
    // Keep on-click media inside the browser's user-activation stack.
    run();
    return activeDuration;
  }
  const timer = setTimeout(run, delay);
  timerStore.push(timer);
  return delay + activeDuration;
}

function presentationAnimationFrames(animation, node) {
  const frames = animationFrames(animation, node);
  const quality = presentationQualityTier("native-animation");
  if (quality === "high" || frames.length <= 2) return frames;
  const maximumFrames = quality === "low" ? 4 : 8;
  const indexes = new Set([0, frames.length - 1]);
  for (let index = 1; index < maximumFrames - 1; index += 1) {
    indexes.add(Math.round(index * (frames.length - 1) / (maximumFrames - 1)));
  }
  // A low frame budget may make the native mask advance in larger steps, but
  // it must never silently turn Random Bars/Wheel/Dissolve into a generic
  // fade.  That semantic downgrade was the main reason editor and exported
  // HTML effects looked unrelated on slower machines.
  return [...indexes].sort((a, b) => a - b).map((index) => ({ ...frames[index] }));
}

function animationFrames(animation, node) {
  const base = node.dataset.animationBase || node.style.transform || "";
  const exit = animation.class === "exit";
  let frames;
  const presetFrames = globalThis.UniPptPresetAnimation?.frames(animation, {
    node,
    slideWidth: state.deck?.width,
    slideHeight: state.deck?.height,
    baseTransform: base,
  });
  if (presetFrames) {
    frames = presetFrames;
  } else if (animation.effect === "flyIn") {
    const direction = animation.direction || "left";
    const x = direction === "left" ? "-22%" : direction === "right" ? "22%" : "0";
    const y = direction === "up" ? "-22%" : direction === "down" ? "22%" : "0";
    frames = [{ opacity: 0, transform: `${base} translate(${x},${y})` }, { opacity: 1, transform: `${base} translate(0,0)` }];
  } else if (animation.effect === "wipe") {
    const direction = animation.direction || "left";
    const hidden = direction === "right" ? "inset(0 100% 0 0)" : direction === "up" ? "inset(100% 0 0 0)" : direction === "down" ? "inset(0 0 100% 0)" : "inset(0 0 0 100%)";
    frames = [{ clipPath: hidden, opacity: 1 }, { clipPath: "inset(0 0 0 0)", opacity: 1 }];
  } else if (animation.effect === "zoom") {
    frames = [{ opacity: 0, transform: `${base} scale(.25)` }, { opacity: 1, transform: `${base} scale(1)` }];
  } else if (animation.effect === "spin") {
    frames = [{ transform: `${base} rotate(0deg)` }, { transform: `${base} rotate(360deg)` }];
  } else if (animation.effect === "growShrink") {
    frames = [{ transform: `${base} scale(1)` }, { transform: `${base} scale(1.28)` }, { transform: `${base} scale(1)` }];
  } else if (animation.effect === "motionPath") {
    frames = globalThis.UniPptMotionPath?.frames(
      animation.motionPath,
      state.deck?.width,
      state.deck?.height,
      base,
    ) || [{ transform: `${base} translate(0,0)` }, { transform: `${base} translate(15%,0)` }];
  } else {
    frames = [{ opacity: 0 }, { opacity: 1 }];
  }
  // Native p:anim keyframes already encode their own entrance/exit direction;
  // reversing them a second time makes PowerPoint exits run backwards.
  return exit && !animation.propertyAnimations?.length ? frames.reverse() : frames;
}

function updateSelected(operation) {
  const object = selectedObject();
  if (object) mutate(() => operation(object));
}

function toggleSelectedStyle(property) {
  if (applySelectedTextProperty(property, (runs) => !runs.every((run) => Boolean(run[property])))) return;
  updateSelected((object) => applyTextStyle(object, property, !object.textStyle[property]));
}

function toggleSelectedRunStyle(property) {
  const object = selectedObject();
  if (object && applySelectedTextProperty(property, (runs) => !runs.every((run) => Boolean(run[property])))) return;
  if (!object) return toast("请先选择文本对象");
  const runs = ensureRichTextParagraphs(object).flatMap((paragraph) => paragraph.runs || []);
  const next = !runs.some((run) => Boolean(run[property]));
  updateSelected((target) => {
    for (const paragraph of ensureRichTextParagraphs(target)) {
      for (const run of paragraph.runs || []) run[property] = next;
    }
  });
}

function toggleSelectedBaseline(value) {
  const object = selectedObject();
  if (object && applySelectedTextProperty("baseline", (runs) => runs.every((run) => run.baseline === value) ? "normal" : value)) return;
  if (!object) return toast("请先选择文本对象");
  const runs = ensureRichTextParagraphs(object).flatMap((paragraph) => paragraph.runs || []);
  const next = runs.some((run) => run.baseline === value) ? "normal" : value;
  updateSelected((target) => {
    for (const paragraph of ensureRichTextParagraphs(target)) {
      for (const run of paragraph.runs || []) run.baseline = next;
    }
  });
}

function adjustSelectedFontSize(points) {
  if (applySelectedTextProperty("fontSize", (_runs, object) => (run) => {
    const current = (Number(run.fontSize) || Number(object.textStyle?.fontSize) || 24) * 72 / 96;
    return clamp(current + points, 1, 400) * 96 / 72;
  })) return;
  updateSelected((object) => {
    const current = (Number(object.textStyle?.fontSize) || 24) * 72 / 96;
    applyTextStyle(object, "fontSize", clamp(current + points, 1, 400) * 96 / 72);
  });
}

function ensureRichTextParagraphs(object) {
  object.textParagraphs ||= [];
  if (!object.textParagraphs.length) {
    const lines = normalizeEditedText(object.text || "").split("\n");
    object.textParagraphs = lines.map((line) => createRichTextParagraph(null, line, object.textStyle || defaultTextStyle()));
  }
  return object.textParagraphs;
}

function applyParagraphChange(operation) {
  if (!selectedObject()) return toast("请先选择文本对象");
  updateSelected((object) => {
    for (const [index, paragraph] of ensureRichTextParagraphs(object).entries()) operation(paragraph, index, object);
  });
}

function toggleParagraphList(kind) {
  const object = selectedObject();
  if (!object) return toast("请先选择文本对象");
  const paragraphs = ensureRichTextParagraphs(object);
  const active = paragraphs.every((item) => kind === "bullet" ? item.bullet === "•" : /^\d+\.$/.test(item.bullet || ""));
  updateSelected((target) => {
    ensureRichTextParagraphs(target).forEach((paragraph, index) => {
      paragraph.bullet = active ? null : kind === "bullet" ? "•" : `${index + 1}.`;
    });
  });
}

function adjustParagraphLevel(delta) {
  applyParagraphChange((paragraph) => { paragraph.level = clamp((Number(paragraph.level) || 0) + delta, 0, 8); });
}

function cycleParagraphSpacing() {
  const object = selectedObject();
  if (!object) return toast("请先选择文本对象");
  const current = Number(ensureRichTextParagraphs(object)[0]?.lineSpacing) || 1.18;
  const choices = [1, 1.18, 1.5, 2];
  const next = choices[(choices.findIndex((value) => Math.abs(value - current) < .02) + 1) % choices.length];
  applyParagraphChange((paragraph) => { paragraph.lineSpacing = next; });
  toast(`行距：${next}`);
}

function setParagraphAlignment(align) {
  updateSelected((object) => {
    object.textStyle ||= defaultTextStyle();
    object.textStyle.align = align;
    for (const paragraph of ensureRichTextParagraphs(object)) paragraph.align = align;
  });
}

function cycleVerticalAlignment() {
  const object = selectedObject();
  if (!object) return toast("请先选择文本对象");
  const choices = ["top", "center", "bottom"];
  const next = choices[(choices.indexOf(object.textFrame?.verticalAlign || "center") + 1) % choices.length];
  updateSelected((target) => { target.textFrame ||= {}; target.textFrame.verticalAlign = next; });
  toast(`垂直对齐：${({ top: "顶端", center: "中部", bottom: "底端" })[next]}`);
}

function applyTextStyle(object, property, value) {
  object.textStyle ||= defaultTextStyle();
  object.textStyle[property] = value;
  for (const paragraph of object.textParagraphs || []) {
    for (const run of paragraph.runs || []) {
      run[property] = value;
      if (property === "color") run.gradient = null;
    }
  }
}

function addSlide() {
  mutate(() => {
    const index = state.deck.slides.length + 1;
    const slide = {
      id: uid("slide"), sourcePartName: null, name: `幻灯片 ${index}`, background: "#ffffff", backgroundAsset: null, notes: "",
      masterObjects: [], layoutObjects: [], inheritedAnimations: [],
      animations: [], transition: null, sourceTimingXml: null, sourceTransitionXml: null,
      objects: [
        newSceneObject("text", "标题占位符", { x: 90, y: 75, width: 1100, height: 105 }, "单击此处添加标题", 44),
        newSceneObject("text", "内容占位符", { x: 105, y: 220, width: 1070, height: 350 }, "单击此处添加文本", 26),
      ],
    };
    state.deck.slides.splice(state.activeSlide + 1, 0, slide);
    state.activeSlide++;
    state.selectedId = null;
  });
}

function duplicateSlide() {
  mutate(() => {
    const clone = structuredClone(currentSlide());
    const idMap = new Map();
    clone.id = uid("slide");
    clone.sourcePartName = null;
    clone.sourceTimingXml = null;
    clone.sourceTransitionXml = null;
    clone.name += " 副本";
    renewIds(clone.objects, idMap);
    clone.animations = (clone.animations || []).map((animation, order) => ({
      ...animation,
      id: uid("anim"),
      sourceTimingId: null,
      targetShapeId: null,
      targetObjectId: idMap.get(animation.targetObjectId) || animation.targetObjectId,
      order,
    }));
    state.deck.slides.splice(state.activeSlide + 1, 0, clone);
    state.activeSlide++;
    state.selectedId = null;
  });
}

function moveSlide(delta) {
  const target = state.activeSlide + delta;
  if (target < 0 || target >= state.deck.slides.length) return;
  mutate(() => {
    const [slide] = state.deck.slides.splice(state.activeSlide, 1);
    state.deck.slides.splice(target, 0, slide);
    state.activeSlide = target;
  });
}

function deleteSlide() {
  if (state.deck.slides.length === 1) return toast("演示文稿至少需要一张幻灯片");
  mutate(() => {
    state.deck.slides.splice(state.activeSlide, 1);
    state.activeSlide = Math.min(state.activeSlide, state.deck.slides.length - 1);
    state.selectedId = null;
  });
}

function addText() {
  addObject(newSceneObject("text", "文本框", { x: 160, y: 160, width: 460, height: 96 }, "双击编辑文本", 28));
}

function addShape() {
  const object = newSceneObject("shape", "圆角矩形", { x: 210, y: 210, width: 360, height: 170 }, "形状", 28);
  object.geometry = "RoundRect";
  object.style = { fill: "#c43e1c", stroke: "#9f2f13", strokeWidth: 1, opacity: 1 };
  Object.assign(object.textStyle, { color: "#ffffff", bold: true, align: "center" });
  addObject(object);
}

async function addImageFile(file) {
  if (!file) return;
  $("#imageInput").value = "";
  const isEmf = /\.emf$/i.test(file.name) || /(?:x-)?emf/i.test(file.type || "");
  if (isEmf) {
    try {
      setStatus("正在把 EMF 转换为可编辑的 SVG 投影…");
      const response = await fetch("/api/vector/emf-to-svg", {
        method: "POST",
        headers: { "Content-Type": file.type || "image/x-emf" },
        body: file,
      });
      if (!response.ok) throw new Error((await response.json().catch(() => null))?.error || `HTTP ${response.status}`);
      const svg = await response.text();
      const object = newSceneObject("image", file.name, { x: 180, y: 130, width: 480, height: 320 }, "", 24);
      object.asset = `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(svg)))}`;
      addObject(object);
      setStatus("EMF 已作为 SVG 投影插入；原始 EMF 字节随投影无损绑定并可原样回写");
    } catch (error) {
      toast(`EMF 插入失败：${error.message}`);
    }
    return;
  }
  const reader = new FileReader();
  reader.onload = () => {
    const object = newSceneObject("image", file.name, { x: 180, y: 130, width: 480, height: 320 }, "", 24);
    object.asset = reader.result;
    addObject(object);
    if (/\.svg$/i.test(file.name) || file.type === "image/svg+xml") {
      setStatus("SVG 已插入；保存时将生成携带原始 SVG 的无损 EMF，并保留浏览器 SVG 投影");
    }
  };
  reader.readAsDataURL(file);
}

async function captureScreen() {
  if (!navigator.mediaDevices?.getDisplayMedia) {
    toast("当前浏览器不支持屏幕截图");
    return;
  }
  let stream;
  try {
    stream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false });
    const video = document.createElement("video");
    video.srcObject = stream;
    video.muted = true;
    await video.play();
    await new Promise((resolve) => requestAnimationFrame(resolve));
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, video.videoWidth);
    canvas.height = Math.max(1, video.videoHeight);
    canvas.getContext("2d").drawImage(video, 0, 0);
    const object = newSceneObject("image", "屏幕截图", fitInsertedMediaFrame(canvas.width, canvas.height), "", 24);
    object.asset = canvas.toDataURL("image/png");
    addObject(object);
    setStatus("屏幕截图已作为可编辑图片插入");
  } catch (error) {
    if (error?.name !== "NotAllowedError") toast(`屏幕截图失败：${error.message}`);
  } finally {
    stream?.getTracks().forEach((track) => track.stop());
  }
}

function fitInsertedMediaFrame(width, height) {
  const maxWidth = 720;
  const maxHeight = 430;
  const ratio = Math.min(maxWidth / Math.max(1, width), maxHeight / Math.max(1, height), 1);
  const fittedWidth = Math.max(160, width * ratio);
  const fittedHeight = Math.max(90, height * ratio);
  return {
    x: Math.max(40, (state.deck.width - fittedWidth) / 2),
    y: Math.max(40, (state.deck.height - fittedHeight) / 2),
    width: fittedWidth,
    height: fittedHeight,
  };
}

function addMediaFile(file, kind) {
  if (!file) return;
  const input = kind === "video" ? $("#videoInput") : $("#audioInput");
  input.value = "";
  const reader = new FileReader();
  reader.onload = () => {
    const isVideo = kind === "video";
    const object = newSceneObject("image", file.name, isVideo
      ? { x: 210, y: 135, width: 680, height: 382 }
      : { x: 430, y: 275, width: 190, height: 190 }, "", 24);
    object.asset = mediaPosterDataUrl(kind);
    object.media = {
      kind,
      asset: reader.result,
      mimeType: file.type || (isVideo ? "video/mp4" : "audio/mpeg"),
      playbackAsset: reader.result,
      playbackMimeType: file.type || (isVideo ? "video/mp4" : "audio/mpeg"),
      sourcePartName: null,
      relationshipId: null,
      legacyRelationshipId: null,
      trimStartMs: null,
      trimEndMs: null,
      volume: 1,
      loopPlayback: false,
      playAcrossSlides: false,
      showWhenStopped: true,
    };
    addObject(object);
    setStatus(`${isVideo ? "视频" : "音频"}已嵌入；UDoc 将按内容哈希只保存一份资源`);
  };
  reader.readAsDataURL(file);
}

function mediaPosterDataUrl(kind) {
  const video = kind === "video";
  const label = video ? "VIDEO" : "AUDIO";
  const glyph = video ? "▶" : "♫";
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360" viewBox="0 0 640 360"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#1d2738"/><stop offset="1" stop-color="#334b68"/></linearGradient></defs><rect width="640" height="360" rx="26" fill="url(#g)"/><circle cx="320" cy="160" r="68" fill="#fff" fill-opacity=".94"/><text x="320" y="190" text-anchor="middle" font-size="78" font-family="Segoe UI Symbol,Arial" fill="#c43e1c">${glyph}</text><text x="320" y="285" text-anchor="middle" font-size="30" font-family="Aptos,Arial" letter-spacing="8" fill="#fff">${label}</text></svg>`;
  return `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(svg)))}`;
}

function openInsertObjectDialog(kind, objectId = null) {
  state.insertingObjectKind = kind;
  state.editingInsertObjectId = objectId;
  const title = $("#insertObjectTitle");
  const subtitle = $("#insertObjectSubtitle");
  const body = $("#insertObjectBody");
  const commit = $("#commitInsertObject");
  body.replaceChildren();
  const definitions = {
    table: ["插入表格", "生成可编辑 DrawingML 表格", tableInsertEditor],
    chart: ["插入图表", "图表数据将写入原生 chart XML 与嵌入工作簿", chartInsertEditor],
    smartArt: ["插入 SmartArt", "生成可独立编辑、可原生回写的形状与连线", smartArtInsertEditor],
    symbol: ["插入符号", "以文本对象保存，可继续编辑字体与字号", symbolInsertEditor],
    dynamic: ["插入动态内容", "HTML/SVG 在 UDoc 与无损 HTML 中保持交互；PPTX 保存可见投影和便携源", dynamicInsertEditor],
  };
  const [heading, detail, builder] = definitions[kind];
  title.textContent = heading;
  subtitle.textContent = detail;
  commit.textContent = heading;
  body.append(builder());
  if (kind === "chart" && objectId) populateChartInsertEditor(findObject(objectId)?.chart);
  commit.textContent = objectId ? "更新" : heading;
  $("#insertObjectDialog").showModal();
  body.querySelector("input,textarea,select,button")?.focus();
}

function closeInsertObjectDialog() {
  state.insertingObjectKind = null;
  state.editingInsertObjectId = null;
  if ($("#insertObjectDialog").open) $("#insertObjectDialog").close();
}

function labeledField(label, control) {
  const wrapper = element("label", "insert-field");
  wrapper.append(element("span", "", label), control);
  return wrapper;
}

function tableInsertEditor() {
  const grid = element("div", "insert-form-grid");
  const rows = document.createElement("input");
  rows.id = "insertTableRows"; rows.type = "number"; rows.min = "1"; rows.max = "20"; rows.value = "3";
  const cols = document.createElement("input");
  cols.id = "insertTableColumns"; cols.type = "number"; cols.min = "1"; cols.max = "12"; cols.value = "4";
  grid.append(labeledField("行数", rows), labeledField("列数", cols));
  return grid;
}

function chartInsertEditor() {
  const grid = element("div", "insert-form-grid chart-insert-grid");
  const type = document.createElement("select");
  type.id = "insertChartType";
  for (const [value, label] of [["bar", "柱形图"], ["line", "折线图"], ["pie", "饼图"], ["doughnut", "圆环图"], ["area", "面积图"]]) {
    const option = document.createElement("option"); option.value = value; option.textContent = label; type.append(option);
  }
  const title = document.createElement("input"); title.id = "insertChartTitle"; title.value = "季度数据";
  const categories = document.createElement("input"); categories.id = "insertChartCategories"; categories.value = "第一季度,第二季度,第三季度,第四季度";
  const series = document.createElement("textarea"); series.id = "insertChartSeries"; series.rows = 4; series.value = "系列 1: 32, 48, 41, 66\n系列 2: 22, 35, 52, 58";
  grid.append(labeledField("图表类型", type), labeledField("标题", title), labeledField("分类（逗号分隔）", categories), labeledField("系列（每行：名称: 数值）", series));
  return grid;
}

function populateChartInsertEditor(chart) {
  if (!chart) return;
  $("#insertChartType").value = chart.chartType || "bar";
  $("#insertChartTitle").value = chart.title || "";
  $("#insertChartCategories").value = (chart.categories || []).join(", ");
  $("#insertChartSeries").value = (chart.series || []).map((series) => `${series.name}: ${(series.values || []).map((value) => value ?? "").join(", ")}`).join("\n");
}

function smartArtInsertEditor() {
  const grid = element("div", "insert-form-grid");
  const layout = document.createElement("select"); layout.id = "insertSmartArtLayout";
  for (const [value, label] of [["process", "基本流程"], ["cycle", "循环"], ["hierarchy", "层次结构"]]) {
    const option = document.createElement("option"); option.value = value; option.textContent = label; layout.append(option);
  }
  const labels = document.createElement("textarea"); labels.id = "insertSmartArtLabels"; labels.rows = 5; labels.value = "规划\n设计\n实施\n复盘";
  grid.append(labeledField("布局", layout), labeledField("节点（每行一个）", labels));
  return grid;
}

function symbolInsertEditor() {
  const picker = element("div", "symbol-picker");
  const input = document.createElement("input"); input.id = "insertSymbolValue"; input.value = "Ω"; input.maxLength = 8;
  const symbols = ["Ω", "π", "±", "×", "÷", "≤", "≥", "≠", "∞", "∑", "∫", "√", "→", "←", "★", "✓", "©", "®"];
  const gallery = element("div", "symbol-gallery");
  symbols.forEach((symbol) => {
    const button = element("button", "", symbol); button.type = "button";
    button.onclick = () => { input.value = symbol; input.focus(); };
    gallery.append(button);
  });
  picker.append(labeledField("符号", input), gallery);
  return picker;
}

function dynamicInsertEditor() {
  const wrapper = element("div", "dynamic-insert-editor");
  wrapper.innerHTML = `
    <label>内容类型<select id="insertDynamicKind"><option value="html">HTML 网页/组件</option><option value="svg">SVG 矢量组件</option></select></label>
    <label>名称<input id="insertDynamicName" value="动态内容" maxlength="80"></label>
    <label class="dynamic-source-field">源码<textarea id="insertDynamicSource" spellcheck="false" placeholder="粘贴完整 HTML，或可独立显示的 SVG 源码"></textarea></label>
    <label class="dynamic-sandbox-field"><input id="insertDynamicScripts" type="checkbox" checked> 允许沙箱内脚本运行（不授予同源权限）</label>
    <p>动态源码属于 AI 友好的文档扩展。导出 PPTX 时 PowerPoint 显示静态占位投影；源码随 customXml 携带，重新导入 UniPPT 后恢复交互。</p>`;
  return wrapper;
}

function commitInsertObject(event) {
  event.preventDefault();
  const kind = state.insertingObjectKind;
  if (kind === "table") insertTableObject();
  else if (kind === "chart") insertChartObject();
  else if (kind === "smartArt") insertSmartArtObjects();
  else if (kind === "symbol") insertSymbolObject();
  else if (kind === "dynamic") insertDynamicObject();
  closeInsertObjectDialog();
}

function insertTableObject() {
  const rowCount = clamp(Math.round(Number($("#insertTableRows").value) || 3), 1, 20);
  const columnCount = clamp(Math.round(Number($("#insertTableColumns").value) || 4), 1, 12);
  const frame = { x: 150, y: 150, width: 920, height: Math.min(440, Math.max(150, rowCount * 56)) };
  const object = newSceneObject("table", `表格 ${rowCount}×${columnCount}`, frame, "", 18);
  const border = { color: "#b7c1cf", width: 1, dash: null };
  object.table = {
    columns: Array.from({ length: columnCount }, () => frame.width / columnCount),
    rows: Array.from({ length: rowCount }, (_, rowIndex) => ({
      height: frame.height / rowCount,
      cells: Array.from({ length: columnCount }, (_, columnIndex) => ({
        text: rowIndex === 0 ? `标题 ${columnIndex + 1}` : "",
        textParagraphs: [], textFrame: { marginLeft: 8, marginRight: 8, marginTop: 5, marginBottom: 5, verticalAlign: "center", verticalType: "horz", wordWrap: true, autoSize: "none" },
        textStyle: { ...defaultTextStyle(), fontSize: 18, bold: rowIndex === 0 },
        fill: rowIndex === 0 ? "#dbe5f1" : rowIndex % 2 ? "#f6f8fb" : "#ffffff",
        borders: { left: border, right: border, top: border, bottom: border },
        gridSpan: 1, rowSpan: 1, hMerge: false, vMerge: false,
      })),
    })),
    firstRow: true, firstCol: false, lastRow: false, lastCol: false, bandRows: true, bandCols: false,
    styleId: "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}",
  };
  addObject(object);
}

function insertChartObject() {
  const chartType = $("#insertChartType").value;
  const categories = $("#insertChartCategories").value.split(/[,，]/).map((value) => value.trim()).filter(Boolean);
  const palette = ["#4472C4", "#ED7D31", "#A5A5A5", "#FFC000", "#5B9BD5", "#70AD47"];
  const series = $("#insertChartSeries").value.split(/\r?\n/).map((line, index) => {
    const [name, raw = ""] = line.split(/[:：]/, 2);
    return { sourceIndex: null, name: name.trim() || `系列 ${index + 1}`, values: raw.split(/[,，]/).map((value) => Number(value.trim())).map((value) => Number.isFinite(value) ? value : null), color: palette[index % palette.length], pointColors: [] };
  }).filter((item) => item.values.length);
  if (!categories.length || !series.length) return toast("请填写分类和至少一个数据系列");
  const chart = { relationshipId: null, sourcePartName: null, chartType, title: $("#insertChartTitle").value.trim(), legend: { visible: true, position: "right", overlay: false }, categories, series, barDirection: chartType === "bar" ? "column" : null, grouping: "clustered", holeSize: chartType === "doughnut" ? 0.5 : null };
  const existing = state.editingInsertObjectId ? findObject(state.editingInsertObjectId) : null;
  if (existing?.kind === "chart") {
    mutate(() => { existing.chart = chart; });
  } else {
    const object = newSceneObject("chart", "图表", { x: 180, y: 120, width: 900, height: 470 }, "", 20);
    object.chart = chart;
    addObject(object);
  }
}

function insertSmartArtObjects() {
  const labels = $("#insertSmartArtLabels").value.split(/\r?\n/).map((value) => value.trim()).filter(Boolean).slice(0, 8);
  if (!labels.length) return toast("请至少输入一个节点");
  const layout = $("#insertSmartArtLayout").value;
  const objects = buildSmartArtObjects(layout, labels);
  addObjects(objects);
}

function buildSmartArtObjects(layout, labels) {
  const palette = ["#4472C4", "#5B9BD5", "#70AD47", "#ED7D31", "#A5A5A5", "#FFC000"];
  const shapes = [];
  const count = labels.length;
  labels.forEach((label, index) => {
    let frame;
    if (layout === "cycle") {
      const angle = -Math.PI / 2 + index * Math.PI * 2 / count;
      frame = { x: 550 + Math.cos(angle) * 300 - 90, y: 340 + Math.sin(angle) * 220 - 55, width: 180, height: 110 };
    } else if (layout === "hierarchy") {
      const top = index === 0; const children = Math.max(1, count - 1);
      frame = top ? { x: 510, y: 120, width: 260, height: 100 } : { x: 80 + (index - 1) * (1120 / children), y: 390, width: Math.min(230, 980 / children), height: 110 };
    } else {
      const width = Math.min(250, 1040 / count);
      frame = { x: 90 + index * (1100 / count), y: 285, width, height: 125 };
    }
    const object = newSceneObject("shape", `SmartArt ${index + 1}`, frame, label, 24);
    object.geometry = layout === "cycle" ? "Ellipse" : "RoundRect";
    object.style = { ...defaultStyle(), fill: palette[index % palette.length], stroke: "#ffffff", strokeWidth: 2, opacity: 1 };
    Object.assign(object.textStyle, { color: "#ffffff", bold: true, align: "center" });
    object.smartArt = { layout, nodeIndex: index, nodeCount: count };
    shapes.push(object);
  });
  return shapes;
}

function insertSymbolObject() {
  const value = $("#insertSymbolValue").value.trim();
  if (!value) return toast("请选择或输入符号");
  const object = newSceneObject("text", "符号", { x: 500, y: 250, width: 280, height: 180 }, value, 88);
  object.textStyle.align = "center";
  object.textFrame.verticalAlign = "center";
  addObject(object);
}

function insertDynamicObject() {
  const source = $("#insertDynamicSource").value.trim();
  if (!source) return toast("请输入 HTML 或 SVG 源码");
  const kind = $("#insertDynamicKind").value === "svg" ? "svg" : "html";
  const name = $("#insertDynamicName").value.trim() || (kind === "svg" ? "动态 SVG" : "动态 HTML");
  const object = newSceneObject("ole", `UniPPT Dynamic · ${name} · ${uid("embed")}`, { x: 190, y: 115, width: 900, height: 490 }, `动态 ${kind.toUpperCase()}\n在 UniPPT 或无损 HTML 中查看交互内容`, 24);
  object.geometry = "Rect";
  object.style = { ...defaultStyle(), fill: "#f8fafc", stroke: "#7890aa", strokeWidth: 1, opacity: 1 };
  Object.assign(object.textStyle, { color: "#526078", align: "center" });
  mutate(() => {
    currentSlide().objects.push(object);
    const extension = state.deck.extensions ||= {};
    const dynamic = extension["org.unippt.dynamic"] ||= { version: 1, objects: {} };
    dynamic.objects ||= {};
    dynamic.objects[object.id] = {
      objectName: object.name,
      kind,
      source,
      allowScripts: Boolean($("#insertDynamicScripts").checked),
      createdAt: new Date().toISOString(),
    };
    state.selectedId = object.id;
  });
  setStatus(`${name} 已作为便携动态内容插入`);
}

function dynamicContentForObject(object) {
  const objects = state.deck?.extensions?.["org.unippt.dynamic"]?.objects || {};
  return objects[object?.id] || Object.values(objects).find((entry) => entry?.objectName === object?.name) || null;
}

function renderDynamicObject(node, object, interactive, thumbnail) {
  const dynamic = dynamicContentForObject(object);
  if (!dynamic) return false;
  node.classList.add("dynamic-content-object");
  if (thumbnail) {
    node.append(element("div", "dynamic-content-placeholder", dynamic.kind === "svg" ? "SVG" : "HTML"));
    return true;
  }
  const frame = document.createElement("iframe");
  frame.className = "dynamic-content-frame";
  frame.title = object.name || "动态内容";
  frame.setAttribute("sandbox", dynamic.allowScripts ? "allow-scripts allow-forms allow-popups" : "");
  frame.srcdoc = dynamic.kind === "svg"
    ? `<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;width:100%;height:100%;overflow:hidden}svg{display:block;width:100%;height:100%}</style></head><body>${dynamic.source}</body></html>`
    : dynamic.source;
  if (interactive) frame.style.pointerEvents = "none";
  node.replaceChildren(frame);
  return true;
}

function newSceneObject(kind, name, frame, text, fontSize) {
  return {
    id: uid("shape"), sourceShapeId: null, name, kind,
    frame: { ...frame, rotation: frame.rotation || 0 }, flipH: false, flipV: false,
    text, textParagraphs: [], textFrame: { marginLeft: 8, marginRight: 8, marginTop: 5, marginBottom: 5, verticalAlign: "center", verticalType: "horz", wordWrap: true, autoSize: "none" },
    geometry: null, customGeometry: null, asset: null, imageCrop: { left: 0, top: 0, right: 0, bottom: 0 }, imageFillRect: { left: 0, top: 0, right: 0, bottom: 0 }, imageEffects: { duotone: null, softEdgeRadius: 0 }, shapeFillAsset: null, formula: null, media: null, table: null, chart: null,
    hyperlinks: { click: null, hover: null },
    style: defaultStyle(), textStyle: { ...defaultTextStyle(), fontSize }, children: [],
  };
}

function addObjects(objects) {
  mutate(() => {
    objects.forEach((object) => {
      object.id ||= uid("shape");
      object.sourceShapeId ??= null;
      currentSlide().objects.push(object);
    });
    state.selectedId = objects.at(-1)?.id || null;
  });
}

function addObject(object) {
  mutate(() => {
    object.id ||= uid("shape");
    object.sourceShapeId ??= null;
    object.formula ??= null;
    currentSlide().objects.push(object);
    state.selectedId = object.id;
  });
}

function deleteSelected() {
  if (!state.selectedId) return;
  mutate(() => {
    currentSlide().animations = (currentSlide().animations || []).filter((animation) => animation.targetObjectId !== state.selectedId);
    removeObject(currentSlide().objects, state.selectedId);
    state.selectedId = null;
    state.selectedAnimationId = null;
  });
}

function clipboardPayload(object) {
  return `UNIPPT_OBJECT_V1\n${JSON.stringify(object)}`;
}

function copySelectedObject() {
  const object = selectedObject();
  if (!object) return toast("请先选择要复制的对象");
  state.internalClipboard = structuredClone(object);
  state.pasteOffset = 0;
  navigator.clipboard?.writeText(clipboardPayload(state.internalClipboard)).catch(() => {});
  toast(`已复制“${displayObjectName(object)}”`);
}

function cutSelectedObject() {
  if (!selectedObject()) return toast("请先选择要剪切的对象");
  copySelectedObject();
  deleteSelected();
}

async function pasteObject() {
  let source = state.internalClipboard;
  if (!source && navigator.clipboard?.readText) {
    try {
      const value = await navigator.clipboard.readText();
      if (value.startsWith("UNIPPT_OBJECT_V1\n")) source = JSON.parse(value.slice("UNIPPT_OBJECT_V1\n".length));
    } catch (_error) { /* The internal clipboard remains the permission-free fallback. */ }
  }
  if (!source) return toast("剪贴板中没有 UniPPT 对象");
  const clone = structuredClone(source);
  renewIds([clone]);
  state.pasteOffset = (state.pasteOffset % 8) + 1;
  const offset = state.pasteOffset * 14;
  clone.frame.x = clamp((Number(clone.frame.x) || 0) + offset, -clone.frame.width + 12, state.deck.width - 12);
  clone.frame.y = clamp((Number(clone.frame.y) || 0) + offset, -clone.frame.height + 12, state.deck.height - 12);
  clone.name = `${displayObjectName(clone)} 副本`;
  addObject(clone);
  toast(`已粘贴“${displayObjectName(clone)}”`);
}

function activateFormatPainter() {
  const object = selectedObject();
  if (!object) return toast("请先选择格式来源对象");
  state.formatPainter = {
    sourceId: object.id,
    style: structuredClone(object.style),
    textStyle: structuredClone(object.textStyle),
    textFrame: structuredClone(object.textFrame),
  };
  $("#formatPainter").classList.add("active");
  toast("格式刷已启用，请单击目标对象");
}

function findObjectContainer(id, objects = currentSlide().objects) {
  const index = objects.findIndex((object) => object.id === id);
  if (index >= 0) return { objects, index };
  for (const object of objects) {
    if (!object.children?.length) continue;
    const found = findObjectContainer(id, object.children);
    if (found) return found;
  }
  return null;
}

function arrangeSelectedObject(position) {
  if (!state.selectedId) return toast("请先选择要排列的对象");
  mutate(() => {
    const location = findObjectContainer(state.selectedId);
    if (!location) return;
    const [object] = location.objects.splice(location.index, 1);
    if (position === "back") location.objects.unshift(object); else location.objects.push(object);
  });
  toast(position === "back" ? "已置于底层" : "已置于顶层");
}

function applyBasicSlideLayout() {
  const textObjects = flattenObjects(currentSlide().objects).filter((object) => object.kind === "text");
  if (!textObjects.length) return toast("当前幻灯片没有可布局的文本框");
  mutate(() => {
    const [title, ...body] = textObjects;
    Object.assign(title.frame, { x: state.deck.width * .08, y: state.deck.height * .07, width: state.deck.width * .84, height: state.deck.height * .15, rotation: 0 });
    applyTextStyle(title, "fontSize", 44 * 96 / 72);
    setObjectAlignment(title, "center");
    body.forEach((object, index) => {
      const columns = Math.min(2, body.length);
      const column = index % columns;
      const row = Math.floor(index / columns);
      Object.assign(object.frame, { x: state.deck.width * (.08 + column * .43), y: state.deck.height * (.27 + row * .29), width: state.deck.width * .39, height: state.deck.height * .23, rotation: 0 });
    });
  });
  toast("已应用标题与内容版式");
}

function setObjectAlignment(object, align) {
  object.textStyle ||= defaultTextStyle();
  object.textStyle.align = align;
  for (const paragraph of object.textParagraphs || []) paragraph.align = align;
}

function resetSlideLayout() {
  mutate(() => {
    for (const object of flattenObjects(currentSlide().objects)) {
      object.frame.width = clamp(Number(object.frame.width) || 120, 12, state.deck.width);
      object.frame.height = clamp(Number(object.frame.height) || 80, 12, state.deck.height);
      object.frame.x = clamp(Number(object.frame.x) || 0, 0, state.deck.width - object.frame.width);
      object.frame.y = clamp(Number(object.frame.y) || 0, 0, state.deck.height - object.frame.height);
      object.frame.rotation = Number(object.frame.rotation) || 0;
    }
  });
  toast("已修正越界对象并恢复有效尺寸");
}

function parseLatexEnvelope(value, fallbackDisplay = true) {
  const source = String(value ?? "").trim();
  if (source.length >= 4 && source.startsWith("$$") && source.endsWith("$$")) {
    return { latex: source.slice(2, -2).trim(), display: true, delimited: true };
  }
  if (source.length >= 2 && source.startsWith("$") && source.endsWith("$")) {
    return { latex: source.slice(1, -1).trim(), display: false, delimited: true };
  }
  return { latex: source, display: Boolean(fallbackDisplay), delimited: false };
}

function formatLatexSource(value, display = true) {
  const parsed = parseLatexEnvelope(value, display);
  return display ? `$$${parsed.latex}$$` : `$${parsed.latex}$`;
}

function openFormulaDialog(id = null) {
  const object = id ? findObject(id) : null;
  state.editingFormulaId = object?.kind === "math" ? id : null;
  const display = object?.formula?.display ?? true;
  $("#formulaDisplay").checked = display;
  $("#formulaSource").value = formatLatexSource(object?.formula?.latex || "E=mc^2", display);
  $("#commitFormula").textContent = object ? "更新公式" : "插入公式";
  $("#formulaNativeStatus").textContent = object?.formula?.omml ? "已保留原生 OMML" : "将生成原生 OMML";
  renderFormulaPreview();
  $("#formulaDialog").showModal();
  $("#formulaSource").focus();
  $("#formulaSource").select();
}

function renderFormulaPreview() {
  const preview = $("#formulaPreview");
  const parsed = parseLatexEnvelope($("#formulaSource").value, $("#formulaDisplay").checked);
  const source = parsed.latex;
  preview.replaceChildren();
  if (!source) return;
  if (window.katex) {
    katex.render(source, preview, { displayMode: parsed.display, throwOnError: false });
  } else {
    preview.textContent = source;
  }
}

async function commitFormula(event) {
  event.preventDefault();
  const parsed = parseLatexEnvelope($("#formulaSource").value, $("#formulaDisplay").checked);
  const latex = parsed.latex;
  if (!latex) return toast("请输入 LaTeX 公式");
  const button = $("#commitFormula");
  button.disabled = true;
  $("#formulaNativeStatus").textContent = "正在生成 OMML…";
  let omml = null;
  try {
    const response = await fetch("/api/math/latex-to-omml", {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ value: latex }),
    });
    const payload = await readJsonResponse(response);
    omml = payload.value;
    $("#formulaNativeStatus").textContent = "原生 OMML 已生成";
  } catch (error) {
    $("#formulaNativeStatus").textContent = "仅保存 LaTeX";
    toast(`公式仍可编辑，但 OMML 生成失败：${error.message}`);
  }

  const display = parsed.display;
  if (state.editingFormulaId) {
    const object = findObject(state.editingFormulaId);
    if (object) mutate(() => {
      object.formula = { latex, omml, display };
      object.text = latex;
      object.name = "公式";
    });
  } else {
    const object = newSceneObject("math", "公式", { x: 210, y: 250, width: 760, height: display ? 145 : 90 }, latex, 34);
    object.formula = { latex, omml, display };
    object.textStyle.align = "center";
    addObject(object);
  }
  button.disabled = false;
  $("#formulaDialog").close();
  setStatus(omml ? "公式已插入 · LaTeX 与 OMML 双源已保存" : "公式已插入 · 当前仅保存 LaTeX");
}

function openFindReplace(showReplace) {
  $("#replaceField").hidden = !showReplace;
  $("#replaceAll").hidden = !showReplace;
  state.findCursor = -1;
  updateFindResultCount();
  showDialog($("#findReplaceDialog"));
  $("#findQuery").focus();
  $("#findQuery").select();
}

function findTextMatches(query = $("#findQuery").value) {
  const value = String(query || "").trim().toLocaleLowerCase();
  if (!value) return [];
  const matches = [];
  state.deck.slides.forEach((slide, slideIndex) => {
    for (const object of flattenObjects(slide.objects || [])) {
      const text = String(object.text || "");
      if (text.toLocaleLowerCase().includes(value)) matches.push({ slideIndex, objectId: object.id, text, source: "slide" });
    }
    const notes = String(slide.notes || "");
    if (notes.toLocaleLowerCase().includes(value)) matches.push({ slideIndex, objectId: null, text: notes, source: "notes" });
  });
  return matches;
}

function updateFindResultCount() {
  const matches = findTextMatches();
  $("#findResultCount").textContent = matches.length ? `共 ${matches.length} 处` : "未找到匹配内容";
  return matches;
}

function findNextText() {
  const matches = updateFindResultCount();
  if (!matches.length) return;
  state.findCursor = (state.findCursor + 1) % matches.length;
  navigateToTextMatch(matches[state.findCursor], state.findCursor, matches.length, "查找");
}

function navigateToTextMatch(match, index, total, label = "搜索") {
  if (!match) return;
  state.activeSlide = match.slideIndex;
  state.selectedId = match.objectId;
  state.outlineView = false;
  renderAll();
  if (match.objectId) requestAnimationFrame(() => {
    document.querySelector(`[data-id="${cssEscape(match.objectId)}"]`)?.scrollIntoView({ block: "center", inline: "center" });
  });
  setStatus(`${label}：第 ${match.slideIndex + 1} 张（${index + 1}/${total}）`);
}

function searchResultSnippet(text, query) {
  const compact = String(text || "").replace(/\s+/g, " ").trim();
  if (!compact) return "（空白内容）";
  const offset = compact.toLocaleLowerCase().indexOf(String(query || "").trim().toLocaleLowerCase());
  const start = Math.max(0, offset - 24);
  const end = Math.min(compact.length, Math.max(offset + String(query || "").trim().length + 38, 72));
  return `${start ? "…" : ""}${compact.slice(start, end)}${end < compact.length ? "…" : ""}`;
}

function updateOfficeSearchResults() {
  const input = $("#officeSearchInput");
  const popup = $("#officeSearchResults");
  const list = $("#officeSearchResultList");
  if (!input || !popup || !list || !state.deck) return [];
  const query = input.value.trim();
  if (!query) {
    closeOfficeSearchResults();
    list.replaceChildren();
    $("#officeSearchSummary").textContent = "输入关键词以搜索当前演示文稿";
    return [];
  }
  const matches = findTextMatches(query);
  $("#officeSearchSummary").textContent = matches.length ? `找到 ${matches.length} 项结果` : "未找到匹配内容";
  list.replaceChildren();
  if (!matches.length) {
    list.append(element("div", "office-search-empty", `没有包含“${query}”的文字`));
  } else {
    matches.slice(0, 100).forEach((match, index) => {
      const button = element("button", index === state.findCursor ? "active" : "");
      button.type = "button";
      button.role = "option";
      button.setAttribute("aria-selected", String(index === state.findCursor));
      button.append(
        element("strong", "", `幻灯片 ${match.slideIndex + 1}${match.source === "notes" ? " · 备注" : ""}`),
        element("span", "", searchResultSnippet(match.text, query)),
      );
      button.onclick = () => {
        state.findCursor = index;
        navigateToTextMatch(match, index, matches.length);
        updateOfficeSearchResults();
        input.focus({ preventScroll: true });
      };
      list.append(button);
    });
  }
  popup.hidden = false;
  input.setAttribute("aria-expanded", "true");
  return matches;
}

function closeOfficeSearchResults() {
  const popup = $("#officeSearchResults");
  if (!popup || popup.hidden) return;
  popup.hidden = true;
  $("#officeSearchInput")?.setAttribute("aria-expanded", "false");
}

function handleOfficeSearchKeydown(event) {
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    closeOfficeSearchResults();
    event.currentTarget.blur();
    return;
  }
  if (!['Enter', 'ArrowDown', 'ArrowUp'].includes(event.key)) return;
  event.preventDefault();
  event.stopPropagation();
  const matches = updateOfficeSearchResults();
  if (!matches.length) return;
  const delta = event.key === "ArrowUp" || (event.key === "Enter" && event.shiftKey) ? -1 : 1;
  state.findCursor = (state.findCursor + delta + matches.length) % matches.length;
  navigateToTextMatch(matches[state.findCursor], state.findCursor, matches.length);
  updateOfficeSearchResults();
}

function replaceAllText() {
  const query = String($("#findQuery").value || "");
  const replacement = String($("#replaceQuery").value || "");
  if (!query) return toast("请输入查找内容");
  const expression = new RegExp(escapeRegExp(query), "giu");
  let count = 0;
  mutate(() => {
    for (const slide of state.deck.slides) {
      for (const object of flattenObjects(slide.objects || [])) {
        const before = String(object.text || "");
        const after = before.replace(expression, () => { count++; return replacement; });
        if (after === before) continue;
        object.text = after;
        if (object.textParagraphs?.length) {
          object.textParagraphs = reconcileRichTextParagraphs(object.textParagraphs, after.split("\n"), object.textStyle || defaultTextStyle());
        }
      }
      slide.notes = String(slide.notes || "").replace(expression, () => { count++; return replacement; });
    }
  });
  updateFindResultCount();
  toast(`已替换 ${count} 处`);
}

function selectNextObject() {
  const objects = flattenObjects(currentSlide().objects || []);
  if (!objects.length) return toast("当前幻灯片没有对象");
  const index = objects.findIndex((object) => object.id === state.selectedId);
  selectObject(objects[(index + 1) % objects.length].id);
}

function cycleThemeColor() {
  const themes = ["theme-chip theme-office", "theme-chip theme-warm", "theme-chip theme-minimal", "theme-chip theme-dark"];
  const index = Number($("#cycleThemeColor").dataset.index || 0);
  applyTheme(themes[index % themes.length]);
  $("#cycleThemeColor").dataset.index = String(index + 1);
}

function cycleThemeFont() {
  const fonts = ["Aptos", "微软雅黑", "等线"];
  const index = Number($("#cycleThemeFont").dataset.index || 0);
  const font = fonts[index % fonts.length];
  mutate(() => {
    for (const slide of state.deck.slides) for (const object of flattenObjects(slide.objects || [])) applyNativeFontFamily(object, font);
  });
  $("#cycleThemeFont").dataset.index = String(index + 1);
  toast(`全局字体：${font}`);
}

function toggleThemeEffects() {
  const enabled = $("#toggleThemeEffects").dataset.enabled !== "true";
  mutate(() => {
    for (const object of flattenObjects(currentSlide().objects || [])) {
      if (["shape", "image", "chart", "table"].includes(object.kind)) object.style.shadow = enabled ? { color: "#000000", opacity: .22, offsetX: 4, offsetY: 4, blur: 12, inset: false } : null;
    }
  });
  $("#toggleThemeEffects").dataset.enabled = String(enabled);
  toast(enabled ? "已应用柔和阴影效果" : "已移除主题阴影效果");
}

function toggleSlideSize() {
  const nextWidth = Math.abs(state.deck.width / state.deck.height - 16 / 9) < .03 ? state.deck.height * 4 / 3 : state.deck.height * 16 / 9;
  const scaleX = nextWidth / state.deck.width;
  mutate(() => {
    for (const slide of state.deck.slides) {
      for (const object of [...(slide.masterObjects || []), ...(slide.layoutObjects || []), ...(slide.objects || [])]) scaleObjectX(object, scaleX);
    }
    state.deck.width = nextWidth;
  });
  toast(Math.abs(nextWidth / state.deck.height - 16 / 9) < .03 ? "幻灯片大小：宽屏 16:9" : "幻灯片大小：标准 4:3");
}

function scaleObjectX(object, scale) {
  object.frame.x *= scale;
  object.frame.width *= scale;
  for (const child of object.children || []) scaleObjectX(child, scale);
}

function showDialog(dialog) {
  if (!dialog || dialog.open) return;
  try {
    if (typeof dialog.showModal === "function") dialog.showModal();
    else dialog.setAttribute("open", "");
  } catch (_error) {
    dialog.setAttribute("open", "");
  }
}
function openHelpDialog() {
  const dialog = $("#helpDialog");
  if (!dialog) return;
  dialog.open = true;
}
function showFeatureStatus() { toast("Ribbon 可用命令已接入；尚无无损模型的命令会明确禁用"); }

function startSpellCheck() {
  const object = selectedObject();
  if (!object || !String(object.text || "").trim()) return toast("请先选择含文字的对象");
  const node = document.querySelector(`[data-id="${cssEscape(object.id)}"]`);
  const content = node?.querySelector(".object-content,.rich-text");
  if (!content) return toast("所选对象不支持文本校对");
  content.spellcheck = true;
  node.dispatchEvent(new MouseEvent("dblclick", { bubbles: true, clientX: node.getBoundingClientRect().left + 12, clientY: node.getBoundingClientRect().top + 12 }));
  toast("已进入浏览器拼写检查模式");
}

function runAccessibilityCheck() {
  let issues = 0;
  for (const slide of state.deck.slides) {
    if (!String(slide.name || "").trim()) issues++;
    for (const object of flattenObjects(slide.objects || [])) {
      if (["image", "chart", "media"].includes(object.kind) && !String(object.name || "").trim()) issues++;
      if (object.frame.width <= 0 || object.frame.height <= 0) issues++;
    }
  }
  $("#accessibilityStatus").textContent = issues ? `辅助功能: ${issues} 个问题` : "辅助功能: 良好";
  toast(issues ? `发现 ${issues} 个缺少名称或无效尺寸的问题` : "辅助功能检查通过");
}

function focusSlideNotes() {
  $("#slideNotes").focus();
  $("#slideNotes").select();
}

function showNormalView() {
  document.body.classList.remove("slide-sorter-mode");
  showSlidesPane();
  closeAnimationPane();
  closeFormatPane();
  fitSlide();
}

function showSlideSorterView() {
  document.body.classList.add("slide-sorter-mode");
  state.outlineView = false;
  renderSlideList();
}

function showOutlineView() { document.body.classList.remove("slide-sorter-mode"); showOutlinePane(); }
function showSlidesPane() { state.outlineView = false; $("#slidesPaneTab").classList.add("active"); $("#outlinePaneTab").classList.remove("active"); renderSlideList(); }
function showOutlinePane() { state.outlineView = true; $("#slidesPaneTab").classList.remove("active"); $("#outlinePaneTab").classList.add("active"); renderSlideList(); }
function toggleGridLines() { document.body.classList.toggle("canvas-grid-hidden"); $("#toggleGridLines").classList.toggle("active", !document.body.classList.contains("canvas-grid-hidden")); }

function applyTheme(className) {
  const backgrounds = {
    "theme-chip theme-office": "#ffffff",
    "theme-chip theme-dark": "linear-gradient(135deg,#182131,#31485f)",
    "theme-chip theme-warm": "linear-gradient(135deg,#fffaf0,#f3e2c2)",
    "theme-chip theme-minimal": "linear-gradient(135deg,#ffffff,#f1f1f1)",
  };
  mutate(() => {
    currentSlide().background = backgrounds[className] || "#ffffff";
    currentSlide().backgroundAsset = null;
  });
}

function rememberCanvasPointer(event) {
  const viewport = $("#canvasViewport");
  if (!viewport || !Number.isFinite(event?.clientX) || !Number.isFinite(event?.clientY)) return;
  const bounds = viewport.getBoundingClientRect();
  if (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom) return;
  state.canvasPointer = { clientX: event.clientX, clientY: event.clientY };
}

function currentCanvasZoomAnchor(anchor = state.canvasPointer) {
  const viewport = $("#canvasViewport");
  if (!viewport) return null;
  const bounds = viewport.getBoundingClientRect();
  const clientX = Number.isFinite(anchor?.clientX) ? anchor.clientX : (bounds.left + bounds.right) / 2;
  const clientY = Number.isFinite(anchor?.clientY) ? anchor.clientY : (bounds.top + bounds.bottom) / 2;
  return {
    clientX: clamp(clientX, bounds.left, bounds.right),
    clientY: clamp(clientY, bounds.top, bounds.bottom),
  };
}

function canvasZoomCorrection(anchor, beforeRect, afterRect, previousZoom, nextZoom) {
  const localX = (anchor.clientX - beforeRect.left) / previousZoom;
  const localY = (anchor.clientY - beforeRect.top) / previousZoom;
  return {
    left: afterRect.left + localX * nextZoom - anchor.clientX,
    top: afterRect.top + localY * nextZoom - anchor.clientY,
  };
}

function adjustZoom(delta, anchor = currentCanvasZoomAnchor()) {
  setZoom(state.zoom + delta, false, anchor);
}

function setZoom(value, autoFit = false, anchor = null) {
  const normalized = Math.round(clamp(Number(value) || 1, MIN_ZOOM, MAX_ZOOM) * 100) / 100;
  const previousZoom = state.zoom;
  const viewport = $("#canvasViewport");
  const stage = $("#slideStage");
  const zoomAnchor = !autoFit && viewport && stage ? currentCanvasZoomAnchor(anchor) : null;
  const beforeRect = zoomAnchor ? stage.getBoundingClientRect() : null;
  state.zoom = normalized;
  state.autoFit = autoFit;
  updateCanvasScale();
  if (zoomAnchor && beforeRect && Math.abs(previousZoom - normalized) > .0001) {
    const correction = canvasZoomCorrection(
      zoomAnchor,
      beforeRect,
      stage.getBoundingClientRect(),
      previousZoom,
      normalized,
    );
    viewport.scrollLeft += correction.left;
    viewport.scrollTop += correction.top;
  }
}

function fitSlide() {
  if (!state.deck) return;
  const viewport = $("#canvasViewport");
  const style = getComputedStyle(viewport);
  const horizontalPadding = (parseFloat(style.paddingLeft) || 0) + (parseFloat(style.paddingRight) || 0);
  const verticalPadding = (parseFloat(style.paddingTop) || 0) + (parseFloat(style.paddingBottom) || 0);
  const width = Math.max(32, viewport.clientWidth - horizontalPadding - 20);
  const height = Math.max(32, viewport.clientHeight - verticalPadding - 20);
  const fitted = Math.floor(Math.min(width / state.deck.width, height / state.deck.height) * 100) / 100;
  setZoom(fitted, true);
}

function scheduleFitSlide() {
  cancelAnimationFrame(fitFrame);
  fitFrame = requestAnimationFrame(fitSlide);
}

function updateCanvasScale() {
  if (!state.deck) return;
  const stage = $("#slideStage");
  stage.style.transform = `scale(${state.zoom})`;
  stage.style.setProperty("--selection-ui-scale", String(1 / Math.max(.01, state.zoom)));
  $("#canvasScaler").style.width = `${state.deck.width * state.zoom}px`;
  $("#canvasScaler").style.height = `${state.deck.height * state.zoom}px`;
  const percent = Math.round(state.zoom * 100);
  $("#zoom").value = String(percent);
  $("#zoomValue").value = `${percent}%`;
  $("#zoomValue").textContent = `${percent}%`;
  $("#zoom").setAttribute("aria-valuetext", `${percent}%`);
  $("#zoomOut").disabled = state.zoom <= MIN_ZOOM;
  $("#zoomIn").disabled = state.zoom >= MAX_ZOOM;
  $("#fitSlide").classList.toggle("active", state.autoFit);
  $("#fitSlide").setAttribute("aria-pressed", String(state.autoFit));
}

function slideShowBounds(ignoreRange = false) {
  const last = Math.max(0, (state.deck?.slides?.length || 1) - 1);
  if (ignoreRange || state.slideShowSettings.range !== "range") return { start: 0, end: last };
  const start = clamp(Math.trunc(Number(state.slideShowSettings.from) || 1) - 1, 0, last);
  const end = clamp(Math.trunc(Number(state.slideShowSettings.to) || last + 1) - 1, start, last);
  return { start, end };
}

function openSlideShowSettings() {
  if (!state.deck?.slides?.length) return;
  const total = state.deck.slides.length;
  const settings = state.slideShowSettings;
  $("#slideShowAll").checked = settings.range !== "range";
  $("#slideShowRange").checked = settings.range === "range";
  $("#slideShowFrom").max = String(total);
  $("#slideShowTo").max = String(total);
  $("#slideShowFrom").value = String(clamp(settings.from || 1, 1, total));
  $("#slideShowTo").value = String(clamp(settings.to || total, 1, total));
  $("#slideShowLoop").checked = Boolean(settings.loop);
  $("#slideShowAutoPlay").checked = Boolean(settings.autoPlay);
  $("#slideShowUseTimings").checked = settings.useTimings !== false;
  $("#slideShowFullscreen").checked = settings.fullscreen !== false;
  syncSlideShowRangeInputs();
  const dialog = $("#slideShowSettingsDialog");
  if (!dialog.open) dialog.showModal();
}

function syncSlideShowRangeInputs() {
  const enabled = Boolean($("#slideShowRange")?.checked);
  $("#slideShowFrom").disabled = !enabled;
  $("#slideShowTo").disabled = !enabled;
}

function closeSlideShowSettings() {
  const dialog = $("#slideShowSettingsDialog");
  if (dialog.open) dialog.close();
}

function applySlideShowSettings() {
  if (!state.deck?.slides?.length) return closeSlideShowSettings();
  const total = state.deck.slides.length;
  const from = clamp(Math.trunc(Number($("#slideShowFrom").value) || 1), 1, total);
  const to = clamp(Math.trunc(Number($("#slideShowTo").value) || total), from, total);
  state.slideShowSettings = {
    range: $("#slideShowRange").checked ? "range" : "all",
    from,
    to,
    loop: $("#slideShowLoop").checked,
    autoPlay: $("#slideShowAutoPlay").checked,
    useTimings: $("#slideShowUseTimings").checked,
    fullscreen: $("#slideShowFullscreen").checked,
  };
  closeSlideShowSettings();
  toast(state.slideShowSettings.range === "range" ? `放映范围已设为第 ${from}–${to} 张` : "放映设置已保存");
}

function presentationNow() {
  return globalThis.performance?.now ? globalThis.performance.now() : Date.now();
}

function formatPresentationTime(milliseconds) {
  const seconds = Math.max(0, Math.floor((Number(milliseconds) || 0) / 1000));
  const minutes = Math.floor(seconds / 60);
  return `${String(minutes).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

function updateRehearsalClock() {
  const clock = $("#rehearsalClock");
  if (state.presenterMode !== "rehearse") {
    clock.hidden = true;
    return;
  }
  const now = presentationNow();
  const slideElapsed = now - state.rehearsalSlideStartedAt;
  const totalElapsed = now - state.rehearsalStartedAt;
  clock.hidden = false;
  clock.textContent = `本页 ${formatPresentationTime(slideElapsed)} · 总计 ${formatPresentationTime(totalElapsed)}`;
  clock.title = `第 ${state.presenterSlide + 1} 张排练计时`;
}

function recordRehearsalTiming() {
  if (state.presenterMode !== "rehearse") return;
  const now = presentationNow();
  state.rehearsalTimings[state.presenterSlide] = Math.max(100, Math.round(now - state.rehearsalSlideStartedAt));
  state.rehearsalSlideStartedAt = now;
  updateRehearsalClock();
}

function startRehearsal() {
  if (!state.deck?.slides?.length) return;
  void startPresentationWhenReady("beginning", { rehearsal: true, ignoreRange: true });
}

function showRehearsalResult() {
  const entries = Object.entries(state.rehearsalTimings)
    .map(([index, milliseconds]) => [Number(index), Number(milliseconds)])
    .filter(([index, milliseconds]) => Number.isFinite(index) && milliseconds > 0)
    .sort((a, b) => a[0] - b[0]);
  if (!entries.length) return;
  const total = entries.reduce((sum, [, milliseconds]) => sum + milliseconds, 0);
  $("#rehearsalSummary").textContent = `已记录 ${entries.length} 张幻灯片，总计 ${formatPresentationTime(total)}。是否保留这些计时？`;
  $("#rehearsalTimingList").replaceChildren(...entries.map(([index, milliseconds]) => element("span", "", `第 ${index + 1} 张  ${formatPresentationTime(milliseconds)}`)));
  const dialog = $("#rehearsalResultDialog");
  if (!dialog.open) dialog.showModal();
}

function discardRehearsalTimings() {
  const dialog = $("#rehearsalResultDialog");
  if (dialog.open) dialog.close();
  state.rehearsalTimings = {};
  toast("已放弃本次排练计时");
}

function applyRehearsalTimings() {
  const entries = Object.entries(state.rehearsalTimings);
  if (entries.length) {
    mutate(() => {
      for (const [rawIndex, rawMilliseconds] of entries) {
        const slide = state.deck.slides[Number(rawIndex)];
        if (!slide) continue;
        slide.transition ||= { kind: "cut", durationMs: 1, advanceOnClick: true, advanceAfterMs: null, direction: null };
        slide.transition.advanceAfterMs = Math.max(100, Math.round(Number(rawMilliseconds) || 0));
      }
    });
  }
  const dialog = $("#rehearsalResultDialog");
  if (dialog.open) dialog.close();
  state.rehearsalTimings = {};
  toast(`已保留 ${entries.length} 张幻灯片的排练计时`);
}

function summarizePresentationFrames(frameIntervals) {
  const intervals = [...(frameIntervals || [])]
    .map(Number)
    .filter((milliseconds) => Number.isFinite(milliseconds) && milliseconds > 0 && milliseconds < 1000);
  if (intervals.length < 3) return null;
  const total = intervals.reduce((sum, milliseconds) => sum + milliseconds, 0);
  const averageFps = total > 0 ? intervals.length * 1000 / total : 0;
  const sorted = [...intervals].sort((a, b) => a - b);
  const p95FrameMs = sorted[Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * .95))];
  const maxFrameMs = sorted[sorted.length - 1];
  const effectiveFps = Math.min(averageFps, 1000 / Math.max(.001, p95FrameMs));
  // Chromium's rAF timestamps on a 30 Hz display commonly wobble around
  // 33.33 ms.  Treat only intervals over 36 ms as dropped so a genuine
  // 30-fps presentation is not rejected by sub-millisecond timer jitter.
  const droppedFrames = intervals.filter((milliseconds) => milliseconds > PRESENTATION_DROPPED_FRAME_MS).length;
  const droppedRatio = droppedFrames / intervals.length;
  return {
    averageFps,
    effectiveFps,
    p95FrameMs,
    maxFrameMs,
    droppedFrames,
    droppedRatio,
    frameCount: intervals.length,
    passed: effectiveFps >= PRESENTATION_MIN_FPS - .5
      && p95FrameMs <= PRESENTATION_DROPPED_FRAME_MS
      && droppedRatio <= .05
      && maxFrameMs <= PRESENTATION_MAX_FRAME_MS,
  };
}

function presentationPerformanceKey(label = "presentation") {
  const normalized = String(label || "presentation").toLowerCase();
  if (normalized.startsWith("slide-transition:")) return normalized.split(":").slice(0, 2).join(":");
  if (normalized.startsWith("native-animation:")) return "native-animation";
  return normalized;
}

function presentationQualityTier(label = "presentation") {
  return presentationPerformance.qualityByKind[presentationPerformanceKey(label)] || "high";
}

function nextPresentationQualityTier(tier, direction) {
  const tiers = ["low", "medium", "high"];
  const index = Math.max(0, tiers.indexOf(tier));
  return tiers[clamp(index + Math.sign(direction || 0), 0, tiers.length - 1)];
}

function updatePresentationPerformance(sample) {
  if (!sample) return;
  const performanceKey = presentationPerformanceKey(sample.label);
  sample.performanceKey = performanceKey;
  presentationPerformance.lastSample = sample;
  presentationPerformance.lastFps = sample.effectiveFps;
  presentationPerformance.samples.push(sample);
  if (presentationPerformance.samples.length > 12) presentationPerformance.samples.shift();
  const streak = presentationPerformance.streaksByKind[performanceKey] || { slow: 0, fast: 0 };
  let tier = presentationQualityTier(performanceKey);
  if (!sample.insufficient) {
    if (!sample.passed) {
      streak.slow += 1;
      streak.fast = 0;
      // Degrade only the effect that missed budget; a heavy Wind transition
      // must not reduce the fidelity of ordinary opacity animations.
      tier = nextPresentationQualityTier(tier, -1);
      if (streak.slow >= 2) tier = "low";
    } else if (sample.effectiveFps >= 50 && sample.p95FrameMs < 25 && sample.droppedRatio <= .02) {
      streak.fast += 1;
      streak.slow = 0;
      if (streak.fast >= 5) {
        tier = nextPresentationQualityTier(tier, 1);
        streak.fast = 0;
      }
    } else {
      streak.fast = 0;
      streak.slow = 0;
    }
    presentationPerformance.qualityByKind[performanceKey] = tier;
    presentationPerformance.streaksByKind[performanceKey] = streak;
  }
  presentationPerformance.tier = tier;
  presentationPerformance.slowRuns = streak.slow;
  presentationPerformance.fastRuns = streak.fast;
  const presenter = $("#presenter");
  if (!presenter) return;
  presenter.dataset.presentationFps = sample.effectiveFps.toFixed(1);
  presenter.dataset.presentationAverageFps = sample.averageFps.toFixed(1);
  presenter.dataset.presentationP95FrameMs = sample.p95FrameMs.toFixed(2);
  presenter.dataset.presentationMaxFrameMs = sample.maxFrameMs.toFixed(2);
  presenter.dataset.presentationDroppedRatio = sample.droppedRatio.toFixed(4);
  presenter.dataset.presentationBudgetPassed = sample.insufficient ? "insufficient" : String(sample.passed);
  presenter.dataset.presentationQuality = tier;
  presenter.dataset.presentationMotion = sample.label || "presentation";
  presenter.dataset.presentationSamples = JSON.stringify(presentationPerformance.samples);
}

function beginPresentationFrameProbe(label = "presentation", expectedDurationMs = 0, initialMetadata = {}) {
  if (
    typeof requestAnimationFrame !== "function"
    || typeof cancelAnimationFrame !== "function"
    || typeof performance?.now !== "function"
    || document.hidden
  ) return { annotate() {}, finish() { return null; } };
  const intervals = [];
  const startedAt = performance.now();
  const expectedDuration = Math.max(0, Number(expectedDurationMs) || 0);
  let metadata = { ...initialMetadata };
  let active = true;
  let paused = false;
  let lastFrame = startedAt;
  let firstFrameDelayMs = null;
  let visibleSampleMs = 0;
  let resumeWarmupFrames = 0;
  let frameRequest = 0;
  const removeListeners = [];
  const setPaused = (nextPaused) => {
    if (!active || paused === nextPaused) return;
    paused = nextPaused;
    lastFrame = null;
    if (!paused) resumeWarmupFrames = 2;
  };
  const handleVisibility = () => setPaused(Boolean(document.hidden));
  if (typeof document.addEventListener === "function") {
    document.addEventListener("visibilitychange", handleVisibility);
    removeListeners.push(() => document.removeEventListener("visibilitychange", handleVisibility));
  }
  if (typeof globalThis.addEventListener === "function") {
    const handlePageHide = () => setPaused(true);
    const handlePageShow = () => setPaused(Boolean(document.hidden));
    globalThis.addEventListener("pagehide", handlePageHide);
    globalThis.addEventListener("pageshow", handlePageShow);
    removeListeners.push(() => globalThis.removeEventListener("pagehide", handlePageHide));
    removeListeners.push(() => globalThis.removeEventListener("pageshow", handlePageShow));
  }
  const probe = {
    annotate(nextMetadata = {}) {
      metadata = { ...metadata, ...nextMetadata };
    },
    finish(commit = true) {
      if (!active) return null;
      active = false;
      if (frameRequest) cancelAnimationFrame(frameRequest);
      removeListeners.forEach((remove) => remove());
      activePresentationFrameProbes.delete(probe);
      if (!commit || document.hidden) return null;
      const summary = summarizePresentationFrames(intervals);
      if (!summary) return null;
      summary.label = label;
      summary.expectedDurationMs = expectedDuration;
      summary.visibleSampleMs = visibleSampleMs;
      summary.firstFrameDelayMs = firstFrameDelayMs;
      Object.assign(summary, metadata);
      const requiredSampleMs = expectedDuration
        ? Math.min(expectedDuration, Math.max(200, expectedDuration * .35))
        : 200;
      summary.insufficient = visibleSampleMs < requiredSampleMs;
      updatePresentationPerformance(summary);
      return summary;
    },
  };
  const sampleFrame = (timestamp) => {
    if (!active) return;
    if (document.hidden || paused) {
      lastFrame = null;
      frameRequest = requestAnimationFrame(sampleFrame);
      return;
    }
    if (firstFrameDelayMs == null) firstFrameDelayMs = Math.max(0, timestamp - startedAt);
    if (lastFrame == null || resumeWarmupFrames > 0) {
      lastFrame = timestamp;
      if (resumeWarmupFrames > 0) resumeWarmupFrames -= 1;
      frameRequest = requestAnimationFrame(sampleFrame);
      return;
    }
    const interval = timestamp - lastFrame;
    lastFrame = timestamp;
    if (interval > 0 && interval < 1000) {
      intervals.push(interval);
      visibleSampleMs += interval;
    }
    frameRequest = requestAnimationFrame(sampleFrame);
  };
  activePresentationFrameProbes.add(probe);
  frameRequest = requestAnimationFrame(sampleFrame);
  return probe;
}

function cancelPresentationFrameProbes(commit = false) {
  for (const probe of [...activePresentationFrameProbes]) probe.finish(commit);
}

function resetPresentationPerformance() {
  cancelPresentationFrameProbes(false);
  presentationPerformance.tier = "high";
  presentationPerformance.lastFps = 60;
  presentationPerformance.lastSample = null;
  presentationPerformance.slowRuns = 0;
  presentationPerformance.fastRuns = 0;
  presentationPerformance.qualityByKind = Object.create(null);
  presentationPerformance.streaksByKind = Object.create(null);
  presentationPerformance.samples = [];
  const presenter = $("#presenter");
  if (!presenter) return;
  presenter.dataset.presentationMinFps = String(PRESENTATION_MIN_FPS);
  presenter.dataset.presentationFps = "60.0";
  presenter.dataset.presentationBudgetPassed = "true";
  presenter.dataset.presentationQuality = "high";
  presenter.dataset.presentationMotion = "idle";
  presenter.dataset.presentationSamples = "[]";
}

globalThis.UniPptDiagnostics = Object.freeze({
  minFps: PRESENTATION_MIN_FPS,
  reset: resetPresentationPerformance,
  snapshot() {
    return {
      minFps: PRESENTATION_MIN_FPS,
      tier: presentationPerformance.tier,
      qualityByKind: { ...presentationPerformance.qualityByKind },
      lastFps: presentationPerformance.lastFps,
      lastSample: presentationPerformance.lastSample ? { ...presentationPerformance.lastSample } : null,
      samples: presentationPerformance.samples.map((sample) => ({ ...sample })),
    };
  },
});

async function startPresentationWhenReady(startMode = "current", options = {}) {
  try {
    await ensureFeature("presentation");
    startPresentation(startMode, options);
  } catch (error) {
    toast(`放映模块加载失败：${error.message || error}`);
  }
}

function startPresentation(startMode = "current", options = {}) {
  if (!state.deck?.slides?.length) return;
  hidePresentationBoundaryNotice();
  if (options.autoPlay != null) state.slideShowSettings.autoPlay = Boolean(options.autoPlay);
  globalThis.UniPptMedia?.stopPersistent(true);
  clearInterval(state.rehearsalClockTimer);
  state.rehearsalClockTimer = 0;
  let bounds = slideShowBounds(Boolean(options.ignoreRange));
  let startIndex = startMode === "beginning" ? bounds.start : clamp(state.activeSlide, 0, state.deck.slides.length - 1);
  if (startIndex < bounds.start || startIndex > bounds.end) bounds = slideShowBounds(true);
  state.presenterRangeStart = bounds.start;
  state.presenterRangeEnd = bounds.end;
  state.presenterSlide = startIndex;
  state.presenterAnimationCursor = 0;
  state.presenterAnimationTargetCursor = null;
  state.presenterAnimationBatchStartCursor = null;
  state.presenterMode = options.rehearsal ? "rehearse" : "show";
  resetPresentationPerformance();
  if (state.presenterMode === "rehearse") {
    state.rehearsalTimings = {};
    state.rehearsalStartedAt = presentationNow();
    state.rehearsalSlideStartedAt = state.rehearsalStartedAt;
    state.rehearsalClockTimer = setInterval(updateRehearsalClock, 250);
  }
  const presenter = $("#presenter");
  document.body?.classList.add("presentation-active");
  presenter.hidden = false;
  presenter.classList.toggle("rehearsing", state.presenterMode === "rehearse");
  $$(".status-view").forEach((button) => button.classList.toggle("active", button.id === "readingView"));
  renderPresentation(false, { runInitialAutomatic: options.runInitialAutomatic });
  updatePresentationAudioControls();
  updatePresentationAutoPlayControl();
  updateRehearsalClock();
  showPresentationControls(state.presenterMode === "rehearse");
  if (state.slideShowSettings.fullscreen !== false && !document.fullscreenElement && presenter.requestFullscreen) {
    void presenter.requestFullscreen({ navigationUI: "hide" }).then(() => {
      if (presenter.hidden && document.fullscreenElement === presenter) {
        void document.exitFullscreen().catch(() => {});
      }
    }).catch(() => {
      state.presenterFullscreen = false;
      fitPresentationStage();
    });
  }
}
function closePresentation(exitFullscreen = true) {
  const wasRehearsing = state.presenterMode === "rehearse";
  if (wasRehearsing) recordRehearsalTiming();
  clearInterval(state.rehearsalClockTimer);
  state.rehearsalClockTimer = 0;
  state.presenterTimers.forEach(clearTimeout);
  state.presenterTimers = [];
  clearPresentationAutoPlayTimer();
  hidePresentationBoundaryNotice();
  state.presenterPlayers.forEach((player) => player.cancel());
  state.presenterPlayers = [];
  state.presenterAnimationTargetCursor = null;
  state.presenterAnimationBatchStartCursor = null;
  state.presenterPlaybackToken += 1;
  cancelPresentationFrameProbes(false);
  hidePresentationControls();
  globalThis.UniPptMedia?.stopAll($("#presentStage"), true);
  clearSlideTransition($("#presentStage"));
  globalThis.UniPptMedia?.stopPersistent(true);
  const presenter = $("#presenter");
  presenter.hidden = true;
  document.body?.classList.remove("presentation-active");
  presenter.classList.remove("rehearsing");
  $("#rehearsalClock").hidden = true;
  state.presenterMode = "show";
  state.presenterFullscreen = false;
  let fullscreenExit = Promise.resolve();
  if (exitFullscreen && document.fullscreenElement === presenter && document.exitFullscreen) {
    fullscreenExit = document.exitFullscreen().catch(() => {});
  }
  $$(".status-view").forEach((button) => button.classList.toggle("active", button.id === "normalView"));
  // The browser recorder needs to know when a self-advancing show has run out
  // of slides so it can close the take. A DOM event keeps that seam one-way.
  document.dispatchEvent(new CustomEvent("unippt:presentation-closed"));
  if (wasRehearsing) void fullscreenExit.finally(() => setTimeout(showRehearsalResult, 0));
}

function presentationAutoPlayEnabled() {
  return state.presenterMode === "show" && Boolean(state.slideShowSettings.autoPlay);
}

function presentationIsRecording() {
  return Boolean($("#presenter")?.classList.contains("recording"));
}

function presentationSlideTargetMs(slide) {
  const raw = slide?.transition?.advanceAfterMs;
  if (state.slideShowSettings.useTimings !== false && raw != null) {
    const explicit = Number(raw);
    if (Number.isFinite(explicit)) return Math.max(100, explicit);
  }
  return presentationIsRecording() ? PRESENTATION_RECORDING_DEFAULT_SLIDE_MS : null;
}

function presentationSlideElapsedMs() {
  return Math.max(0, presentationNow() - (state.presenterSlideStartedAt || presentationNow()));
}

function presentationInitialAutoPlayDelay(slide, transitionLead, hasAnimations) {
  if (hasAnimations) return Math.max(0, transitionLead);
  const target = presentationSlideTargetMs(slide);
  if (target == null) return Math.max(0, transitionLead) + PRESENTATION_DEFAULT_AUTO_ADVANCE_MS;
  return Math.max(transitionLead, target - presentationSlideElapsedMs());
}

function presentationCompletionDelay(slide, minimum = PRESENTATION_MIN_POST_ANIMATION_MS) {
  const target = presentationSlideTargetMs(slide);
  if (target == null) return minimum;
  return Math.max(minimum, target - presentationSlideElapsedMs());
}

function clearPresentationAutoPlayTimer() {
  clearTimeout(state.presenterAutoPlayTimer);
  state.presenterAutoPlayTimer = 0;
}

function updatePresentationAutoPlayControl() {
  const button = $("#presentAutoPlay");
  if (!button) return;
  const active = presentationAutoPlayEnabled();
  button.classList.toggle("active", active);
  button.setAttribute("aria-pressed", String(active));
  button.title = active
    ? "停止自动放映（当前动画仍会完成，A）"
    : "自动放映：本页全部动画完成后自动换页（A）";
}

function schedulePresentationAutoPlay(delay = 0) {
  clearPresentationAutoPlayTimer();
  if (!presentationAutoPlayEnabled() || $("#presenter")?.hidden) return;
  const playbackToken = state.presenterPlaybackToken;
  state.presenterAutoPlayTimer = setTimeout(() => {
    state.presenterAutoPlayTimer = 0;
    if (
      playbackToken !== state.presenterPlaybackToken
      || !presentationAutoPlayEnabled()
      || $("#presenter")?.hidden
    ) return;
    if (state.presenterAnimationTargetCursor != null) {
      schedulePresentationAutoPlay(40);
      return;
    }
    advancePresentation();
  }, Math.max(0, Number(delay) || 0));
}

function togglePresentationAutoPlay(force) {
  const enabled = force == null ? !state.slideShowSettings.autoPlay : Boolean(force);
  state.slideShowSettings.autoPlay = enabled;
  updatePresentationAutoPlayControl();
  if (!enabled) {
    clearPresentationAutoPlayTimer();
    return;
  }
  schedulePresentationAutoPlay(40);
}
function handlePresentationFullscreenChange() {
  const presenter = $("#presenter");
  if (presenter.hidden) return;
  if (!document.fullscreenElement && state.presenterFullscreen) {
    closePresentation(false);
    return;
  }
  state.presenterFullscreen = document.fullscreenElement === presenter;
  requestAnimationFrame(fitPresentationStage);
}
function showPresentationControls(pinned = false) {
  const presenter = $("#presenter");
  if (presenter.hidden) return;
  clearTimeout(state.presenterControlsTimer);
  state.presenterControlsTimer = 0;
  presenter.classList.add("controls-visible");
  if (pinned) return;
  state.presenterControlsTimer = setTimeout(() => {
    presenter.classList.remove("controls-visible");
    state.presenterControlsTimer = 0;
  }, 1500);
}
function hidePresentationControls() {
  clearTimeout(state.presenterControlsTimer);
  state.presenterControlsTimer = 0;
  $("#presenter").classList.remove("controls-visible");
}

function hidePresentationBoundaryNotice() {
  clearTimeout(state.presenterBoundaryTimer);
  state.presenterBoundaryTimer = 0;
  const notice = $("#presentationBoundaryNotice");
  if (!notice) return;
  notice.classList.remove("visible");
  notice.hidden = true;
}

function showPresentationBoundaryNotice() {
  const presenter = $("#presenter");
  const notice = $("#presentationBoundaryNotice");
  if (!notice || presenter?.hidden) return;
  clearTimeout(state.presenterBoundaryTimer);
  notice.hidden = false;
  notice.classList.remove("visible");
  void notice.offsetWidth;
  notice.classList.add("visible");
  state.presenterBoundaryTimer = setTimeout(() => {
    notice.classList.remove("visible");
    notice.hidden = true;
    state.presenterBoundaryTimer = 0;
  }, 1900);
}

function syncPresentationOutput() {
  globalThis.UniPptMedia?.setOutput(
    $("#presentStage"),
    state.presenterVolume,
    state.presenterMuted,
    true,
  );
}

function updatePresentationAudioControls() {
  const volume = clamp(Number(state.presenterVolume) || 0, 0, 1);
  const silent = state.presenterMuted || volume === 0;
  const button = $("#presentMute");
  const slider = $("#presentVolume");
  const glyph = button.querySelector(".present-volume-glyph");
  glyph.textContent = silent ? "🔇" : volume < 0.5 ? "🔉" : "🔊";
  button.title = silent ? "取消静音（M）" : "静音（M）";
  button.setAttribute("aria-label", button.title);
  button.setAttribute("aria-pressed", String(silent));
  slider.value = String(Math.round(volume * 100));
  slider.setAttribute("aria-valuetext", `${Math.round(volume * 100)}%${state.presenterMuted ? "，已静音" : ""}`);
}

function setPresentationVolume(volume) {
  state.presenterVolume = clamp(Number(volume) || 0, 0, 1);
  state.presenterMuted = state.presenterVolume === 0;
  updatePresentationAudioControls();
  syncPresentationOutput();
}

function togglePresentationMute() {
  state.presenterMuted = !state.presenterMuted;
  if (!state.presenterMuted && state.presenterVolume === 0) state.presenterVolume = 0.5;
  updatePresentationAudioControls();
  syncPresentationOutput();
}

function stepPresentation(delta) {
  hidePresentationBoundaryNotice();
  if (delta > 0) return advancePresentation({ userInitiated: true });
  if (state.presenterAnimationTargetCursor != null) {
    const atFirstNavigationPosition = (state.presenterAnimationBatchStartCursor ?? state.presenterAnimationCursor) <= 0;
    cancelPresentationAnimationBatch();
    if (atFirstNavigationPosition) jumpPresentationPage(-1);
    return;
  }
  const animations = presentationAnimations();
  const previousCursor = previousAnimationNavigationCursor(animations, state.presenterAnimationCursor);
  if (previousCursor != null) {
    renderPresentation(false, {
      restoreCursor: previousCursor,
      runInitialAutomatic: false,
    });
    return;
  }
  jumpPresentationPage(-1);
}

function jumpPresentationPage(delta, options = {}) {
  hidePresentationBoundaryNotice();
  const direction = Math.sign(Number(delta) || 0);
  if (!direction) return;
  if (direction > 0) {
    advanceToNextPresentationSlide({ userInitiated: options.userInitiated !== false });
    return;
  }
  let target = state.presenterSlide - 1;
  if (target < state.presenterRangeStart) {
    if (state.presenterMode === "rehearse" || !state.slideShowSettings.loop) return;
    target = state.presenterRangeEnd;
  }
  recordRehearsalTiming();
  state.presenterSlide = target;
  const previousSlideAnimations = presentationAnimations();
  renderPresentation(true, {
    restoreCursor: previousSlideAnimations.length,
    runInitialAutomatic: false,
    reverseTransition: true,
  });
}

function reverseTransitionDirection(direction) {
  const normalized = String(direction || "left").toLowerCase();
  return ({
    l: "r", left: "right", r: "l", right: "left",
    u: "d", up: "down", d: "u", down: "up",
    in: "out", out: "in",
  })[normalized] || direction || "right";
}

function renderPresentation(playTransition = false, options = {}) {
  clearPresentationAutoPlayTimer();
  cancelPresentationFrameProbes(false);
  state.presenterTimers.forEach(clearTimeout);
  state.presenterTimers = [];
  state.presenterPlayers.forEach((player) => player.cancel());
  state.presenterPlayers = [];
  state.presenterAnimationTargetCursor = null;
  state.presenterAnimationBatchStartCursor = null;
  state.presenterPlaybackToken += 1;
  const stage = $("#presentStage");
  const slide = state.deck.slides[state.presenterSlide];
  state.presenterSlideStartedAt = presentationNow();
  clearSlideTransition(stage);
  const transitionUnderlay = playTransition && slide.transition
    ? capturePresentationTransitionUnderlay(stage)
    : null;
  globalThis.UniPptMedia?.stopAll(stage, true, true);
  stage.replaceChildren();
  stage.style.width = `${state.deck.width}px`;
  stage.style.height = `${state.deck.height}px`;
  applySlideBackground(stage, slide);
  fitPresentationStage();
  const mediaContext = { scope: "presentation", slideKey: slide.sourcePartName || slide.id };
  renderReadOnlySlide(slide, stage, mediaContext);
  syncPresentationOutput();
  const animations = presentationAnimations();
  resetAnimationTargets(stage, animations);
  if (playTransition && slide.transition) {
    const transition = options.reverseTransition
      ? { ...slide.transition, direction: reverseTransitionDirection(slide.transition.direction) }
      : slide.transition;
    animateSlideTransition(stage, transition, transitionUnderlay);
  } else {
    transitionUnderlay?.remove();
  }
  const restoreCursor = Math.max(0, Math.min(
    animations.length,
    Math.trunc(Number(options.restoreCursor) || 0),
  ));
  restorePresentationAnimationState(stage, animations, restoreCursor);
  state.presenterAnimationCursor = restoreCursor;
  if (
    !presentationAutoPlayEnabled()
    &&
    options.runInitialAutomatic !== false
    && restoreCursor === 0
    && animations.length
    && (animations[0].trigger || "onClick") !== "onClick"
  ) {
    runPresentationAnimationBatch(
      stage,
      animations,
      presentationTransitionLead(slide, playTransition),
    );
  }
  if (presentationAutoPlayEnabled()) {
    const transitionLead = presentationTransitionLead(slide, playTransition);
    schedulePresentationAutoPlay(presentationInitialAutoPlayDelay(
      slide,
      transitionLead,
      animations.length > 0,
    ));
  }
  updatePresentationCount(animations);
  updateRehearsalClock();
}

function namespaceTransitionCloneIds(root, label = "transition") {
  const prefix = `unippt-${label}-${++transitionCloneIdSequence}`;
  const escapeRegex = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const candidates = [root, ...root.querySelectorAll("[id]")];
  const replacements = new Map();
  for (const node of candidates) {
    const original = node.getAttribute?.("id");
    if (!original) continue;
    const safe = original.replace(/[^A-Za-z0-9_-]/g, "-");
    const next = `${prefix}-${safe}`;
    replacements.set(original, next);
    node.setAttribute("id", next);
  }
  if (!replacements.size) return root;
  for (const node of [root, ...root.querySelectorAll("*")]) {
    for (const attribute of [...(node.attributes || [])]) {
      let value = attribute.value;
      for (const [original, next] of replacements) {
        value = value
          .replaceAll(`url(#${original})`, `url(#${next})`)
          .replaceAll(`url("#${original}")`, `url("#${next}")`)
          .replaceAll(`url('#${original}')`, `url('#${next}')`);
        if (["begin", "end"].includes(attribute.name)) {
          const timingReference = new RegExp(`(^|[;\\s])${escapeRegex(original)}(?=\\.)`, "g");
          value = value.replace(timingReference, (_match, separator) => `${separator}${next}`);
        }
        if (value === `#${original}`) value = `#${next}`;
      }
      if (["aria-labelledby", "aria-describedby", "for"].includes(attribute.name)) {
        value = value.split(/\s+/).map((token) => replacements.get(token) || token).join(" ");
      }
      if (value !== attribute.value) node.setAttribute(attribute.name, value);
    }
  }
  for (const style of root.querySelectorAll("style")) {
    let css = style.textContent || "";
    for (const [original, next] of replacements) {
      css = css
        .replaceAll(`url(#${original})`, `url(#${next})`)
        .replaceAll(`url("#${original}")`, `url("#${next}")`)
        .replaceAll(`url('#${original}')`, `url('#${next}')`);
      const idSelector = new RegExp(`(^|[\\s,{>+~])#${escapeRegex(original)}(?=[\\s.{:[>+~,]|$)`, "g");
      css = css.replace(idSelector, (_match, separator) => `${separator}#${next}`);
    }
    style.textContent = css;
  }
  return root;
}

function capturePresentationTransitionUnderlay(stage) {
  if (globalThis.UniPptPresentationTransitions?.captureUnderlay) {
    return globalThis.UniPptPresentationTransitions.captureUnderlay(stage);
  }
  const presenter = stage.parentElement;
  if (!presenter) return null;
  presenter.querySelectorAll(":scope > .slide-transition-underlay").forEach((node) => node.remove());
  const underlay = stage.cloneNode(true);
  namespaceTransitionCloneIds(underlay, "underlay");
  underlay.classList.add("slide-transition-underlay");
  underlay.setAttribute("aria-hidden", "true");
  underlay.querySelectorAll("audio").forEach((node) => node.remove());
  underlay.querySelectorAll("video").forEach((node) => {
    node.autoplay = false;
    node.muted = true;
    node.removeAttribute("controls");
  });
  presenter.insertBefore(underlay, stage);
  return underlay;
}

function fitPresentationStage() {
  const presenter = $("#presenter");
  if (presenter.hidden || !state.deck) return;
  const stage = $("#presentStage");
  const slideWidth = Math.max(1, Number(state.deck.width) || 1);
  const slideHeight = Math.max(1, Number(state.deck.height) || 1);
  const viewportWidth = Math.max(1, presenter.clientWidth || window.innerWidth);
  const viewportHeight = Math.max(1, presenter.clientHeight || window.innerHeight);
  const scale = Math.max(0.001, Math.min(viewportWidth / slideWidth, viewportHeight / slideHeight));
  stage.style.width = `${slideWidth}px`;
  stage.style.height = `${slideHeight}px`;
  stage.style.transform = `translate3d(-50%, -50%, 0) scale(${scale})`;
}

function updatePresentationCount(animations = presentationAnimations()) {
  const count = $("#presentCount");
  count.textContent = `${state.presenterSlide + 1} / ${state.deck.slides.length}`;
  const navigationSteps = animationNavigationRanges(animations);
  const completedSteps = navigationSteps.filter((step) => step.endIndex <= state.presenterAnimationCursor).length;
  const detail = navigationSteps.length
    ? `，动画节点 ${completedSteps} / ${navigationSteps.length}`
    : "";
  count.title = `第 ${state.presenterSlide + 1} 张，共 ${state.deck.slides.length} 张${detail}`;
  count.setAttribute("aria-label", count.title);
}

function animateSlideTransition(stage, transition, underlay = null) {
  const duration = Math.max(1, Number(transition.durationMs) || 700);
  const kind = transition.kind || "fade";
  const normalizedKind = String(kind).toLowerCase();
  // Office's p15 Wind preset has no dir attribute and defaults to the old
  // sheet travelling toward the upper-right.  Do not inherit the generic
  // leftward default used by wipe/push transitions.
  const direction = transition.direction || (normalizedKind === "wind" ? "right" : "left");
  stage.dataset.transitionKind = normalizedKind;
  stage.dataset.transitionDurationMs = String(duration);
  if (globalThis.UniPptPresentationTransitions?.supports(normalizedKind)) {
    return runSlideTransition(stage, () => (
      globalThis.UniPptPresentationTransitions.create(stage, transition, underlay, {
        quality: presentationQualityTier(normalizedKind),
        slideIndex: stage.id === "slideStage" ? state.activeSlide : state.presenterSlide,
      })
    ), underlay);
  }
  let frames;
  if (normalizedKind === "randombar") {
    return animateRandomBarsTransition(stage, duration, direction, underlay);
  }
  if (normalizedKind === "reveal") {
    return animateRevealTransition(stage, duration, direction, underlay);
  }
  if (normalizedKind === "flash") {
    return animateFlashTransition(stage, duration, underlay);
  }
  if (normalizedKind === "pagecurldouble") {
    return animateTopDownBookTransition(stage, duration, direction, underlay, normalizedKind);
  }
  if (["pagecurl", "peeloff", "origami", "fallover"].includes(normalizedKind)) {
    return animatePageCurlTransition(stage, duration, direction, underlay, normalizedKind);
  }
  if (["ripple", "honeycomb", "glitter", "prism"].includes(normalizedKind)) {
    return animateRippleTransition(stage, duration, underlay, normalizedKind);
  }
  if (["wind", "warp", "vortex"].includes(normalizedKind)) {
    return animateWindTransition(stage, duration, direction, underlay, normalizedKind);
  }
  if (["curtains", "drape", "doors"].includes(normalizedKind)) {
    return animateCurtainTransition(stage, duration, underlay, normalizedKind);
  }
  if (["circle", "diamond", "plus", "wedge", "wheel"].includes(normalizedKind)) {
    const start = normalizedKind === "diamond" ? "polygon(50% 50%,50% 50%,50% 50%,50% 50%)" : "circle(0% at 50% 50%)";
    const end = normalizedKind === "diamond" ? "polygon(50% -50%,150% 50%,50% 150%,-50% 50%)" : "circle(75% at 50% 50%)";
    frames = [{ clipPath: start }, { clipPath: end }];
  } else if (["wipe", "push", "pull", "cover", "strips", "split", "blinds", "checker", "comb", "gallery", "conveyor", "switch", "pan", "ferris"].includes(normalizedKind)) {
    const hidden = direction === "r" || direction === "right" ? "inset(0 100% 0 0)" : direction === "u" || direction === "up" ? "inset(100% 0 0 0)" : direction === "d" || direction === "down" ? "inset(0 0 100% 0)" : "inset(0 0 0 100%)";
    frames = [{ clipPath: hidden, opacity: .45 }, { clipPath: "inset(0 0 0 0)", opacity: 1 }];
  } else if (normalizedKind === "cut" || normalizedKind === "none") {
    clearSlideTransition(stage);
    return null;
  } else {
    frames = [{ opacity: 0, filter: normalizedKind === "zoom" ? "blur(5px)" : "none" }, { opacity: 1, filter: "none" }];
  }
  return runSlideTransition(stage, () => {
    const player = stage.animate(frames, { duration, easing: "ease-out", fill: "both" });
    return { player, animations: [player] };
  }, underlay);
}

function buildPageCurlStripPlan(kind = "pagecurl", direction = "left", requestedCount = null) {
  const normalizedKind = String(kind).toLowerCase();
  const turnsRight = ["r", "right"].includes(String(direction).toLowerCase());
  const doubleCurl = normalizedKind === "pagecurldouble";
  // pageCurlDouble is a top-down open book.  Only the half-leaf beside the
  // centre spine moves, so 24 half-page strips have the same projected
  // density as 48 full-slide strips without duplicating the other page.
  const maximumCount = doubleCurl ? 24 : 20;
  const minimumCount = doubleCurl ? 16 : 14;
  const requestedNumber = Number(requestedCount);
  const count = requestedCount == null || !Number.isFinite(requestedNumber)
    ? maximumCount
    : clamp(Math.round(requestedNumber), minimumCount, maximumCount);
  const leafSpan = doubleCurl ? .5 : .28;
  const spine = doubleCurl ? .5 : (turnsRight ? leafSpan : 1 - leafSpan);
  const band = doubleCurl ? .5 : leafSpan;
  const strips = Array.from({ length: count }, (_, index) => {
    const width = leafSpan / count;
    const left = turnsRight
      ? spine - (index + 1) * width
      : spine + index * width;
    const backLeft = turnsRight
      ? spine + index * width
      : spine - (index + 1) * width;
    const center = left + width / 2;
    const curvature = Math.sin((index + .5) / count * Math.PI);
    return {
      index,
      left,
      backLeft,
      width,
      center,
      pageU: (index + .5) / count,
      // These values describe the continuous cylindrical sheet.  They are
      // deliberately moderate: rotating full-slide strips by nearly 180deg
      // collapses the projected curl into a dark seam instead of a page.
      angle: (turnsRight ? 1 : -1) * (22 + 54 * curvature),
      depth: 8 + (doubleCurl ? 58 : 34) * curvature,
      compression: 1 - .36 * curvature,
      faces: ["front", "back"],
      front: true,
      back: true,
      shadow: true,
    };
  });
  return {
    kind: normalizedKind,
    direction,
    turnsRight,
    doubleCurl,
    band,
    leafSpan,
    spine,
    topology: doubleCurl ? "top-down-open-book-even-leaf" : "single-leaf",
    shadow: true,
    strips,
  };
}

function pageCurlMonotoneValue(anchors, valueIndex, progress) {
  const count = anchors.length;
  if (!count) return 0;
  if (count === 1) return Number(anchors[0][valueIndex]) || 0;
  const p = clamp(Number(progress) || 0, anchors[0][0], anchors[count - 1][0]);
  const slopes = Array.from({ length: count - 1 }, (_, index) => {
    const dx = Math.max(.000001, anchors[index + 1][0] - anchors[index][0]);
    return (anchors[index + 1][valueIndex] - anchors[index][valueIndex]) / dx;
  });
  const tangents = Array(count).fill(0);
  tangents[0] = slopes[0];
  tangents[count - 1] = slopes[count - 2];
  for (let index = 1; index < count - 1; index += 1) {
    const before = slopes[index - 1];
    const after = slopes[index];
    if (before === 0 || after === 0 || Math.sign(before) !== Math.sign(after)) {
      tangents[index] = 0;
    } else {
      const leftWidth = anchors[index][0] - anchors[index - 1][0];
      const rightWidth = anchors[index + 1][0] - anchors[index][0];
      const weightA = 2 * rightWidth + leftWidth;
      const weightB = rightWidth + 2 * leftWidth;
      tangents[index] = (weightA + weightB) / (weightA / before + weightB / after);
    }
  }
  let interval = count - 2;
  for (let index = 0; index < count - 1; index += 1) {
    if (p <= anchors[index + 1][0]) {
      interval = index;
      break;
    }
  }
  const start = anchors[interval];
  const end = anchors[interval + 1];
  const width = Math.max(.000001, end[0] - start[0]);
  const t = clamp((p - start[0]) / width, 0, 1);
  const t2 = t * t;
  const t3 = t2 * t;
  return (2 * t3 - 3 * t2 + 1) * start[valueIndex]
    + (t3 - 2 * t2 + t) * width * tangents[interval]
    + (-2 * t3 + 3 * t2) * end[valueIndex]
    + (t3 - t2) * width * tangents[interval + 1];
}

function pageCurlGeometryAt(progress, kind = "pagecurl", direction = "left") {
  const normalizedKind = String(kind).toLowerCase();
  const turnsRight = ["r", "right"].includes(String(direction).toLowerCase());
  const doubleCurl = normalizedKind === "pagecurldouble";
  const anchors = doubleCurl
    ? [
        // [time, trailing hinge, turn angle / PI].  These samples come from
        // the native 1.25s recording.  The right leaf rises around a fixed
        // centre spine before the hinge itself starts travelling left.
        [0, .5, 0], [.22, .5, 0], [.333, .5, .1], [.444, .5, .3],
        [.556, .44, .44], [.667, .27, .9], [.778, .11, .98],
        [.889, 0, 1], [1, 0, 1],
      ]
    : [
        [0, .72, 0], [.18, .72, 0], [.42, .58, .34], [.64, .34, .7],
        [.84, .08, .96], [1, 0, 1],
      ];
  const p = clamp(Number(progress) || 0, 0, 1);
  // Monotone cubic interpolation preserves the sampled PowerPoint path but
  // removes the visible velocity corners of piecewise-linear interpolation.
  const hinge = pageCurlMonotoneValue(anchors, 1, p);
  const theta = pageCurlMonotoneValue(anchors, 2, p) * Math.PI;
  const materialSpan = Math.max(.0001, 1 - hinge);
  const radius = theta > .00001 ? materialSpan / theta : materialSpan;
  const radiusScale = 1 - .1 * Math.pow(theta / Math.PI, 2);
  const freeEdge = theta > .00001
    ? hinge + radius * radiusScale * Math.sin(theta)
    : 1;
  const crest = theta >= Math.PI / 2
    ? hinge + radius * radiusScale
    : freeEdge;
  const logicalLeft = Math.min(hinge, crest);
  const logicalRight = Math.max(hinge, crest);
  const physicalLeft = turnsRight ? 1 - logicalRight : logicalLeft;
  const physicalRight = turnsRight ? 1 - logicalLeft : logicalRight;
  const bandWidth = Math.max(0, physicalRight - physicalLeft);
  const liftT = clamp((p - .22) / .113, 0, 1);
  const leafLift = liftT * liftT * (3 - 2 * liftT);
  const physicalHinge = turnsRight ? 1 - hinge : hinge;
  const flatBoundary = turnsRight
    ? leafLift * physicalHinge
    : 1 - leafLift * (1 - physicalHinge);
  return {
    progress: p,
    turnsRight,
    doubleCurl,
    hinge,
    theta,
    radius,
    radiusScale,
    fixedSpine: .5,
    leafLift,
    leafOpacity: leafLift * (p <= .889 ? 1 : clamp((1 - p) / .111, 0, 1)),
    freeEdge,
    crest,
    sheetOpacity: p <= .889 ? 1 : clamp((1 - p) / .111, 0, 1),
    flatBoundary,
    hingeBoundary: physicalHinge,
    outerBoundary: turnsRight ? 1 - crest : crest,
    bandLeft: physicalLeft,
    bandRight: physicalRight,
    bandWidth,
  };
}

function pageCurlFlatClipPolygon(geometry) {
  const boundary = clamp(geometry.flatBoundary * 100, 0, 100).toFixed(3);
  return geometry.turnsRight
    ? `polygon(${boundary}% 0,100% 0,100% 100%,${boundary}% 100%)`
    : `polygon(0 0,${boundary}% 0,${boundary}% 100%,0 100%)`;
}

function pageCurlStripPoseAt(strip, geometry, stageWidth) {
  if (geometry.theta <= .00001) {
    return { active: false, angle: 0, scaleX: 1, x: strip.center, z: 0, brightness: 1 };
  }
  if (strip.pageU == null) {
    const outsideMovingLeaf = geometry.turnsRight
      ? strip.center - strip.width / 2 >= .5 - 1e-7
      : strip.center + strip.width / 2 <= .5 + 1e-7;
    if (outsideMovingLeaf) {
      return { active: false, angle: 0, scaleX: 1, x: strip.center, z: 0, brightness: 1 };
    }
  }
  // Material always comes from the page beside the fixed centre spine.  The
  // travelling `hinge` is the projected crease, not a source-texture cutoff.
  const fallbackU = geometry.turnsRight
    ? (.5 - strip.center) / .5
    : (strip.center - .5) / .5;
  const u = clamp(Number(strip.pageU ?? fallbackU), 0, 1);
  const phi = u * geometry.theta;
  const mappedLogical = geometry.theta > .00001
    ? geometry.hinge + geometry.radius * geometry.radiusScale * Math.sin(phi)
    : strip.center;
  const mappedPhysical = geometry.turnsRight ? 1 - mappedLogical : mappedLogical;
  const z = geometry.theta > .00001
    ? .4 * geometry.radius * stageWidth * (1 - Math.cos(phi))
    : 0;
  const tangentX = geometry.radiusScale * Math.cos(phi);
  const tangentZ = .4 * Math.sin(phi);
  const tangentAngle = Math.atan2(tangentZ, tangentX);
  const tangentScale = Math.max(.08, Math.hypot(tangentX, tangentZ));
  return {
    active: true,
    angle: (geometry.turnsRight ? 1 : -1) * tangentAngle * 180 / Math.PI,
    scaleX: tangentScale,
    x: mappedPhysical,
    z,
    brightness: 1 - .16 * Math.sin(tangentAngle),
  };
}

function createSharedTransitionTexture(underlay, host, stageWidth, stageHeight, label) {
  const runtime = globalThis.UniPptTransitionTexture;
  if (!runtime?.create || !underlay || !host) return null;
  try {
    const texture = runtime.create(underlay, {
      host,
      width: stageWidth,
      height: stageHeight,
      prepareClone(clone) {
        clone.classList.remove("slide-transition-underlay");
        clone.classList.add("shared-transition-texture-source");
      },
    });
    if (texture?.registry) texture.registry.dataset.transitionTextureFor = label;
    return texture;
  } catch (_error) {
    return null;
  }
}

function makePageCurlStripFace(source, strip, stageWidth, stageHeight, face, sharedTexture = null) {
  const frontFace = face !== "back";
  const surface = document.createElement("div");
  surface.className = `page-curl-strip-face ${frontFace ? "page-curl-strip-front" : "page-curl-strip-back"}`;
  surface.dataset.pageCurlFace = frontFace ? "front" : "back";
  const textureLeft = frontFace ? strip.left : strip.backLeft;
  let texture = sharedTexture?.createHorizontalSlice({
    x: textureLeft * stageWidth,
    width: strip.width * stageWidth,
    className: `page-curl-texture page-curl-shared-texture ${frontFace ? "page-curl-front-texture" : "page-curl-back-texture"}`,
    dataset: { pageCurlTexture: strip.index, pageCurlTextureFace: frontFace ? "front" : "back" },
    style: {
      position: "absolute", inset: "0", width: "100%", height: "100%",
      transform: "none", transformOrigin: "0 50%", zIndex: "1", willChange: "auto",
    },
  });
  if (!texture) {
    texture = source.cloneNode(true);
    namespaceTransitionCloneIds(texture, "curl");
    texture.classList.remove("slide-transition-underlay");
    texture.classList.add("page-curl-texture", frontFace ? "page-curl-front-texture" : "page-curl-back-texture");
    texture.querySelectorAll("audio,video").forEach((node) => node.remove());
    Object.assign(texture.style, {
      inset: "auto", left: `${(-textureLeft * stageWidth).toFixed(3)}px`, top: "0",
      width: `${stageWidth}px`, height: `${stageHeight}px`, transform: "none",
      transformOrigin: "0 50%", clipPath: "none", visibility: "visible", zIndex: "1", willChange: "auto",
    });
  }
  surface.append(texture);
  return surface;
}

function makeTopDownBookLeafFace(source, stageWidth, stageHeight, face, sourceLeft) {
  const isBack = face === "back";
  const surface = document.createElement("div");
  surface.className = `page-book-leaf-face page-book-leaf-${isBack ? "back" : "front"}`;
  surface.dataset.pageBookFace = isBack ? "incoming-left-page" : "outgoing-even-page";
  const texture = source.cloneNode(true);
  namespaceTransitionCloneIds(texture, `book-${face}`);
  texture.classList.remove("slide-transition-underlay");
  texture.classList.add("page-book-leaf-texture");
  texture.querySelectorAll("audio,video").forEach((node) => node.remove());
  Object.assign(texture.style, {
    position: "absolute",
    inset: "auto",
    left: `${(-sourceLeft).toFixed(3)}px`,
    top: "0",
    width: `${stageWidth}px`,
    height: `${stageHeight}px`,
    transform: "none",
    transformOrigin: "0 0",
    clipPath: "none",
    visibility: "visible",
    zIndex: "1",
    willChange: "auto",
  });
  surface.append(texture);
  return surface;
}

function topDownBookLeafAngle(geometry) {
  const projected = clamp(Math.abs(geometry.freeEdge - geometry.hinge) / .5, 0, 1);
  const risingAngle = Math.acos(projected) * 180 / Math.PI;
  if (geometry.progress <= .778) return risingAngle;
  const landingT = clamp((geometry.progress - .778) / .222, 0, 1);
  const smooth = landingT * landingT * (3 - 2 * landingT);
  return risingAngle + (180 - risingAngle) * smooth;
}

function topDownBookSegmentPoses(geometry, segmentCount, stageWidth, turnsRight) {
  const count = Math.max(1, Math.round(Number(segmentCount) || 1));
  const width = stageWidth * .5 / count;
  const pageAngle = topDownBookLeafAngle(geometry);
  const curlProgress = clamp((geometry.progress - .22) / .78, 0, 1);
  const curlAmount = Math.sin(curlProgress * Math.PI) * 58;
  const directionSign = turnsRight ? -1 : 1;
  let chainX = 0;
  let chainZ = 0;
  return Array.from({ length: count }, (_, index) => {
    const pageU = (index + .5) / count;
    const segmentAngle = clamp(pageAngle + curlAmount * (pageU - .5) * 1.15, 0, 180);
    const originalOriginX = index * width;
    const dx = directionSign * (chainX - originalOriginX);
    const pose = {
      index,
      angle: (turnsRight ? 1 : -1) * segmentAngle,
      dx,
      z: chainZ,
      width,
    };
    chainX += width * Math.cos(segmentAngle * Math.PI / 180);
    chainZ += width * Math.sin(segmentAngle * Math.PI / 180);
    return pose;
  });
}

function topDownBookLeafFrames(direction = "left") {
  const turnsRight = ["r", "right"].includes(String(direction).toLowerCase());
  const sign = turnsRight ? 1 : -1;
  // Sampled from PowerPoint's 1.25s pageCurlDouble recording.  Keeping one
  // coherent leaf is important here: the native preset is a nearly rigid
  // sheet around the centre spine, with only a very small free-edge curl.
  const samples = [
    [0, 0], [.08, 4], [.18, 16], [.3, 37], [.42, 66], [.5, 90],
    [.6, 119], [.72, 149], [.84, 169], [.94, 178], [1, 180],
  ];
  return samples.map(([offset, angle]) => {
    const radians = angle * Math.PI / 180;
    // Grazing light: nothing at rest, strongest with the sheet on its edge.
    const grazing = Math.sin(radians);
    // Which physical side the room actually sees at this angle.
    const facing = Math.max(0, Math.cos(radians));
    const away = Math.max(0, -Math.cos(radians));
    return {
      offset,
      angle,
      transform: `rotateY(${(sign * angle).toFixed(3)}deg)`,
      shadeOpacity: (.26 * grazing).toFixed(4),
      shadowOpacity: (.22 * grazing).toFixed(4),
      // A lifting page catches a highlight along its free edge, and that band
      // travels toward the spine as the sheet stands up. Sweeping one gradient
      // with a transform keeps the effect on the compositor, unlike repainting
      // gradient stops every frame.
      frontShade: (.34 * grazing * (.4 + .6 * facing)).toFixed(4),
      frontSlide: (sign * -32 * grazing * facing).toFixed(3),
      // The reverse stays dark until it has swung past vertical into view.
      backShade: (.3 * grazing * (.4 + .6 * away)).toFixed(4),
      backSlide: (sign * 32 * grazing * away).toFixed(3),
      // Chromium drops backface-visibility on overflow/contain faces, so cull
      // with the same facing term instead of relying on the compositor.
      frontOpacity: clamp((facing - .02) / .12, 0, 1).toFixed(4),
      backOpacity: clamp((away - .02) / .12, 0, 1).toFixed(4),
      // A sheet standing on its edge casts a narrow shadow; it broadens again
      // as the page falls flat on the far side.
      shadowScale: (.34 + .66 * Math.abs(Math.cos(radians))).toFixed(4),
    };
  });
}

function synchronizeTransitionAnimations(animations) {
  const timelineNow = Number(document?.timeline?.currentTime);
  if (!Number.isFinite(timelineNow)) return null;
  // WAAPI animations created in a loop can otherwise start a few milliseconds
  // apart.  A shared compositor timestamp keeps physical mesh seams locked.
  const sharedStartTime = timelineNow + 1;
  for (const animation of animations || []) {
    try {
      animation.startTime = sharedStartTime;
    } catch (_error) {
      // Older embedded WebViews may expose a read-only startTime.  They still
      // create all players in one task, which is the best available fallback.
    }
  }
  return sharedStartTime;
}

function animateTopDownBookTransition(stage, duration, direction, underlay = null, kind = "pagecurldouble") {
  return runSlideTransition(stage, () => {
    if (!underlay || !stage.parentElement) {
      const player = stage.animate([
        { opacity: .35, transformOrigin: "50% 50%", transform: `${stage.style.transform || ""} rotateY(-12deg)` },
        { opacity: 1, transform: stage.style.transform || "" },
      ], { duration, easing: "linear", fill: "both" });
      return { player, animations: [player] };
    }

    const presenter = stage.parentElement;
    const turnsRight = ["r", "right"].includes(String(direction).toLowerCase());
    const baseTransform = underlay.style.transform || stage.style.transform || "";
    const stageWidth = Math.max(1, Number(stage.clientWidth) || parseFloat(stage.style.width) || 1280);
    const stageHeight = Math.max(1, Number(stage.clientHeight) || parseFloat(stage.style.height) || 720);
    const textureComplexity = underlay.querySelectorAll("*").length;
    const quality = presentationQualityTier(`slide-transition:${kind}`);
    const samples = topDownBookLeafFrames(direction);

    stage.style.zIndex = "1";
    underlay.style.zIndex = "2";
    underlay.style.clipPath = turnsRight ? "inset(0 0 0 50%)" : "inset(0 50% 0 0)";
    underlay.style.willChange = "auto";

    const bookFrame = document.createElement("div");
    bookFrame.className = "page-book-frame";
    bookFrame.dataset.pageCurlTopology = "top-down-open-book-even-leaf";
    bookFrame.dataset.pageCurlSurface = "coherent-two-sided-half-page";
    bookFrame.dataset.pageCurlSpine = "fixed-50-percent";
    Object.assign(bookFrame.style, {
      left: "50%", top: "50%", width: `${stageWidth}px`, height: `${stageHeight}px`,
      transform: baseTransform,
      perspective: `${Math.max(1600, stageWidth * 1.65).toFixed(0)}px`,
    });

    const leaf = document.createElement("div");
    leaf.className = "page-book-turning-leaf";
    leaf.dataset.pageCurlStripCount = "1";
    leaf.dataset.pageCurlLeaf = "coherent-sheet";
    Object.assign(leaf.style, {
      left: turnsRight ? "0" : "50%",
      top: "0",
      width: "50%",
      height: "100%",
      transformOrigin: turnsRight ? "100% 50%" : "0 50%",
    });
    const frontSourceLeft = turnsRight ? 0 : stageWidth * .5;
    const backSourceLeft = turnsRight ? stageWidth * .5 : 0;
    const front = makeTopDownBookLeafFace(underlay, stageWidth, stageHeight, "front", frontSourceLeft);
    const back = makeTopDownBookLeafFace(stage, stageWidth, stageHeight, "back", backSourceLeft);
    const frontLighting = document.createElement("div");
    frontLighting.className = "page-book-leaf-lighting page-book-leaf-front-lighting";
    const backLighting = document.createElement("div");
    backLighting.className = "page-book-leaf-lighting page-book-leaf-back-lighting";
    front.append(frontLighting);
    back.append(backLighting);
    leaf.append(front, back);

    const spine = document.createElement("div");
    spine.className = "page-book-spine";
    Object.assign(spine.style, { left: `calc(50% - 7px)`, top: "0", height: "100%" });
    bookFrame.append(spine, leaf);
    presenter.insertBefore(bookFrame, stage.nextSibling);

    const leafAnimation = leaf.animate(samples.map((sample) => ({
      offset: sample.offset,
      opacity: 1,
      transform: sample.transform,
    })), { duration, easing: "linear", fill: "both" });
    const frontFaceAnimation = front.animate(samples.map((sample) => ({
      offset: sample.offset,
      opacity: sample.frontOpacity,
    })), { duration, easing: "linear", fill: "both" });
    const backFaceAnimation = back.animate(samples.map((sample) => ({
      offset: sample.offset,
      opacity: sample.backOpacity,
    })), { duration, easing: "linear", fill: "both" });
    const frontLightAnimation = frontLighting.animate(samples.map((sample) => ({
      offset: sample.offset,
      opacity: sample.frontShade,
      transform: `translate3d(${sample.frontSlide}%,0,0)`,
    })), { duration, easing: "linear", fill: "both" });
    const backLightAnimation = backLighting.animate(samples.map((sample) => ({
      offset: sample.offset,
      opacity: sample.backShade,
      transform: `translate3d(${sample.backSlide}%,0,0)`,
    })), { duration, easing: "linear", fill: "both" });
    const spineAnimation = spine.animate(samples.map((sample) => ({
      offset: sample.offset,
      opacity: Math.min(.58, Number(sample.shadowOpacity) * 2.25).toFixed(4),
    })), { duration, easing: "linear", fill: "both" });

    const castShadowFrame = document.createElement("div");
    castShadowFrame.className = "page-curl-shadow-frame page-curl-cast-frame page-book-shadow-frame";
    const castShadow = document.createElement("div");
    castShadow.className = "page-curl-moving-shadow page-book-free-edge-shadow";
    if (turnsRight) castShadow.classList.add("turns-right");
    castShadowFrame.append(castShadow);
    Object.assign(castShadowFrame.style, {
      left: "50%", top: "50%", width: `${stageWidth}px`, height: `${stageHeight}px`,
      transform: baseTransform,
    });
    presenter.insertBefore(castShadowFrame, stage.nextSibling);
    const castAnimation = castShadow.animate(samples.map((sample) => {
      const radians = sample.angle * Math.PI / 180;
      const edgeX = stageWidth * .5 + (turnsRight ? -1 : 1) * stageWidth * .5 * Math.cos(radians);
      return {
        offset: sample.offset,
        opacity: sample.shadowOpacity,
        transform: `translate3d(${(edgeX - 48).toFixed(3)}px,0,0) scaleX(${sample.shadowScale})`,
      };
    }), { duration, easing: "linear", fill: "both" });

    const animations = [
      leafAnimation, frontFaceAnimation, backFaceAnimation,
      frontLightAnimation, backLightAnimation, spineAnimation, castAnimation,
    ];
    synchronizeTransitionAnimations(animations);
    return {
      player: leafAnimation,
      animations,
      metrics: { quality, layers: 1, nodeCount: textureComplexity, coherentLeaf: true },
      cleanup: () => {
        bookFrame.remove();
        castShadowFrame.remove();
        stage.style.zIndex = "1";
      },
    };
  }, underlay);
}

function animatePageCurlTransition(stage, duration, direction, underlay = null, kind = "pagecurl") {
  return runSlideTransition(stage, () => {
    if (!underlay) {
      const player = stage.animate([
        { clipPath: "polygon(0 0,100% 0,100% 100%,0 100%)", filter: "brightness(.82)" },
        { clipPath: "polygon(0 0,8% 5%,0 100%,0 100%)", filter: "brightness(1)" },
      ], { duration, easing: "cubic-bezier(.35,.02,.2,1)", fill: "both" });
      return { player, animations: [player] };
    }

    const presenter = stage.parentElement;
    const baseTransform = underlay.style.transform || stage.style.transform || "";
    const stageWidth = Math.max(1, Number(stage.clientWidth) || parseFloat(stage.style.width) || 1);
    const stageHeight = Math.max(1, Number(stage.clientHeight) || parseFloat(stage.style.height) || 1);
    const doubleCurl = String(kind).toLowerCase() === "pagecurldouble";
    const outgoingTexture = createSharedTransitionTexture(
      underlay,
      presenter,
      stageWidth,
      stageHeight,
      "page-curl-outgoing",
    );
    const incomingTexture = createSharedTransitionTexture(
      stage,
      presenter,
      stageWidth,
      stageHeight,
      "page-curl-incoming",
    );
    const sceneTextureMode = Boolean(
      outgoingTexture?.createSceneHorizontalSlice
      && incomingTexture?.createSceneHorizontalSlice,
    );
    const textureComplexity = underlay.querySelectorAll("*").length;
    const quality = presentationQualityTier(`slide-transition:${kind}`);
    const adaptiveCount = sceneTextureMode
      ? (doubleCurl ? 24 : 20)
      : textureComplexity > 0
      ? Math.floor((doubleCurl ? 1300 : 900) / textureComplexity)
      : null;
    const qualityCap = quality === "low" ? 14 : quality === "medium" ? 16 : null;
    const plan = buildPageCurlStripPlan(
      kind,
      direction,
      qualityCap == null ? adaptiveCount : Math.min(adaptiveCount ?? qualityCap, qualityCap),
    );
    underlay.style.visibility = "hidden";
    stage.style.zIndex = "1";
    if (sceneTextureMode) {
      outgoingTexture.mountScene({
        className: "page-curl-texture-scene page-curl-outgoing-scene",
        dataset: { pageCurlScene: "outgoing-front" },
        style: {
          position: "absolute", left: "50%", top: "50%", transform: baseTransform,
          transformOrigin: "50% 50%", zIndex: "3", overflow: "visible", pointerEvents: "none",
        },
      });
      incomingTexture.mountScene({
        className: "page-curl-texture-scene page-curl-incoming-scene",
        dataset: { pageCurlScene: "incoming-back" },
        style: {
          position: "absolute", left: "50%", top: "50%", transform: baseTransform,
          transformOrigin: "50% 50%", zIndex: "4", overflow: "visible", pointerEvents: "none",
        },
      });
    }

    // PowerPoint's 1.25s native recording has 9 topology anchors.  Four
    // midpoints give the compositor enough material around the moving crest
    // while monotone cubic geometry keeps velocity continuous between them.
    const sampleOffsets = [0, .11, .18, .22, .278, .333, .389, .444, .5, .556, .612, .667, .722, .778, .834, .889, 1];
    const samples = sampleOffsets.map((offset) => {
      return { offset, geometry: pageCurlGeometryAt(offset, kind, direction) };
    });

    let oldFlat = sceneTextureMode ? outgoingTexture.createSceneFullPage({
      className: "page-curl-old-flat page-curl-shared-texture",
      dataset: { pageCurlSurface: "outgoing-flat-left-page" },
      style: { willChange: "clip-path" },
    }) : outgoingTexture?.createFullPage({
      className: "page-curl-old-flat page-curl-shared-texture",
      dataset: { pageCurlSurface: "outgoing-flat-left-page" },
    });
    if (!oldFlat) {
      oldFlat = underlay.cloneNode(true);
      namespaceTransitionCloneIds(oldFlat, "curl-flat");
      oldFlat.classList.remove("slide-transition-underlay");
      oldFlat.classList.add("page-curl-old-flat");
      oldFlat.querySelectorAll("audio,video").forEach((node) => node.remove());
    }
    if (!sceneTextureMode) {
      Object.assign(oldFlat.style, {
        position: "absolute", left: "50%", top: "50%", width: stage.style.width,
        height: stage.style.height, transform: baseTransform, transformOrigin: "50% 50%",
        visibility: "visible", zIndex: "2", willChange: "clip-path",
      });
      presenter.insertBefore(oldFlat, stage.nextSibling);
    }
    const player = oldFlat.animate(samples.map(({ offset, geometry }) => ({
      offset,
      clipPath: pageCurlFlatClipPolygon(geometry),
    })), { duration, easing: "linear", fill: "both" });

    const mesh = document.createElement("div");
    mesh.className = "page-curl-mesh";
    if (plan.turnsRight) mesh.classList.add("turns-right");
    mesh.dataset.pageCurlSurface = plan.doubleCurl ? "cylindrical-double-book-leaf" : "cylindrical-book-leaf";
    mesh.dataset.pageCurlTopology = plan.topology;
    mesh.dataset.pageCurlStripCount = String(plan.strips.length);
    Object.assign(mesh.style, {
      left: "50%", top: "50%", width: stage.style.width, height: stage.style.height,
      transform: baseTransform,
      perspective: `${Math.max(900, stageWidth * 1.4).toFixed(0)}px`,
    });
    presenter.insertBefore(mesh, stage.nextSibling);

    const stripNodes = [];
    const animations = [player];
    for (const strip of plan.strips) {
      if (sceneTextureMode) {
        const displayLeft = strip.left * stageWidth;
        const displayWidth = strip.width * stageWidth + .8;
        const front = outgoingTexture.createSceneHorizontalSlice({
          x: strip.left * stageWidth,
          width: strip.width * stageWidth,
          left: displayLeft,
          displayWidth,
          className: "page-curl-scene-strip page-curl-front-texture",
          dataset: { pageCurlStrip: strip.index, pageCurlFace: "front" },
          style: { willChange: "transform,opacity", transformOrigin: "50% 50%" },
        });
        const back = incomingTexture.createSceneHorizontalSlice({
          x: strip.backLeft * stageWidth,
          width: strip.width * stageWidth,
          left: displayLeft,
          displayWidth,
          className: "page-curl-scene-strip page-curl-back-texture",
          dataset: { pageCurlStrip: strip.index, pageCurlFace: "back" },
          style: { willChange: "transform,opacity", transformOrigin: "50% 50%" },
        });
        stripNodes.push(front, back);
        const stripFrames = samples.map(({ offset, geometry }) => {
          const pose = pageCurlStripPoseAt(strip, geometry, stageWidth);
          const sourceX = strip.center * stageWidth;
          const dx = pose.x * stageWidth - sourceX;
          const backMix = clamp((Math.abs(pose.angle) - 82) / 18, 0, 1);
          const alpha = pose.active ? geometry.leafOpacity : 0;
          const transform = `translate(${dx.toFixed(3)}px,0) scaleX(${pose.scaleX.toFixed(4)})`;
          return {
            offset,
            transform,
            frontOpacity: (alpha * (1 - backMix)).toFixed(4),
            backOpacity: (alpha * backMix).toFixed(4),
          };
        });
        animations.push(front.animate(stripFrames.map((frame) => ({
          offset: frame.offset,
          opacity: frame.frontOpacity,
          transform: frame.transform,
        })), { duration, easing: "linear", fill: "both" }));
        animations.push(back.animate(stripFrames.map((frame) => ({
          offset: frame.offset,
          opacity: frame.backOpacity,
          transform: frame.transform,
        })), { duration, easing: "linear", fill: "both" }));
        continue;
      }
      const shell = document.createElement("div");
      shell.className = "page-curl-mesh-strip";
      shell.dataset.pageCurlStrip = String(strip.index);
      shell.dataset.pageCurlFaces = strip.faces.join(",");
      Object.assign(shell.style, {
        left: `${(strip.left * 100).toFixed(5)}%`, top: "0",
        width: `calc(${(strip.width * 100).toFixed(5)}% + .8px)`, height: "100%",
      });
      shell.append(
        makePageCurlStripFace(underlay, strip, stageWidth, stageHeight, "front", null),
        makePageCurlStripFace(stage, strip, stageWidth, stageHeight, "back", null),
      );
      mesh.append(shell);
      stripNodes.push(shell);
      animations.push(shell.animate(samples.map(({ offset, geometry }) => {
        const pose = pageCurlStripPoseAt(strip, geometry, stageWidth);
        const sourceX = strip.center * stageWidth;
        const dx = (pose.x * stageWidth) - sourceX;
        return {
          offset,
          opacity: String(pose.active ? geometry.leafOpacity : 0),
          transform: `translate3d(${dx.toFixed(3)}px,0,${pose.z.toFixed(3)}px) rotateY(${pose.angle.toFixed(3)}deg) scaleX(${pose.scaleX.toFixed(4)})`,
        };
      }), { duration, easing: "linear", fill: "both" }));
    }

    const foldShadowFrame = document.createElement("div");
    foldShadowFrame.className = "page-curl-shadow-frame page-curl-fold-frame";
    const foldShadow = document.createElement("div");
    foldShadow.className = "page-curl-moving-shadow page-curl-fold-shadow";
    if (plan.turnsRight) foldShadow.classList.add("turns-right");
    foldShadow.dataset.pageCurlShadow = "fixed-fold-shadow";
    foldShadowFrame.append(foldShadow);
    const castShadowFrame = document.createElement("div");
    castShadowFrame.className = "page-curl-shadow-frame page-curl-cast-frame";
    const castShadow = document.createElement("div");
    castShadow.className = "page-curl-moving-shadow page-curl-cast-shadow";
    if (plan.turnsRight) castShadow.classList.add("turns-right");
    castShadow.dataset.pageCurlShadow = "moving-page-shadow";
    castShadowFrame.append(castShadow);
    for (const frame of [foldShadowFrame, castShadowFrame]) {
      Object.assign(frame.style, {
        left: "50%", top: "50%", width: stage.style.width, height: stage.style.height,
        transform: baseTransform,
      });
      presenter.insertBefore(frame, stage.nextSibling);
    }

    animations.push(foldShadow.animate(samples.map(({ offset, geometry }) => ({
      offset,
      opacity: geometry.theta > .02 ? Math.min(.58, geometry.theta / Math.PI * .72) * geometry.sheetOpacity : 0,
      transform: `translateX(${(geometry.hingeBoundary * stageWidth - 28).toFixed(3)}px)`,
    })), { duration, easing: "linear", fill: "both" }));
    animations.push(castShadow.animate(samples.map(({ offset, geometry }) => ({
      offset,
      opacity: geometry.theta > .08 ? Math.min(.42, geometry.theta / Math.PI * .48) * geometry.sheetOpacity : 0,
      transform: `translateX(${(geometry.outerBoundary * stageWidth - 46).toFixed(3)}px)`,
    })), { duration, easing: "linear", fill: "both" }));

    return {
      player,
      animations,
      metrics: {
        quality,
        layers: plan.strips.length,
        nodeCount: textureComplexity,
        sharedTexture: sceneTextureMode,
      },
      cleanup: () => {
        stripNodes.forEach((node) => node.remove());
        mesh.remove();
        oldFlat.remove();
        foldShadowFrame.remove();
        castShadowFrame.remove();
        outgoingTexture?.cleanup();
        incomingTexture?.cleanup();
        stage.style.zIndex = "1";
      },
    };
  }, underlay);
}

function animateRippleTransition(stage, duration, underlay = null, kind = "ripple") {
  return runSlideTransition(stage, () => {
    if (!underlay || !stage.parentElement) {
      const player = stage.animate([
        { opacity: 0, clipPath: "circle(0% at 50% 50%)", transformOrigin: "50% 50%" },
        { offset: .58, opacity: .86, clipPath: "circle(46% at 50% 50%)" },
        { opacity: 1, clipPath: "circle(76% at 50% 50%)" },
      ], { duration, easing: "linear", fill: "both" });
      return { player, animations: [player] };
    }

    // PowerPoint's Ripple is one coherent wavefront: the new page is exposed
    // inside a thick ring while the old page remains outside it.  Keep the
    // ring to three adjacent bands so the browser composites transforms and
    // masks without the old eight-layer blur stack.  A texture provider can
    // share their source; the built-in fallback remains bounded to 3 clones.
    const presenter = stage.parentElement;
    const baseTransform = underlay.style.transform || stage.style.transform || "";
    const stageWidth = Math.max(1, parseFloat(stage.style.width || underlay.style.width) || stage.clientWidth || 1280);
    const stageHeight = Math.max(1, parseFloat(stage.style.height || underlay.style.height) || stage.clientHeight || 720);
    const sharedTexture = createSharedTransitionTexture(
      underlay,
      presenter,
      stageWidth,
      stageHeight,
      `ripple-${kind}`,
    );
    const sceneTextureMode = Boolean(sharedTexture?.createSceneFullPage);
    const quality = presentationQualityTier(`slide-transition:${kind}`);
    const layerCount = quality === "low" ? 1 : quality === "medium" ? 2 : 3;
    const layers = [];
    const animations = [];
    underlay.style.zIndex = "2";
    underlay.style.transformOrigin = "50% 50%";
    underlay.style.willChange = "transform,opacity";
    stage.style.zIndex = "3";
    stage.style.transformOrigin = "50% 50%";
    stage.style.willChange = "clip-path,opacity";
    if (sceneTextureMode) {
      sharedTexture.mountScene({
        className: "ripple-texture-scene",
        dataset: { rippleScene: kind },
        style: {
          position: "absolute", left: "50%", top: "50%", transform: baseTransform,
          transformOrigin: "50% 50%", zIndex: "4", overflow: "visible", pointerEvents: "none",
        },
      });
    }

    const player = underlay.animate([
      { opacity: 1, transform: baseTransform },
      { offset: .7, opacity: 1, transform: `${baseTransform} scale(1.006)` },
      { opacity: 0, transform: `${baseTransform} scale(1.018)` },
    ], { duration, easing: "linear", fill: "both" });
    animations.push(player);

    for (let index = 0; index < layerCount; index += 1) {
      let layer = sceneTextureMode ? sharedTexture.createSceneFullPage({
        className: "ripple-refraction-layer",
        dataset: { rippleRefraction: String(index) },
      }) : sharedTexture?.createFullPage({
        className: "ripple-refraction-layer",
        dataset: { rippleRefraction: String(index) },
      });
      if (!layer) {
        layer = underlay.cloneNode(true);
        namespaceTransitionCloneIds(layer, "ripple");
        layer.classList.remove("slide-transition-underlay");
      }
      layer.classList.add("ripple-refraction-layer");
      layer.dataset.rippleRefraction = String(index);
      layer.querySelectorAll("audio,video").forEach((node) => node.remove());
      const polarity = index - (layerCount - 1) / 2;
      const layerBaseTransform = sceneTextureMode ? "" : baseTransform;
      Object.assign(layer.style, {
        left: sceneTextureMode ? "" : "50%",
        top: sceneTextureMode ? "" : "50%",
        width: sceneTextureMode ? "" : `${stageWidth}px`,
        height: sceneTextureMode ? "" : `${stageHeight}px`,
        zIndex: "4",
        transform: layerBaseTransform,
        transformOrigin: "50% 50%",
        "--ripple-band-offset": `${polarity * 1.45}%`,
        "--ripple-band-width": kind === "prism" ? "8.5%" : "7%",
        willChange: "transform,opacity",
      });
      if (!sceneTextureMode) presenter.insertBefore(layer, stage.nextSibling);
      layers.push(layer);

      animations.push(layer.animate([
        { offset: 0, opacity: 0, transform: `${layerBaseTransform} scale(.992)`, "--ripple-radius": "-9%" },
        { offset: .06, opacity: .78 - Math.abs(polarity) * .1, transform: `${layerBaseTransform} scale(${.997 + polarity * .002})`, "--ripple-radius": "-4%" },
        { offset: .5, opacity: .9 - Math.abs(polarity) * .1, transform: `${layerBaseTransform} scale(${1.012 + polarity * .004})`, "--ripple-radius": "35%" },
        { offset: .88, opacity: .65 - Math.abs(polarity) * .08, transform: `${layerBaseTransform} scale(${1.018 + polarity * .003})`, "--ripple-radius": "69%" },
        { offset: 1, opacity: 0, transform: `${layerBaseTransform} scale(1.022)`, "--ripple-radius": "80%" },
      ], { duration, easing: "linear", fill: "both" }));
    }

    if (quality !== "low") {
      const caustic = document.createElement("div");
      caustic.className = "ripple-refraction-caustic";
      caustic.dataset.rippleCaustic = "outgoing-refraction";
      Object.assign(caustic.style, {
        width: `${stageWidth}px`,
        height: `${stageHeight}px`,
        transform: baseTransform,
      });
      presenter.insertBefore(caustic, stage.nextSibling);
      layers.push(caustic);
      animations.push(caustic.animate([
        { opacity: 0, transform: baseTransform, "--ripple-radius": "-8%" },
        { offset: .08, opacity: .72, "--ripple-radius": "-2%" },
        { offset: .72, opacity: .42, "--ripple-radius": "55%" },
        { opacity: 0, transform: `${baseTransform} scale(1.01)`, "--ripple-radius": "80%" },
      ], { duration, easing: "linear", fill: "both" }));
    }

    animations.push(stage.animate([
      { opacity: .98, clipPath: "circle(0% at 50% 50%)" },
      { offset: .5, opacity: 1, clipPath: "circle(35% at 50% 50%)" },
      { opacity: 1, clipPath: "circle(78% at 50% 50%)" },
    ], { duration, easing: "linear", fill: "both" }));

    return {
      player,
      animations,
      metrics: {
        quality,
        layers: layerCount,
        nodeCount: underlay.querySelectorAll("*").length,
        sharedTexture: sceneTextureMode,
      },
      cleanup: () => {
        layers.forEach((node) => node.remove());
        sharedTexture?.cleanup();
        stage.style.zIndex = "1";
        stage.style.willChange = "";
      },
    };
  }, underlay);
}

function windSheetStateAt(progress, stripCount, stageWidth, stageHeight, direction = "right") {
  const p = clamp(Number(progress) || 0, 0, 1);
  const count = Math.max(1, Math.round(Number(stripCount) || 1));
  const width = Math.max(1, Number(stageWidth) || 1) / count;
  // PowerPoint defaults Wind to the right.  The outgoing sheet accelerates
  // diagonally up/right while a broad travelling bend runs through it.
  const movesLeft = ["l", "left"].includes(String(direction).toLowerCase());
  const sign = movesLeft ? -1 : 1;
  const q = clamp((p - .15) / .7, 0, 1);
  const travelX = sign * stageWidth * 1.22 * Math.pow(q, 2.1);
  const travelY = -stageHeight * .55 * Math.pow(q, 1.55);
  const envelope = q > 0 && q < 1 ? 34 * Math.pow(Math.sin(Math.PI * q), .55) : 0;
  const baseDepth = 104 * Math.sin(Math.PI * q);
  const surfaceEnvelope = envelope / 34;
  const boundaries = Array.from({ length: count + 1 }, (_, index) => {
    const u = index / count;
    const phase = 2 * Math.PI * (1.1 * u - .18 * q);
    const topWave = Math.sin(phase - .35);
    const bottomWave = Math.sin(phase + .72);
    return {
      u,
      top: {
        x: travelX + u * stageWidth + sign * stageWidth * .014 * surfaceEnvelope * topWave,
        y: travelY + stageHeight * .025 * surfaceEnvelope * topWave,
      },
      bottom: {
        x: travelX + u * stageWidth + sign * stageWidth * .052 * surfaceEnvelope * bottomWave,
        y: travelY + stageHeight + stageHeight * .09 * surfaceEnvelope * bottomWave,
      },
    };
  });
  const poses = [];
  let chainX = travelX;
  let chainZ = baseDepth;
  for (let index = 0; index < count; index += 1) {
    const phase = 2 * Math.PI * (1.1 * (index + .5) / count - .18 * q);
    const angle = sign * envelope * Math.sin(phase);
    poses.push({
      index,
      dx: chainX - index * width,
      y: travelY,
      z: chainZ,
      angle,
      light: Math.min(.1, Math.abs(Math.sin(angle * Math.PI / 180)) * .11),
      quad: [
        boundaries[index].top,
        boundaries[index + 1].top,
        boundaries[index + 1].bottom,
        boundaries[index].bottom,
      ],
    });
    const radians = angle * Math.PI / 180;
    chainX += width * Math.cos(radians);
    // CSS rotateY moves a positive-x edge toward negative z for positive
    // angles.  Carry that exact endpoint into the next strip: no random
    // stagger, no independent flight path, and therefore no torn seams.
    chainZ -= width * Math.sin(radians);
  }
  return {
    progress: p,
    q,
    sign,
    travelX,
    travelY,
    envelope,
    baseDepth,
    boundaries,
    poses,
  };
}

function windProjectiveTransform(pose, sourceWidth, sourceHeight, originalLeft) {
  const width = Math.max(.001, Number(sourceWidth) || .001);
  const height = Math.max(.001, Number(sourceHeight) || .001);
  const [topLeft, topRight, bottomRight, bottomLeft] = pose.quad.map((point) => ({
    x: point.x - originalLeft,
    y: point.y,
  }));
  const dx1 = topRight.x - bottomRight.x;
  const dx2 = bottomLeft.x - bottomRight.x;
  const dx3 = topLeft.x - topRight.x + bottomRight.x - bottomLeft.x;
  const dy1 = topRight.y - bottomRight.y;
  const dy2 = bottomLeft.y - bottomRight.y;
  const dy3 = topLeft.y - topRight.y + bottomRight.y - bottomLeft.y;
  let projectX = 0;
  let projectY = 0;
  const denominator = dx1 * dy2 - dx2 * dy1;
  if (Math.abs(dx3) > .000001 || Math.abs(dy3) > .000001) {
    if (Math.abs(denominator) > .000001) {
      projectX = (dx3 * dy2 - dx2 * dy3) / denominator;
      projectY = (dx1 * dy3 - dx3 * dy1) / denominator;
    }
  }
  const a = topRight.x - topLeft.x + projectX * topRight.x;
  const b = bottomLeft.x - topLeft.x + projectY * bottomLeft.x;
  const c = topLeft.x;
  const d = topRight.y - topLeft.y + projectX * topRight.y;
  const e = bottomLeft.y - topLeft.y + projectY * bottomLeft.y;
  const f = topLeft.y;
  const values = [
    a / width, d / width, 0, projectX / width,
    b / height, e / height, 0, projectY / height,
    0, 0, 1, 0,
    c, f, 0, 1,
  ];
  return `matrix3d(${values.map((value) => (Math.abs(value) < 1e-9 ? 0 : value).toFixed(8)).join(",")})`;
}

function animateWindTransition(stage, duration, direction, underlay = null, kind = "wind") {
  return runSlideTransition(stage, () => {
    const movesLeft = ["l", "left"].includes(String(direction).toLowerCase());
    if (!underlay || !stage.parentElement) {
      const hidden = movesLeft ? "inset(0 0 0 100%)" : "inset(0 100% 0 0)";
      const player = stage.animate([
        { clipPath: hidden, opacity: .42 },
        { clipPath: "inset(0)", opacity: 1 },
      ], { duration, easing: "linear", fill: "both" });
      return { player, animations: [player] };
    }

    // Wind is one coherent sheet, not staggered ribbons.  Adjacent strips use
    // the same sampled timeline and a cumulative 3-D chain so the right edge
    // of strip i is the physical left edge of strip i+1 at every keyframe.
    const presenter = stage.parentElement;
    const baseTransform = underlay.style.transform || stage.style.transform || "";
    const stageWidth = Math.max(1, Number(stage.clientWidth) || parseFloat(stage.style.width) || 1280);
    const stageHeight = Math.max(1, Number(stage.clientHeight) || parseFloat(stage.style.height) || 720);
    const quality = presentationQualityTier(`slide-transition:${kind}`);
    const textureComplexity = underlay.querySelectorAll("*").length;
    let stripCount = quality === "low" ? 6 : quality === "medium" ? 8 : 10;
    const strips = [];
    const animations = [];
    const sharedTexture = createSharedTransitionTexture(
      underlay,
      presenter,
      stageWidth,
      stageHeight,
      `wind-${kind}`,
    );
    const sceneTextureMode = Boolean(sharedTexture?.createSceneHorizontalSlice);
    // Without a shared texture every strip owns a full cloned scene.  Ten
    // connected facets are smooth on ordinary pages; complex scenes cap at
    // eight before the first run to protect the first-frame budget.
    if (!sceneTextureMode) {
      if (textureComplexity > 150) stripCount = Math.min(stripCount, quality === "low" ? 6 : 8);
    }
    underlay.style.visibility = "hidden";
    stage.style.zIndex = "1";
    stage.style.opacity = "1";

    let mesh = null;
    if (sceneTextureMode) {
      sharedTexture.mountScene({
        className: "wind-transition-mesh wind-transition-texture-scene",
        dataset: { windScene: kind },
        style: {
          position: "absolute", left: "50%", top: "50%", transform: baseTransform,
          transformOrigin: "50% 50%", zIndex: "3", overflow: "visible", pointerEvents: "none",
        },
      });
    } else {
      mesh = document.createElement("div");
      mesh.className = "wind-transition-mesh";
      Object.assign(mesh.style, {
        left: "50%", top: "50%", width: `${stageWidth}px`, height: `${stageHeight}px`,
        transform: baseTransform,
      });
      presenter.insertBefore(mesh, stage.nextSibling);
    }

    for (let index = 0; index < stripCount; index += 1) {
      const left = index / stripCount;
      const width = 1 / stripCount;
      const strip = sceneTextureMode
        ? sharedTexture.createSceneHorizontalSlice({
            x: left * stageWidth,
            width: width * stageWidth,
            left: left * stageWidth,
            displayWidth: width * stageWidth + .7,
            className: "wind-transition-strip wind-transition-texture",
            dataset: { windStrip: index },
            style: { willChange: "transform", transformOrigin: "0 0" },
          })
        : document.createElement("div");
      strip.classList.add("wind-transition-strip");
      strip.dataset.windStrip = String(index);
      if (!sceneTextureMode) {
        Object.assign(strip.style, {
          left: `${(left * 100).toFixed(5)}%`, top: "0",
          width: `calc(${(width * 100).toFixed(5)}% + 1.4px)`, height: "100%",
          transformOrigin: "0 0",
        });
        const texture = underlay.cloneNode(true);
        namespaceTransitionCloneIds(texture, "wind");
        texture.classList.remove("slide-transition-underlay");
        texture.querySelectorAll("audio,video").forEach((node) => node.remove());
        Object.assign(texture.style, {
          position: "absolute", left: `${(-left * stageWidth).toFixed(3)}px`, top: "0",
          width: `${stageWidth}px`, height: `${stageHeight}px`, transform: "none", visibility: "visible",
        });
        strip.append(texture);
        mesh.append(strip);
      }
      const lighting = document.createElement("div");
      lighting.className = "wind-transition-lighting";
      strip.append(lighting);
      strips.push(strip);
      const sampleOffsets = [0, .08, .15, .2, .26, .34, .42, .5, .58, .66, .74, .82, .86, .92, 1];
      const frames = sampleOffsets.map((offset) => {
        const pose = windSheetStateAt(offset, stripCount, stageWidth, stageHeight, direction).poses[index];
        return {
          offset,
          opacity: 1,
          transform: windProjectiveTransform(pose, width * stageWidth, stageHeight, left * stageWidth),
        };
      });
      animations.push(strip.animate(frames, { duration, easing: "linear", fill: "both" }));
      animations.push(lighting.animate(sampleOffsets.map((offset) => {
        const pose = windSheetStateAt(offset, stripCount, stageWidth, stageHeight, direction).poses[index];
        return { offset, opacity: pose.light.toFixed(4) };
      }), { duration, easing: "linear", fill: "both" }));
    }

    const shadowFrame = document.createElement("div");
    shadowFrame.className = "wind-transition-shadow-frame";
    Object.assign(shadowFrame.style, {
      left: "50%", top: "50%", width: `${stageWidth}px`, height: `${stageHeight}px`,
      transform: baseTransform,
    });
    const sheetShadow = document.createElement("div");
    sheetShadow.className = "wind-transition-sheet-shadow";
    shadowFrame.append(sheetShadow);
    presenter.insertBefore(shadowFrame, stage.nextSibling);
    const shadowOffsets = [0, .15, .26, .42, .58, .74, .86, 1];
    animations.push(sheetShadow.animate(shadowOffsets.map((offset) => {
      const state = windSheetStateAt(offset, stripCount, stageWidth, stageHeight, direction);
      return {
        offset,
        opacity: (.2 * Math.sin(Math.PI * state.q)).toFixed(4),
        transform: `translate3d(${(state.travelX * .92).toFixed(3)}px,${(state.travelY + stageHeight * .72).toFixed(3)}px,0) scaleX(${(1 - .18 * Math.sin(Math.PI * state.q)).toFixed(4)})`,
      };
    }), { duration, easing: "linear", fill: "both" }));

    const player = animations[0];
    synchronizeTransitionAnimations(animations);
    return {
      player,
      animations,
      metrics: { quality, layers: stripCount + 1, nodeCount: textureComplexity, sharedTexture: sceneTextureMode, coherentSheet: true },
      cleanup: () => {
        strips.forEach((node) => node.remove());
        mesh?.remove();
        shadowFrame.remove();
        sharedTexture?.cleanup();
        stage.style.zIndex = "1";
      },
    };
  }, underlay);
}

function animateCurtainTransition(stage, duration, underlay = null, kind = "curtains") {
  return runSlideTransition(stage, () => {
    const player = stage.animate([
      { clipPath: "inset(0 50% 0 50%)", filter: "brightness(.55)" },
      { offset: .48, filter: kind === "drape" ? "brightness(.72) contrast(1.15)" : "brightness(.86)" },
      { clipPath: "inset(0 0 0 0)", filter: "brightness(1)" },
    ], { duration, easing: "cubic-bezier(.3,.06,.16,1)", fill: "both" });
    return { player, animations: [player] };
  }, underlay);
}

function runSlideTransition(stage, create, underlay = null) {
  clearSlideTransition(stage, underlay);
  const kind = stage.dataset.transitionKind || "transition";
  const duration = Math.max(0, Number(stage.dataset.transitionDurationMs) || 0);
  const label = `slide-transition:${kind}`;
  const frameProbe = beginPresentationFrameProbe(label, duration, {
    kind,
    quality: presentationQualityTier(label),
  });
  let result;
  try {
    // Start the probe before create(): cloning a complex outgoing page is a
    // first-frame cost and must count against the 30-fps budget.
    result = create();
    frameProbe.annotate(result.metrics || {});
  } catch (error) {
    frameProbe.finish(false);
    underlay?.remove();
    throw error;
  }
  const effectCleanup = result.cleanup || (() => {});
  const record = {
    animations: result.animations || [result.player],
    cleanup: (commitProbe = true) => {
      frameProbe.finish(commitProbe);
      effectCleanup();
      underlay?.remove();
    },
    settled: false,
  };
  activeSlideTransitions.set(stage, record);
  const settle = () => queueMicrotask(() => settleSlideTransition(stage, record));
  result.player.addEventListener("finish", settle, { once: true });
  result.player.addEventListener("cancel", settle, { once: true });
  return result.player;
}

function settleSlideTransition(stage, record) {
  if (record.settled) return;
  record.settled = true;
  if (activeSlideTransitions.get(stage) === record) activeSlideTransitions.delete(stage);
  record.cleanup(true);
  for (const animation of record.animations) {
    if (animation.playState !== "idle") animation.cancel();
  }
}

function clearSlideTransition(stage, preserveUnderlay = null) {
  const record = activeSlideTransitions.get(stage);
  if (record) {
    record.settled = true;
    activeSlideTransitions.delete(stage);
    record.cleanup(false);
    for (const animation of record.animations) animation.cancel();
  }
  clearTransitionMask(stage);
  stage.querySelectorAll(":scope > .slide-transition-overlay").forEach((node) => node.remove());
  stage.parentElement?.querySelectorAll(":scope > .slide-transition-underlay").forEach((node) => {
    if (node !== preserveUnderlay) node.remove();
  });
}

function clearTransitionMask(stage) {
  for (const property of [
    "mask-image", "mask-size", "mask-position", "mask-repeat", "mask-composite",
    "-webkit-mask-image", "-webkit-mask-size", "-webkit-mask-position", "-webkit-mask-repeat",
  ]) stage.style.removeProperty(property);
}

function animateRandomBarsTransition(stage, duration, direction, underlay = null) {
  return runSlideTransition(stage, () => {
    const vertical = ["vert", "vertical", "v"].includes(String(direction).toLowerCase());
    const quality = presentationQualityTier("slide-transition:randombar");
    const count = quality === "low"
      ? (vertical ? 18 : 10)
      : quality === "medium"
      ? (vertical ? 24 : 14)
      : (vertical ? 30 : 18);
    const barSize = 100 / count + .08;
    const layers = Array(count).fill("linear-gradient(#000 0 0)").join(",");
    const positions = Array.from({ length: count }, (_, index) => {
      const axis = count === 1 ? 0 : index * 100 / (count - 1);
      return vertical
        ? `${axis.toFixed(4)}% ${index % 2 ? 100 : 0}%`
        : `${index % 2 ? 100 : 0}% ${axis.toFixed(4)}%`;
    }).join(",");
    const rank = randomBarRanks(count);
    const keyframes = Array.from({ length: 15 }, (_, frameIndex) => {
      const offset = frameIndex / 14;
      const sizes = Array.from({ length: count }, (_, barIndex) => {
        const start = rank[barIndex] / Math.max(1, count - 1) * .68;
        const local = clamp((offset - start) / .32, 0, 1);
        const progress = local * local * (3 - 2 * local) * 100;
        return vertical
          ? `${barSize.toFixed(4)}% ${progress.toFixed(3)}%`
          : `${progress.toFixed(3)}% ${barSize.toFixed(4)}%`;
      }).join(",");
      return { offset, maskSize: sizes, webkitMaskSize: sizes };
    });

    Object.assign(stage.style, {
      maskImage: layers,
      maskPosition: positions,
      maskRepeat: "no-repeat",
      maskComposite: "add",
      webkitMaskImage: layers,
      webkitMaskPosition: positions,
      webkitMaskRepeat: "no-repeat",
    });
    const player = stage.animate(keyframes, { duration, easing: "linear", fill: "both" });
    return {
      player,
      animations: [player],
      metrics: { quality, layers: count },
      cleanup: () => clearTransitionMask(stage),
    };
  }, underlay);
}

function randomBarRanks(count) {
  const order = Array.from({ length: count }, (_, index) => index);
  let seed = 0x51f15e5d;
  for (let index = count - 1; index > 0; index -= 1) {
    seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
    const swapIndex = seed % (index + 1);
    [order[index], order[swapIndex]] = [order[swapIndex], order[index]];
  }
  const rank = Array(count);
  order.forEach((bar, index) => { rank[bar] = index; });
  return rank;
}

function animateRevealTransition(stage, duration, direction, underlay = null) {
  return runSlideTransition(stage, () => {
    const normalized = String(direction).toLowerCase();
    const vertical = ["u", "up", "d", "down"].includes(normalized);
    const forward = !["l", "left", "u", "up"].includes(normalized);
    const gradientDirection = vertical ? "to bottom" : "to right";
    const mask = forward
      ? `linear-gradient(${gradientDirection},#000 0 46%,transparent 54% 100%)`
      : `linear-gradient(${gradientDirection},transparent 0 46%,#000 54% 100%)`;
    const maskSize = vertical ? "100% 220%" : "220% 100%";
    const start = vertical
      ? (forward ? "0 100%" : "0 0")
      : (forward ? "100% 0" : "0 0");
    const end = vertical
      ? (forward ? "0 0" : "0 100%")
      : (forward ? "0 0" : "100% 0");

    Object.assign(stage.style, {
      maskImage: mask,
      maskSize,
      maskPosition: start,
      maskRepeat: "no-repeat",
      webkitMaskImage: mask,
      webkitMaskSize: maskSize,
      webkitMaskPosition: start,
      webkitMaskRepeat: "no-repeat",
    });
    const player = stage.animate([
      { offset: 0, maskPosition: start, webkitMaskPosition: start, filter: "brightness(.92)" },
      { offset: .62, filter: "brightness(1.06)" },
      { offset: 1, maskPosition: end, webkitMaskPosition: end, filter: "brightness(1)" },
    ], { duration, easing: "cubic-bezier(.22,.7,.22,1)", fill: "both" });
    return { player, animations: [player], cleanup: () => clearTransitionMask(stage) };
  }, underlay);
}

function animateFlashTransition(stage, duration, underlay = null) {
  return runSlideTransition(stage, () => {
    const flash = document.createElement("div");
    flash.className = "slide-transition-overlay slide-transition-flash";
    stage.append(flash);
    const player = stage.animate([
      { offset: 0, opacity: 0 },
      { offset: .1, opacity: 1 },
      { offset: 1, opacity: 1 },
    ], { duration, easing: "linear", fill: "both" });
    const flashPlayer = flash.animate([
      { offset: 0, opacity: 0 },
      { offset: .08, opacity: 1 },
      { offset: .28, opacity: 1 },
      { offset: .68, opacity: 0 },
      { offset: 1, opacity: 0 },
    ], { duration, easing: "ease-out", fill: "both" });
    return {
      player,
      animations: [player, flashPlayer],
      cleanup: () => flash.remove(),
    };
  }, underlay);
}

function presentationAnimations(slide = state.deck.slides[state.presenterSlide]) {
  const inheritedAnimations = [...(slide?.inheritedAnimations || [])].sort((a, b) => a.order - b.order);
  const slideAnimations = [...(slide?.animations || [])].sort((a, b) => a.order - b.order);
  return [...inheritedAnimations, ...slideAnimations];
}

function presentationTransitionLead(slide, playTransition) {
  if (!playTransition || !slide?.transition) return 0;
  const kind = String(slide.transition.kind || "fade").toLowerCase();
  return kind === "none" || kind === "cut"
    ? 0
    : Math.max(1, Number(slide.transition.durationMs) || 700);
}

function restorePresentationAnimationState(stage, animations, cursor) {
  for (const animation of animations.slice(0, cursor)) {
    applyCompletedAnimationState(stage, animation);
  }
}

function applyCompletedAnimationState(stage, animation, runMedia = false) {
  if (!animation.targetObjectId) return;
  const node = stage.querySelector(`[data-id="${cssEscape(animation.targetObjectId)}"]`);
  if (!node) return;
  if (animation.effect === "media" || animation.class === "media") {
    if (runMedia) globalThis.UniPptMedia?.controlNode(node, animation.mediaAction || "play");
    return;
  }
  const frames = animationFrames(animation, node);
  const returnsToStart = Boolean(animation.autoReverse) || Number(animation.speed) < 0;
  const finalFrame = frames[returnsToStart ? 0 : frames.length - 1] || {};
  for (const [property, value] of Object.entries(finalFrame)) {
    if (property !== "offset") node.style[property] = String(value);
  }
  if (animation.class === "exit" && !returnsToStart) {
    node.style.visibility = "hidden";
    node.style.opacity = "0";
  } else {
    node.style.visibility = "visible";
    if (finalFrame.opacity == null) node.style.opacity = "1";
  }
}

function runPresentationAnimationBatch(stage, animations, startOffset = 0) {
  const schedule = scheduleAnimationNavigation(animations, state.presenterAnimationCursor);
  if (!schedule.entries.length || !schedule.positions.length) return schedule;
  const firstPosition = schedule.positions[0];
  state.presenterAnimationBatchStartCursor = firstPosition.startIndex;
  state.presenterAnimationTargetCursor = firstPosition.endIndex;
  const playbackToken = state.presenterPlaybackToken;
  const visualEntries = schedule.entries.filter(({ animation }) => (
    animation.targetObjectId
    && animation.effect !== "media"
    && animation.class !== "media"
  ));
  if (visualEntries.length) {
    const offset = Math.max(0, Number(startOffset) || 0);
    const firstMotionMs = offset + Math.min(...visualEntries.map((entry) => entry.start));
    const lastMotionMs = offset + Math.max(...visualEntries.map((entry) => entry.end));
    const expectedDurationMs = Math.max(1, lastMotionMs - firstMotionMs);
    let frameProbe = null;
    const beginProbe = () => {
      if (playbackToken !== state.presenterPlaybackToken) return;
      frameProbe = beginPresentationFrameProbe(
        `native-animation:slide-${state.presenterSlide + 1}`,
        expectedDurationMs,
        {
          kind: "native-animation",
          quality: presentationQualityTier("native-animation"),
          layers: visualEntries.length,
          slide: state.presenterSlide + 1,
        },
      );
    };
    if (firstMotionMs <= 0) beginProbe();
    else {
      const probeStartTimer = setTimeout(beginProbe, firstMotionMs);
      state.presenterTimers.push(probeStartTimer);
    }
    const probeFinishTimer = setTimeout(() => frameProbe?.finish(true), lastMotionMs + 50);
    state.presenterTimers.push(probeFinishTimer);
  }
  for (const entry of schedule.entries) {
    animateSceneEffect(stage, entry.animation, startOffset + entry.start, state.presenterTimers, state.presenterPlayers);
  }
  schedule.positions.forEach((position, index) => {
    const settleTimer = setTimeout(() => {
      if (playbackToken !== state.presenterPlaybackToken) return;
      state.presenterAnimationCursor = position.endIndex;
      const nextPosition = schedule.positions[index + 1];
      state.presenterAnimationBatchStartCursor = nextPosition?.startIndex ?? null;
      state.presenterAnimationTargetCursor = nextPosition?.endIndex ?? null;
      updatePresentationCount(animations);
    }, Math.max(0, startOffset) + position.end);
    state.presenterTimers.push(settleTimer);
  });
  return schedule;
}

function finishPresentationAnimationBatch(animations = presentationAnimations()) {
  const target = state.presenterAnimationTargetCursor;
  if (target == null) return false;
  state.presenterPlaybackToken += 1;
  cancelPresentationFrameProbes(false);
  state.presenterTimers.forEach(clearTimeout);
  state.presenterTimers = [];
  state.presenterPlayers.forEach((player) => player.cancel());
  state.presenterPlayers = [];
  const stage = $("#presentStage");
  // Cancelling a finished Web Animation removes its `fill: forwards` effect,
  // so rebuild every settled DOM state up to this navigation position. Media
  // play is idempotent for the persistent cross-slide audio decoder. Seeking
  // visual media to an exact native end-state remains a separate concern.
  for (const animation of animations.slice(0, target)) {
    applyCompletedAnimationState(stage, animation, true);
  }
  state.presenterAnimationCursor = target;
  state.presenterAnimationTargetCursor = null;
  state.presenterAnimationBatchStartCursor = null;
  updatePresentationCount(animations);
  return true;
}

function presentationPageComplete(animations = presentationAnimations()) {
  return state.presenterAnimationTargetCursor == null
    && state.presenterAnimationCursor >= animations.length;
}

function completePresentationPageAnimations() {
  const animations = presentationAnimations();
  state.presenterPlaybackToken += 1;
  cancelPresentationFrameProbes(false);
  state.presenterTimers.forEach(clearTimeout);
  state.presenterTimers = [];
  state.presenterPlayers.forEach((player) => player.cancel());
  state.presenterPlayers = [];
  const stage = $("#presentStage");
  for (const animation of animations) applyCompletedAnimationState(stage, animation, true);
  state.presenterAnimationCursor = animations.length;
  state.presenterAnimationTargetCursor = null;
  state.presenterAnimationBatchStartCursor = null;
  updatePresentationCount(animations);
}

// Ctrl+Home / Ctrl+End settle every animation on the current page instantly.
// Once the page is already settled, End walks forward and Home walks back,
// settling the destination page the same way.
function skipPresentationToPageEnd(direction) {
  hidePresentationBoundaryNotice();
  if (!presentationPageComplete()) {
    completePresentationPageAnimations();
    return;
  }
  if (direction > 0) {
    if (!advanceToNextPresentationSlide({ userInitiated: true })) return;
    completePresentationPageAnimations();
  } else {
    jumpPresentationPage(-1);
  }
}

function cancelPresentationAnimationBatch() {
  if (state.presenterAnimationTargetCursor == null) return false;
  const restoreCursor = state.presenterAnimationBatchStartCursor ?? state.presenterAnimationCursor;
  renderPresentation(false, { restoreCursor, runInitialAutomatic: false });
  return true;
}

function advanceToNextPresentationSlide(options = {}) {
  let target = state.presenterSlide + 1;
  if (target > state.presenterRangeEnd) {
    const autoPlay = state.presenterMode !== "rehearse" && Boolean(state.slideShowSettings?.autoPlay);
    const automaticCompletion = autoPlay && options.userInitiated !== true;
    if (state.presenterMode === "rehearse" || automaticCompletion || options.exitAtEnd) {
      closePresentation();
      return false;
    }
    if (!state.slideShowSettings.loop) {
      showPresentationBoundaryNotice();
      return false;
    }
    target = state.presenterRangeStart;
  }
  recordRehearsalTiming();
  state.presenterSlide = target;
  state.presenterAnimationCursor = 0;
  state.presenterAnimationTargetCursor = null;
  state.presenterAnimationBatchStartCursor = null;
  renderPresentation(true);
  return true;
}

function advancePresentation(options = {}) {
  hidePresentationBoundaryNotice();
  const animations = presentationAnimations();
  const autoPlay = state.presenterMode !== "rehearse" && Boolean(state.slideShowSettings?.autoPlay);
  if (finishPresentationAnimationBatch(animations)) {
    if (autoPlay) {
      const slide = state.deck.slides[state.presenterSlide];
      const completedAllAnimations = state.presenterAnimationCursor >= animations.length;
      schedulePresentationAutoPlay(completedAllAnimations
        ? presentationCompletionDelay(slide)
        : PRESENTATION_MIN_POST_ANIMATION_MS);
    }
    return;
  }
  if (state.presenterAnimationCursor >= animations.length) {
    advanceToNextPresentationSlide(options);
    return;
  }
  const stage = $("#presentStage");
  const schedule = runPresentationAnimationBatch(stage, animations);
  if (autoPlay) {
    schedulePresentationAutoPlay(
      Math.max(1, Number(schedule?.end) || 0) + PRESENTATION_MIN_POST_ANIMATION_MS,
    );
  }
  updatePresentationCount(animations);
}

function handleKeyboard(event) {
  if (!$("#slideContextMenu")?.hidden) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeSlideContextMenu();
      $("#slideList .active")?.focus({ preventScroll: true });
    }
    if (event.target.closest?.("#slideContextMenu")) return;
  }
  if (!$("#exportMenuPopup")?.hidden) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeExportMenu();
      $("#exportMenuButton")?.focus({ preventScroll: true });
    }
    if (event.target.closest?.("#exportMenuPopup")) return;
  }
  if (handleBrowserShortcut(event)) return;
  if ($("#udocStructureDialog")?.open) return;
  if (!$("#formulaDialog").open && !$("#presenter").hidden) {
    if (event.key === "Escape") { event.preventDefault(); closePresentation(); return; }
    if (event.key === "Tab") { showPresentationControls(true); return; }
    if ((event.ctrlKey || event.metaKey) && event.key === "Home") { event.preventDefault(); skipPresentationToPageEnd(-1); return; }
    if ((event.ctrlKey || event.metaKey) && event.key === "End") { event.preventDefault(); skipPresentationToPageEnd(1); return; }
    if (event.key === "Home") { event.preventDefault(); stepPresentation(-1); return; }
    if (event.key === "End") { event.preventDefault(); stepPresentation(1); return; }
    if (event.target.closest?.(".present-controls")) return;
    if (event.key.toLowerCase() === "m") { event.preventDefault(); togglePresentationMute(); showPresentationControls(true); return; }
    if (event.key.toLowerCase() === "a") { event.preventDefault(); togglePresentationAutoPlay(); showPresentationControls(true); return; }
    if (["ArrowRight", "ArrowDown", "PageDown", " ", "Enter"].includes(event.key)) { event.preventDefault(); stepPresentation(1); }
    if (["ArrowLeft", "ArrowUp", "PageUp"].includes(event.key)) { event.preventDefault(); stepPresentation(-1); }
    return;
  }
  if (isTextEditingTarget(event.target)) return;
  if (event.altKey && !event.ctrlKey && !event.metaKey && event.key.toLowerCase() === "a") {
    event.preventDefault();
    void openAiAssistant();
    return;
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") { event.preventDefault(); saveUdoc(); return; }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "z") { event.preventDefault(); event.shiftKey ? redo() : undo(); return; }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "y") { event.preventDefault(); redo(); return; }
  if ((event.key === "Delete" || event.key === "Backspace") && state.selectedId) { event.preventDefault(); deleteSelected(); return; }
  const object = selectedObject();
  if (object && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
    event.preventDefault();
    const step = event.shiftKey ? 10 : 1;
    mutate(() => {
      if (event.key === "ArrowLeft") object.frame.x -= step;
      if (event.key === "ArrowRight") object.frame.x += step;
      if (event.key === "ArrowUp") object.frame.y -= step;
      if (event.key === "ArrowDown") object.frame.y += step;
    });
  }
}

function handleBrowserShortcut(event) {
  const modifier = event.ctrlKey || event.metaKey;
  const key = event.key.toLowerCase();
  const modalOpen = Boolean(document.querySelector("dialog[open]"));
  const editing = isTextEditingTarget(event.target);
  if (event.altKey && key === "q" && !modalOpen) {
    event.preventDefault();
    $("#officeSearchInput")?.focus();
    $("#officeSearchInput")?.select();
    return true;
  }
  if (modifier && !editing && !modalOpen && key === "c") { event.preventDefault(); copySelectedObject(); return true; }
  if (modifier && !editing && !modalOpen && key === "x") { event.preventDefault(); cutSelectedObject(); return true; }
  if (modifier && !editing && !modalOpen && key === "v") { event.preventDefault(); void pasteObject(); return true; }
  if (modifier && !editing && !modalOpen && key === "f") { event.preventDefault(); openFindReplace(false); return true; }
  if (modifier && event.shiftKey && !editing && !modalOpen && event.key === "]") { event.preventDefault(); arrangeSelectedObject("front"); return true; }
  if (modifier && event.shiftKey && !editing && !modalOpen && event.key === "[") { event.preventDefault(); arrangeSelectedObject("back"); return true; }
  if (modifier && (
    ["+", "="].includes(event.key)
    || event.code === "NumpadAdd"
  )) {
    event.preventDefault();
    if (state.deck && !modalOpen) adjustZoom(ZOOM_STEP);
    return true;
  }
  if (modifier && (
    ["-", "_"].includes(event.key)
    || event.code === "NumpadSubtract"
  )) {
    event.preventDefault();
    if (state.deck && !modalOpen) adjustZoom(-ZOOM_STEP);
    return true;
  }
  if (modifier && (key === "0" || event.code === "Numpad0")) {
    event.preventDefault();
    if (state.deck && !modalOpen) fitSlide();
    return true;
  }
  if (modifier && key === "s") {
    event.preventDefault();
    if (state.deck && !modalOpen) saveUdoc();
    return true;
  }
  if (event.key === "F5") {
    event.preventDefault();
    if (state.deck && !modalOpen && $("#presenter").hidden) {
      void startPresentationWhenReady(event.shiftKey ? "current" : "beginning");
    }
    return true;
  }
  if (modifier && key === "o") {
    event.preventDefault();
    if (!modalOpen) openDocumentPicker();
    return true;
  }
  if (modifier && ["l", "n", "p", "r", "t", "w"].includes(key)) {
    event.preventDefault();
    return true;
  }
  if (modifier && key === "u" && !isTextEditingTarget(event.target)) {
    event.preventDefault();
    return true;
  }
  return false;
}

function mutate(operation, rerender = true) {
  checkpoint();
  operation();
  globalThis.UniPptPresentationHost?.enrichSemantics?.(state.deck);
  state.future = [];
  markDeckChanged();
  if (rerender) renderAll(); else updateHistoryButtons();
  if (!state.hostCommitDepth) state.documentHost?._emit?.("change", { revision: state.documentCache?.revision || 0, source: "editor" });
}
function historySnapshot() {
  const clone = globalThis.UniPptDocumentCache?.cloneForHistory || structuredClone;
  return {
    deck: clone(state.deck), documentCache: clone(state.documentCache),
    activeSlide: state.activeSlide, selectedId: state.selectedId,
    selectedAnimationId: state.selectedAnimationId,
  };
}
function checkpoint() {
  if (!state.deck) return;
  state.history.push(historySnapshot());
  if (state.history.length > 80) state.history.shift();
}
function undo() {
  if (!state.history.length) return;
  state.future.push(historySnapshot());
  restoreSnapshot(state.history.pop());
}
function redo() {
  if (!state.future.length) return;
  state.history.push(historySnapshot());
  restoreSnapshot(state.future.pop());
}
function restoreSnapshot(snapshot) {
  const clone = globalThis.UniPptDocumentCache?.cloneForHistory || structuredClone;
  state.deck = clone(snapshot.deck);
  if (snapshot.documentCache) state.documentCache = clone(snapshot.documentCache);
  state.activeSlide = snapshot.activeSlide;
  state.selectedId = snapshot.selectedId;
  state.selectedAnimationId = snapshot.selectedAnimationId || null;
  autoTextFitCache.clear();
  void installEmbeddedFonts(state.deck);
  markDeckChanged();
  renderAll();
  if (state.autoFit) requestAnimationFrame(fitSlide);
  state.documentHost?._emit?.("change", { revision: state.documentCache?.revision || 0, source: "history" });
}
function updateHistoryButtons() {
  $("#undo").disabled = !state.history.length;
  $("#redo").disabled = !state.future.length;
}

function currentSlide() { return state.deck.slides[state.activeSlide]; }
function selectedObject() { return state.selectedId ? findObject(state.selectedId) : null; }
function findObject(id, objects = currentSlide().objects) {
  for (const object of objects) {
    if (object.id === id) return object;
    const child = object.children?.length ? findObject(id, object.children) : null;
    if (child) return child;
  }
  return null;
}
function removeObject(objects, id) {
  const index = objects.findIndex((object) => object.id === id);
  if (index >= 0) { objects.splice(index, 1); return true; }
  return objects.some((object) => object.children?.length && removeObject(object.children, id));
}
function flattenObjects(objects, result = []) {
  for (const object of objects) {
    if (object.kind === "group" && object.children?.length) flattenObjects(object.children, result);
    else result.push(object);
  }
  return result;
}
function renewIds(objects, idMap = new Map()) {
  objects.forEach((object) => {
    const oldId = object.id;
    object.id = uid("shape");
    idMap.set(oldId, object.id);
    object.sourceShapeId = null;
    if (object.children) renewIds(object.children, idMap);
  });
  return idMap;
}
function defaultStyle() { return { fill: "transparent", gradient: null, stroke: "transparent", strokeWidth: 0, strokeDash: null, opacity: 1, shadow: null }; }
function defaultTextStyle() { return { fontFamily: "Aptos, 'Microsoft YaHei', sans-serif", nativeFontFamily: "Aptos", fontSize: 28, color: "#172033", bold: false, italic: false, align: "left" }; }
function uid(prefix) { return `${prefix}-${crypto.randomUUID?.() || Date.now().toString(36) + Math.random().toString(36).slice(2)}`; }
function element(tag, className, text) { const node = document.createElement(tag); if (className) node.className = className; if (text !== undefined) node.textContent = text; return node; }
function number(value, fallback) { const parsed = Number(value); return Number.isFinite(parsed) ? parsed : fallback; }
function clamp(value, min, max) { return Math.max(min, Math.min(max, value)); }
function round(value) { return Math.round(value * 10) / 10; }
function normalizeColor(value, fallback) { return /^#[0-9a-f]{6}$/i.test(value || "") ? value : fallback; }
function escapeRegExp(value) { return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); }
function safeFilename(value) { return (value || "unippt").replace(/[\\/:*?"<>|]+/g, "-").trim() || "unippt"; }
function cssEscape(value) { return CSS.escape ? CSS.escape(value) : value.replace(/[^a-zA-Z0-9_-]/g, "\\$&"); }
function escapeHtml(value) { return String(value).replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#039;" })[char]); }
function download(name, content, type) { const link = document.createElement("a"); link.href = URL.createObjectURL(new Blob([content], { type })); link.download = name; link.click(); setTimeout(() => URL.revokeObjectURL(link.href), 1000); }
function downloadBlob(name, blob) { const link = document.createElement("a"); link.href = URL.createObjectURL(blob); link.download = name; link.click(); setTimeout(() => URL.revokeObjectURL(link.href), 1000); }
function setStatus(message) { $("#status").textContent = message; }
let toastTimer;
function toast(message) { const node = $("#toast"); node.textContent = message; node.classList.add("show"); clearTimeout(toastTimer); toastTimer = setTimeout(() => node.classList.remove("show"), 2600); }
})();
