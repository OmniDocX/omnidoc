(() => {
  "use strict";

  const SVG_NS = "http://www.w3.org/2000/svg";
  const DEFAULT_PALETTE = ["#4472c4", "#ed7d31", "#a5a5a5", "#ffc000", "#5b9bd5", "#70ad47"];

  const finite = (value) => {
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
  };
  const clamp = (value, min, max) => Math.max(min, Math.min(max, Number(value)));

  function normalize(chart) {
    const source = chart || {};
    const categories = Array.isArray(source.categories) ? source.categories.map((value) => String(value ?? "")) : [];
    const series = (Array.isArray(source.series) ? source.series : []).map((item, index) => ({
      name: String(item?.name || `Series ${index + 1}`),
      values: (Array.isArray(item?.values) ? item.values : []).map(finite),
      color: validColor(item?.color) ? item.color : DEFAULT_PALETTE[index % DEFAULT_PALETTE.length],
      pointColors: (Array.isArray(item?.pointColors) ? item.pointColors : []).map((color) => validColor(color) ? color : null),
    }));
    const valueCount = Math.max(categories.length, 0, ...series.map((item) => item.values.length));
    while (categories.length < valueCount) categories.push(String(categories.length + 1));
    return {
      chartType: ["line", "bar", "pie", "doughnut", "area"].includes(source.chartType) ? source.chartType : "unknown",
      title: String(source.title || ""),
      legend: {
        visible: Boolean(source.legend?.visible),
        position: ["left", "right", "top", "bottom", "topRight"].includes(source.legend?.position) ? source.legend.position : "right",
        overlay: Boolean(source.legend?.overlay),
      },
      categories,
      series,
      barDirection: source.barDirection === "bar" ? "bar" : "column",
      grouping: source.grouping || null,
      holeSize: clamp(source.holeSize ?? 0.5, 0, 0.95),
    };
  }

  function validColor(value) {
    return typeof value === "string" && /^(#[0-9a-f]{3,8}|rgba?\(|hsla?\(|[a-z]+$)/i.test(value.trim());
  }

  function bounds(series) {
    const values = series.flatMap((item) => item.values).filter(Number.isFinite);
    if (!values.length) return { min: 0, max: 1 };
    let min = Math.min(0, ...values);
    let max = Math.max(0, ...values);
    if (min === max) {
      const pad = Math.abs(min || 1) * 0.1;
      min -= pad;
      max += pad;
    }
    return { min, max };
  }

  function pieSlices(values) {
    const normalized = values.map((value) => Math.max(0, finite(value) || 0));
    const total = normalized.reduce((sum, value) => sum + value, 0);
    let angle = -Math.PI / 2;
    return normalized.map((value, index) => {
      const start = angle;
      const sweep = total > 0 ? value / total * Math.PI * 2 : 0;
      angle += sweep;
      return { index, value, total, start, end: angle, sweep };
    });
  }

  function svgElement(name, attributes = {}, text = null) {
    const node = document.createElementNS(SVG_NS, name);
    for (const [key, value] of Object.entries(attributes)) {
      if (value != null) node.setAttribute(key, String(value));
    }
    if (text != null) node.textContent = String(text);
    return node;
  }

  function render(host, source, options = {}) {
    if (!host) return null;
    const chart = normalize(source);
    const width = Math.max(160, finite(options.width) || finite(host.clientWidth) || 640);
    const height = Math.max(100, finite(options.height) || finite(host.clientHeight) || 360);
    const svg = svgElement("svg", {
      class: "native-chart-svg",
      viewBox: `0 0 ${width} ${height}`,
      width: "100%",
      height: "100%",
      role: "img",
      "aria-label": chart.title || "PowerPoint chart",
      preserveAspectRatio: "none",
    });
    svg.style.display = "block";
    svg.style.overflow = "hidden";
    svg.append(svgElement("rect", { x: 0, y: 0, width, height, fill: "transparent" }));

    const titleHeight = chart.title ? Math.min(42, Math.max(24, height * 0.11)) : 0;
    if (chart.title) {
      svg.append(svgElement("text", {
        x: width / 2,
        y: titleHeight * 0.72,
        "text-anchor": "middle",
        fill: "#222",
        "font-family": "Aptos, Segoe UI, sans-serif",
        "font-size": Math.max(12, Math.min(20, height * 0.055)),
        "font-weight": "600",
      }, chart.title));
    }

    const legend = legendLayout(chart, width, height, titleHeight);
    const plot = legend.plot;
    if (chart.chartType === "pie" || chart.chartType === "doughnut") {
      renderPie(svg, chart, plot);
    } else if (chart.chartType === "line" || chart.chartType === "area") {
      renderCartesianFrame(svg, chart, plot, false);
      renderLines(svg, chart, plot, chart.chartType === "area");
    } else if (chart.chartType === "bar") {
      renderCartesianFrame(svg, chart, plot, chart.barDirection === "bar");
      renderBars(svg, chart, plot);
    } else {
      renderUnknown(svg, plot);
    }
    if (chart.legend.visible && chart.series.length) renderLegend(svg, chart, legend.box);

    host.style.padding = "0";
    host.style.overflow = "hidden";
    host.append(svg);
    return svg;
  }

  function legendLayout(chart, width, height, titleHeight) {
    const base = { x: 44, y: titleHeight + 18, width: Math.max(1, width - 64), height: Math.max(1, height - titleHeight - 44) };
    if (!chart.legend.visible || chart.legend.overlay || !chart.series.length) return { plot: base, box: null };
    const position = chart.legend.position;
    if (position === "left" || position === "right" || position === "topRight") {
      const legendWidth = Math.min(width * 0.28, 150);
      return {
        plot: { ...base, x: position === "left" ? base.x + legendWidth : base.x, width: Math.max(1, base.width - legendWidth) },
        box: { x: position === "left" ? 8 : width - legendWidth + 4, y: titleHeight + 14, width: legendWidth - 8, height: height - titleHeight - 24, vertical: true },
      };
    }
    const legendHeight = Math.min(34, height * 0.13);
    return {
      plot: { ...base, y: position === "top" ? base.y + legendHeight : base.y, height: Math.max(1, base.height - legendHeight) },
      box: { x: 12, y: position === "top" ? titleHeight + 4 : height - legendHeight, width: width - 24, height: legendHeight, vertical: false },
    };
  }

  function renderLegend(svg, chart, box) {
    if (!box) return;
    const count = chart.series.length;
    chart.series.forEach((item, index) => {
      const x = box.vertical ? box.x + 4 : box.x + index * box.width / count + 4;
      const y = box.vertical ? box.y + 16 + index * Math.min(22, box.height / Math.max(1, count)) : box.y + box.height / 2;
      svg.append(svgElement("rect", { x, y: y - 7, width: 10, height: 10, fill: item.color }));
      svg.append(svgElement("text", {
        x: x + 15, y: y + 2, fill: "#444", "font-size": 11,
        "font-family": "Aptos, Segoe UI, sans-serif",
      }, shorten(item.name, box.vertical ? 19 : Math.max(6, Math.floor(box.width / count / 8)))));
    });
  }

  function renderCartesianFrame(svg, chart, plot, horizontal) {
    const group = svgElement("g", { class: "native-chart-grid" });
    const range = bounds(chart.series);
    for (let tick = 0; tick <= 4; tick += 1) {
      const ratio = tick / 4;
      const x = horizontal ? plot.x + ratio * plot.width : plot.x;
      const y = horizontal ? plot.y : plot.y + (1 - ratio) * plot.height;
      group.append(svgElement("line", {
        x1: x, y1: y, x2: horizontal ? x : plot.x + plot.width, y2: horizontal ? plot.y + plot.height : y,
        stroke: tick === 0 ? "#9e9e9e" : "#dedede", "stroke-width": 1,
      }));
      const value = horizontal ? range.min + ratio * (range.max - range.min) : range.min + ratio * (range.max - range.min);
      group.append(svgElement("text", {
        x: horizontal ? x : plot.x - 6,
        y: horizontal ? plot.y + plot.height + 15 : y + 4,
        "text-anchor": horizontal ? "middle" : "end",
        fill: "#666", "font-size": 10, "font-family": "Aptos, Segoe UI, sans-serif",
      }, formatValue(value)));
    }
    const count = Math.max(1, chart.categories.length);
    chart.categories.forEach((category, index) => {
      const ratio = (index + 0.5) / count;
      group.append(svgElement("text", {
        x: horizontal ? plot.x - 7 : plot.x + ratio * plot.width,
        y: horizontal ? plot.y + ratio * plot.height + 4 : plot.y + plot.height + 15,
        "text-anchor": horizontal ? "end" : "middle",
        fill: "#555", "font-size": 10, "font-family": "Aptos, Segoe UI, sans-serif",
      }, shorten(category, horizontal ? 12 : Math.max(4, Math.floor(plot.width / count / 7)))));
    });
    svg.append(group);
  }

  function renderLines(svg, chart, plot, area) {
    const range = bounds(chart.series);
    const count = Math.max(1, chart.categories.length, ...chart.series.map((item) => item.values.length));
    const xAt = (index) => plot.x + (count === 1 ? plot.width / 2 : index / (count - 1) * plot.width);
    const yAt = (value) => plot.y + (range.max - value) / (range.max - range.min) * plot.height;
    const zeroY = yAt(clamp(0, range.min, range.max));
    chart.series.forEach((item) => {
      const segments = splitSegments(item.values);
      segments.forEach((segment) => {
        const points = segment.map(({ index, value }) => `${xAt(index)},${yAt(value)}`).join(" ");
        if (area && segment.length > 1) {
          const first = segment[0];
          const last = segment[segment.length - 1];
          svg.append(svgElement("polygon", {
            points: `${xAt(first.index)},${zeroY} ${points} ${xAt(last.index)},${zeroY}`,
            fill: item.color, opacity: 0.28,
          }));
        }
        svg.append(svgElement("polyline", {
          points, fill: "none", stroke: item.color, "stroke-width": 2.2,
          "stroke-linejoin": "round", "stroke-linecap": "round",
        }));
        segment.forEach(({ index, value }) => svg.append(svgElement("circle", {
          cx: xAt(index), cy: yAt(value), r: 3.2, fill: item.color, stroke: "#fff", "stroke-width": 1,
        })));
      });
    });
  }

  function renderBars(svg, chart, plot) {
    const range = bounds(chart.series);
    const categories = Math.max(1, chart.categories.length, ...chart.series.map((item) => item.values.length));
    const seriesCount = Math.max(1, chart.series.length);
    const horizontal = chart.barDirection === "bar";
    const zero = (0 - range.min) / (range.max - range.min);
    chart.series.forEach((item, seriesIndex) => {
      item.values.forEach((value, index) => {
        if (!Number.isFinite(value)) return;
        const ratio = (value - range.min) / (range.max - range.min);
        const groupSpan = (horizontal ? plot.height : plot.width) / categories;
        const barSpan = groupSpan * 0.78 / seriesCount;
        if (horizontal) {
          const x0 = plot.x + Math.min(zero, ratio) * plot.width;
          svg.append(svgElement("rect", {
            x: x0, y: plot.y + index * groupSpan + groupSpan * 0.11 + seriesIndex * barSpan,
            width: Math.max(0.5, Math.abs(ratio - zero) * plot.width), height: Math.max(1, barSpan - 1), fill: item.color,
          }));
        } else {
          const y0 = plot.y + (1 - Math.max(zero, ratio)) * plot.height;
          svg.append(svgElement("rect", {
            x: plot.x + index * groupSpan + groupSpan * 0.11 + seriesIndex * barSpan, y: y0,
            width: Math.max(1, barSpan - 1), height: Math.max(0.5, Math.abs(ratio - zero) * plot.height), fill: item.color,
          }));
        }
      });
    });
  }

  function renderPie(svg, chart, plot) {
    const series = chart.series[0];
    if (!series) return renderUnknown(svg, plot, "No chart data");
    const slices = pieSlices(series.values);
    const radius = Math.max(1, Math.min(plot.width, plot.height) * 0.45);
    const cx = plot.x + plot.width / 2;
    const cy = plot.y + plot.height / 2;
    if (!slices.some((slice) => slice.sweep > 0)) return renderUnknown(svg, plot, "No chart data");
    slices.forEach((slice) => {
      if (slice.sweep <= 0) return;
      const color = series.pointColors[slice.index] || DEFAULT_PALETTE[slice.index % DEFAULT_PALETTE.length] || series.color;
      const fullCircle = slice.sweep >= Math.PI * 2 - 1e-8;
      if (fullCircle) {
        svg.append(svgElement("circle", { cx, cy, r: radius, fill: color, stroke: "#fff", "stroke-width": 1 }));
      } else {
        svg.append(svgElement("path", {
          d: wedgePath(cx, cy, radius, slice.start, slice.end), fill: color, stroke: "#fff", "stroke-width": 1,
        }));
      }
    });
    if (chart.chartType === "doughnut") {
      svg.append(svgElement("circle", { cx, cy, r: radius * chart.holeSize, fill: "white" }));
    }
  }

  function wedgePath(cx, cy, radius, start, end) {
    const x1 = cx + Math.cos(start) * radius;
    const y1 = cy + Math.sin(start) * radius;
    const x2 = cx + Math.cos(end) * radius;
    const y2 = cy + Math.sin(end) * radius;
    const large = end - start > Math.PI ? 1 : 0;
    return `M ${cx} ${cy} L ${x1} ${y1} A ${radius} ${radius} 0 ${large} 1 ${x2} ${y2} Z`;
  }

  function splitSegments(values) {
    const segments = [];
    let active = [];
    values.forEach((value, index) => {
      if (Number.isFinite(value)) active.push({ index, value });
      else if (active.length) {
        segments.push(active);
        active = [];
      }
    });
    if (active.length) segments.push(active);
    return segments;
  }

  function renderUnknown(svg, plot, label = "Unsupported native chart") {
    svg.append(svgElement("rect", {
      x: plot.x, y: plot.y, width: plot.width, height: plot.height,
      rx: 4, fill: "#f7f7f7", stroke: "#c9c9c9", "stroke-dasharray": "4 3",
    }));
    svg.append(svgElement("text", {
      x: plot.x + plot.width / 2, y: plot.y + plot.height / 2,
      "text-anchor": "middle", fill: "#777", "font-size": 13,
      "font-family": "Aptos, Segoe UI, sans-serif",
    }, label));
  }

  function shorten(value, limit) {
    const text = String(value ?? "");
    return text.length > limit ? `${text.slice(0, Math.max(1, limit - 1))}…` : text;
  }

  function formatValue(value) {
    const magnitude = Math.abs(value);
    if (magnitude >= 1e6) return `${(value / 1e6).toFixed(1)}m`;
    if (magnitude >= 1e3) return `${(value / 1e3).toFixed(1)}k`;
    return Number(value.toFixed(2)).toString();
  }

  const api = { render, normalize, _test: { bounds, pieSlices, splitSegments, wedgePath } };
  globalThis.UniPPTChartRuntime = api;
  // Backward-compatible short alias for early UniPPT editor builds.
  globalThis.UniPptChart = api;
  if (typeof module !== "undefined" && module.exports) module.exports = api;
})();
