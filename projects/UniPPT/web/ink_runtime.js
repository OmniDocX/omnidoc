"use strict";

// Geometry-only ink runtime.  The algorithms are adapted from the UniDoc
// mother project, while storage remains native UniPPT scene objects so HTML,
// UDoc and PPTX all consume the same editable representation.
(function installUniPptInk(global) {
  const clonePoint = (point) => [Number(point[0]) || 0, Number(point[1]) || 0, pressure(point[2])];
  const pressure = (value) => Number.isFinite(Number(value)) ? Math.max(0.02, Math.min(1, Number(value))) : 0.5;

  function simplify(points, epsilon = 1.5) {
    if ((points || []).length <= 2) return (points || []).map(clonePoint);
    const first = points[0], last = points.at(-1);
    const dx = last[0] - first[0], dy = last[1] - first[1], length = Math.hypot(dx, dy) || 1;
    let distance = 0, index = 0;
    for (let i = 1; i < points.length - 1; i += 1) {
      const current = Math.abs(dy * points[i][0] - dx * points[i][1] + last[0] * first[1] - last[1] * first[0]) / length;
      if (current > distance) { distance = current; index = i; }
    }
    if (distance <= epsilon) return [clonePoint(first), clonePoint(last)];
    return simplify(points.slice(0, index + 1), epsilon).slice(0, -1)
      .concat(simplify(points.slice(index), epsilon));
  }

  function bounds(points) {
    if (!points?.length) return null;
    const xs = points.map((point) => point[0]), ys = points.map((point) => point[1]);
    const x = Math.min(...xs), y = Math.min(...ys), right = Math.max(...xs), bottom = Math.max(...ys);
    return { x, y, right, bottom, width: Math.max(0.1, right - x), height: Math.max(0.1, bottom - y) };
  }

  function pointsToPath(points, close = false) {
    if (!points?.length) return "";
    const commands = points.map((point, index) => `${index ? "L" : "M"}${round(point[0])} ${round(point[1])}`);
    return `${commands.join(" ")}${close ? " Z" : ""}`;
  }

  function pathToPoints(path) {
    const points = [];
    const expression = /[ML]\s*(-?\d+(?:\.\d+)?)\s+(-?\d+(?:\.\d+)?)/gi;
    let match;
    while ((match = expression.exec(path || ""))) points.push([Number(match[1]), Number(match[2]), 0.5]);
    return points;
  }

  function recognizeShape(rawPoints) {
    if (!rawPoints || rawPoints.length < 2) return null;
    const points = rawPoints.map(clonePoint), box = bounds(points);
    const diagonal = Math.hypot(box.width, box.height), start = points[0], end = points.at(-1);
    if (diagonal < 12) return null;
    const closed = Math.hypot(end[0] - start[0], end[1] - start[1]) < Math.max(16, diagonal * 0.2);
    if (!closed && simplify(points, Math.max(4, diagonal * 0.04)).length === 2) {
      return { kind: "line", points: [start, end], closed: false };
    }
    if (points.length < 4 || !closed) return null;
    const cx = box.x + box.width / 2, cy = box.y + box.height / 2;
    const ellipseError = points.reduce((sum, point) => {
      const nx = (point[0] - cx) / Math.max(1, box.width / 2);
      const ny = (point[1] - cy) / Math.max(1, box.height / 2);
      return sum + Math.abs(Math.hypot(nx, ny) - 1);
    }, 0) / points.length;
    if (ellipseError < 0.22) return { kind: "ellipse", box, closed: true };
    const edgeError = points.reduce((sum, point) => {
      const horizontal = Math.min(Math.abs(point[1] - box.y), Math.abs(point[1] - box.bottom)) / Math.max(1, box.height);
      const vertical = Math.min(Math.abs(point[0] - box.x), Math.abs(point[0] - box.right)) / Math.max(1, box.width);
      return sum + Math.min(horizontal, vertical);
    }, 0) / points.length;
    if (edgeError < 0.08) return { kind: "rectangle", box, closed: true };
    const corners = simplify(points.concat([points[0]]), Math.max(6, diagonal * 0.06)).slice(0, -1);
    if (corners.length === 3) return { kind: "triangle", points: corners.concat([corners[0]]), closed: true };
    if (corners.length >= 4 && corners.length <= 6) return { kind: "rectangle", box, closed: true };
    return null;
  }

  function segmentDistance(point, start, end) {
    const dx = end[0] - start[0], dy = end[1] - start[1];
    const divisor = dx * dx + dy * dy;
    const t = divisor ? Math.max(0, Math.min(1, ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / divisor)) : 0;
    return Math.hypot(point[0] - (start[0] + t * dx), point[1] - (start[1] + t * dy));
  }

  function hitTest(points, point, radius) {
    for (let index = 1; index < points.length; index += 1) {
      if (segmentDistance(point, points[index - 1], points[index]) <= radius) return true;
    }
    return points.length === 1 && Math.hypot(point[0] - points[0][0], point[1] - points[0][1]) <= radius;
  }

  function round(value) { return Math.round(Number(value) * 10) / 10; }

  global.UniPptInk = Object.freeze({ simplify, bounds, pointsToPath, pathToPoints, recognizeShape, hitTest });
})(globalThis);
