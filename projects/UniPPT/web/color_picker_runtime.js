"use strict";

// Shared UniDoc-style color picker used by every UniPPT color command.
(function installUniPptColorPicker(global) {
  const DEFAULT_THEME = [
    "#FFFFFF", "#000000", "#E7E6E6", "#44546A", "#4472C4",
    "#ED7D31", "#A5A5A5", "#FFC000", "#5B9BD5", "#70AD47",
  ];
  const TINTS = [0.8, 0.6, 0.4, -0.25, -0.5];
  const STANDARD = [
    "#C00000", "#FF0000", "#FFC000", "#FFFF00", "#92D050",
    "#00B050", "#00B0F0", "#0070C0", "#002060", "#7030A0",
  ];
  const RECENT_KEY = "unippt.recentColors.v1";
  let popup = null;

  function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value));
  }

  function normalizeHex(value, fallback = "#000000") {
    let hex = String(value || "").trim();
    if (/^#[0-9a-f]{3}$/i.test(hex)) {
      hex = `#${hex.slice(1).split("").map((part) => part + part).join("")}`;
    }
    return /^#[0-9a-f]{6}$/i.test(hex) ? hex.toUpperCase() : fallback.toUpperCase();
  }

  function hexToRgb(value) {
    const hex = normalizeHex(value).slice(1);
    return [0, 2, 4].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16));
  }

  function rgbToHex(red, green, blue) {
    const part = (value) => clamp(Math.round(value), 0, 255).toString(16).padStart(2, "0");
    return `#${part(red)}${part(green)}${part(blue)}`.toUpperCase();
  }

  function tint(value, ratio) {
    const [red, green, blue] = hexToRgb(value);
    return ratio >= 0
      ? rgbToHex(red + (255 - red) * ratio, green + (255 - green) * ratio, blue + (255 - blue) * ratio)
      : rgbToHex(red * (1 + ratio), green * (1 + ratio), blue * (1 + ratio));
  }

  function rgbToHsv(red, green, blue) {
    red /= 255; green /= 255; blue /= 255;
    const max = Math.max(red, green, blue);
    const min = Math.min(red, green, blue);
    const delta = max - min;
    let hue = 0;
    if (delta) {
      if (max === red) hue = ((green - blue) / delta) % 6;
      else if (max === green) hue = (blue - red) / delta + 2;
      else hue = (red - green) / delta + 4;
      hue *= 60;
      if (hue < 0) hue += 360;
    }
    return [hue, max ? delta / max : 0, max];
  }

  function hsvToRgb(hue, saturation, value) {
    const chroma = value * saturation;
    const secondary = chroma * (1 - Math.abs((hue / 60) % 2 - 1));
    const match = value - chroma;
    let red = 0; let green = 0; let blue = 0;
    if (hue < 60) [red, green] = [chroma, secondary];
    else if (hue < 120) [red, green] = [secondary, chroma];
    else if (hue < 180) [green, blue] = [chroma, secondary];
    else if (hue < 240) [green, blue] = [secondary, chroma];
    else if (hue < 300) [red, blue] = [secondary, chroma];
    else [red, blue] = [chroma, secondary];
    return [(red + match) * 255, (green + match) * 255, (blue + match) * 255];
  }

  function recentColors() {
    try {
      const values = JSON.parse(global.localStorage?.getItem(RECENT_KEY) || "[]");
      return Array.isArray(values)
        ? values.filter((value) => /^#[0-9a-f]{6}$/i.test(value)).slice(0, 10)
        : [];
    } catch (_) {
      return [];
    }
  }

  function rememberColor(value) {
    if (!/^#[0-9a-f]{6}$/i.test(value)) return;
    const color = normalizeHex(value);
    const values = [color, ...recentColors().filter((item) => item !== color)].slice(0, 10);
    try { global.localStorage?.setItem(RECENT_KEY, JSON.stringify(values)); } catch (_) { /* private mode */ }
  }

  function close() {
    popup?.remove();
    popup = null;
  }

  function place(anchor) {
    if (!popup || !anchor) return;
    const rect = anchor.getBoundingClientRect();
    const width = popup.offsetWidth;
    const height = popup.offsetHeight;
    popup.style.left = `${clamp(rect.left, 8, Math.max(8, global.innerWidth - width - 8))}px`;
    popup.style.top = `${clamp(rect.bottom + 5, 8, Math.max(8, global.innerHeight - height - 8))}px`;
  }

  function swatch(value, choose) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "cp-sw";
    button.style.setProperty("--swatch-color", value);
    button.style.backgroundColor = value;
    button.title = value;
    button.setAttribute("aria-label", `选择颜色 ${value}`);
    button.addEventListener("click", () => choose(value));
    return button;
  }

  function advancedPicker(initial, confirm, cancel) {
    const root = document.createElement("div");
    root.className = "cp-adv";
    root.innerHTML = `
      <div class="cpa-sv" role="slider" aria-label="饱和度和亮度"><span class="cpa-sv-thumb"></span></div>
      <div class="cpa-hue" role="slider" aria-label="色相"><span class="cpa-hue-thumb"></span></div>
      <div class="cpa-row"><span class="cpa-preview"></span><label class="cpa-hexl">HEX<input class="cpa-hex" maxlength="7" spellcheck="false"></label></div>
      <div class="cpa-row cpa-rgb"><label>R<input class="cpa-r" type="number" min="0" max="255"></label><label>G<input class="cpa-g" type="number" min="0" max="255"></label><label>B<input class="cpa-b" type="number" min="0" max="255"></label></div>
      <input class="cpa-native" type="color" tabindex="-1" aria-hidden="true">
      <div class="cpa-actions"><button type="button" class="cpa-eyedropper" title="从屏幕取色" aria-label="从屏幕取色">⌾</button><button type="button" class="cpa-cancel">取消</button><button type="button" class="cpa-ok">确认</button></div>`;
    const query = (selector) => root.querySelector(selector);
    const sv = query(".cpa-sv");
    const svThumb = query(".cpa-sv-thumb");
    const hueBar = query(".cpa-hue");
    const hueThumb = query(".cpa-hue-thumb");
    const preview = query(".cpa-preview");
    const hexInput = query(".cpa-hex");
    const rgbInputs = [query(".cpa-r"), query(".cpa-g"), query(".cpa-b")];
    const nativeInput = query(".cpa-native");
    const [initialRed, initialGreen, initialBlue] = hexToRgb(initial);
    let [hue, saturation, value] = rgbToHsv(initialRed, initialGreen, initialBlue);
    let current = normalizeHex(initial);

    function sync() {
      const rgb = hsvToRgb(hue, saturation, value);
      current = rgbToHex(...rgb);
      sv.style.backgroundColor = `hsl(${Math.round(hue)},100%,50%)`;
      svThumb.style.left = `${saturation * 100}%`;
      svThumb.style.top = `${(1 - value) * 100}%`;
      hueThumb.style.left = `${hue / 360 * 100}%`;
      preview.style.backgroundColor = current;
      hexInput.value = current;
      hexInput.classList.remove("is-invalid");
      rgbInputs.forEach((input, index) => { input.value = String(Math.round(rgb[index])); });
      nativeInput.value = current;
    }

    function setHex(next) {
      let candidate = String(next || "").trim();
      if (!candidate.startsWith("#")) candidate = `#${candidate}`;
      if (!/^#[0-9a-f]{6}$/i.test(candidate)) return false;
      [hue, saturation, value] = rgbToHsv(...hexToRgb(candidate));
      sync();
      return true;
    }

    function bindDrag(element, update) {
      element.addEventListener("pointerdown", (event) => {
        event.preventDefault();
        element.setPointerCapture?.(event.pointerId);
        const move = (nextEvent) => update(nextEvent);
        update(event);
        element.addEventListener("pointermove", move);
        element.addEventListener("pointerup", () => element.removeEventListener("pointermove", move), { once: true });
      });
    }
    bindDrag(sv, (event) => {
      const rect = sv.getBoundingClientRect();
      saturation = clamp((event.clientX - rect.left) / rect.width, 0, 1);
      value = clamp(1 - (event.clientY - rect.top) / rect.height, 0, 1);
      sync();
    });
    bindDrag(hueBar, (event) => {
      const rect = hueBar.getBoundingClientRect();
      hue = clamp((event.clientX - rect.left) / rect.width * 360, 0, 359.999);
      sync();
    });
    hexInput.addEventListener("input", () => {
      if (hexInput.value.replace(/^#/, "").length === 6) setHex(hexInput.value);
      else hexInput.classList.toggle("is-invalid", Boolean(hexInput.value));
    });
    hexInput.addEventListener("change", () => { if (!setHex(hexInput.value)) sync(); });
    rgbInputs.forEach((input) => input.addEventListener("input", () => {
      const rgb = rgbInputs.map((field) => clamp(Number(field.value) || 0, 0, 255));
      [hue, saturation, value] = rgbToHsv(...rgb);
      sync();
    }));
    nativeInput.addEventListener("input", () => setHex(nativeInput.value));
    query(".cpa-eyedropper").addEventListener("click", async () => {
      if (typeof global.EyeDropper !== "function") return nativeInput.click();
      try {
        const result = await new global.EyeDropper().open();
        if (result?.sRGBHex) setHex(result.sRGBHex);
      } catch (error) {
        if (error?.name !== "AbortError") throw error;
      }
    });
    query(".cpa-cancel").addEventListener("click", cancel);
    query(".cpa-ok").addEventListener("click", () => confirm(current));
    sync();
    return root;
  }

  function open(anchor, options = {}) {
    close();
    if (!anchor) return;
    const choose = (value) => {
      rememberColor(value);
      options.onPick?.(value);
      close();
    };
    popup = document.createElement("div");
    popup.className = "color-pop";
    popup.setAttribute("role", "dialog");
    popup.setAttribute("aria-label", options.label || "选择颜色");
    popup.addEventListener("mousedown", (event) => {
      if (!event.target.closest("input")) event.preventDefault();
    });
    document.body.appendChild(popup);

    function renderPalette() {
      popup.classList.remove("cp-advanced");
      popup.replaceChildren();
      if (options.autoLabel) {
        const automatic = document.createElement("button");
        automatic.type = "button";
        automatic.className = "cp-auto";
        automatic.textContent = options.autoLabel;
        automatic.addEventListener("click", () => choose(options.autoColor ?? "transparent"));
        popup.appendChild(automatic);
      }
      const recent = recentColors();
      if (recent.length) {
        const label = document.createElement("div");
        label.className = "cp-label";
        label.textContent = "最近使用的颜色";
        const row = document.createElement("div");
        row.className = "cp-std cp-recent";
        recent.forEach((color) => row.appendChild(swatch(color, choose)));
        popup.append(label, row);
      }
      const themeLabel = document.createElement("div");
      themeLabel.className = "cp-label";
      themeLabel.textContent = "主题颜色";
      const themeGrid = document.createElement("div");
      themeGrid.className = "cp-grid";
      (options.themeColors || DEFAULT_THEME).slice(0, 10).forEach((base) => {
        const column = document.createElement("div");
        column.className = "cp-col";
        column.appendChild(swatch(normalizeHex(base), choose));
        TINTS.forEach((ratio) => column.appendChild(swatch(tint(base, ratio), choose)));
        themeGrid.appendChild(column);
      });
      const standardLabel = document.createElement("div");
      standardLabel.className = "cp-label";
      standardLabel.textContent = "标准色";
      const standard = document.createElement("div");
      standard.className = "cp-std";
      STANDARD.forEach((color) => standard.appendChild(swatch(color, choose)));
      const more = document.createElement("button");
      more.type = "button";
      more.className = "cp-more";
      more.textContent = "🎨 更多颜色…";
      more.addEventListener("click", renderAdvanced);
      popup.append(themeLabel, themeGrid, standardLabel, standard, more);
      place(anchor);
    }

    function renderAdvanced() {
      popup.classList.add("cp-advanced");
      popup.replaceChildren(advancedPicker(
        normalizeHex(options.current || "#C64224", "#C64224"),
        choose,
        renderPalette,
      ));
      place(anchor);
    }
    renderPalette();
  }

  function attachInput(input, options = {}) {
    if (!input || input.dataset.colorPickerAttached === "true") return null;
    input.dataset.colorPickerAttached = "true";
    input.classList.add("color-input-native");
    const button = document.createElement("button");
    button.type = "button";
    button.className = "color-swatch";
    button.setAttribute("aria-label", options.label || input.getAttribute("aria-label") || "选择颜色");
    input.before(button);
    const sync = () => {
      const value = options.current?.() || input.value || "#000000";
      button.style.backgroundColor = value === "transparent" ? "transparent" : normalizeHex(value);
      button.classList.toggle("is-transparent", value === "transparent");
      button.title = value;
      button.disabled = Boolean(input.disabled);
    };
    button.addEventListener("click", () => open(button, {
      ...options,
      current: options.current?.() || input.value,
      onPick: (value) => {
        if (options.onPick) options.onPick(value);
        else if (value !== "transparent") {
          input.value = normalizeHex(value);
          input.dispatchEvent(new Event("input", { bubbles: true }));
          input.dispatchEvent(new Event("change", { bubbles: true }));
        }
        sync();
      },
    }));
    input.addEventListener("input", sync);
    input.addEventListener("change", sync);
    input._syncColorSwatch = sync;
    sync();
    return button;
  }

  document.addEventListener("mousedown", (event) => {
    if (popup && !popup.contains(event.target) && !event.target.closest?.(".color-swatch,.color-command")) close();
  });
  document.addEventListener("keydown", (event) => { if (event.key === "Escape") close(); });
  global.addEventListener?.("resize", close);

  global.UniPptColorPicker = Object.freeze({
    DEFAULT_THEME,
    STANDARD,
    attachInput,
    close,
    hexToRgb,
    normalizeHex,
    open,
    recentColors,
    rgbToHex,
    tint,
  });
})(globalThis);
