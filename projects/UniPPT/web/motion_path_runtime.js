(() => {
  "use strict";

  const NUMBER = /^[-+]?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?$/i;
  const COMMAND = /^[MLCQZE]$/i;
  const MAX_PATH_LENGTH = 32768;
  const MAX_TOKENS = 4096;
  const MAX_COORDINATE = 100;

  function tokenize(source) {
    if (typeof source !== "string" || !source.trim() || source.length > MAX_PATH_LENGTH) return null;
    const tokens = source.match(/[MLCQZE]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g);
    if (!tokens?.length || tokens.length > MAX_TOKENS) return null;
    const residue = source.replace(/[MLCQZE]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?|[\s,]+/g, "");
    return residue ? null : tokens;
  }

  function coordinate(token) {
    if (!NUMBER.test(token || "")) return null;
    const value = Number(token);
    return Number.isFinite(value) && Math.abs(value) <= MAX_COORDINATE ? value : null;
  }

  function interpolateLine(from, to, count = 12) {
    return Array.from({ length: count }, (_, index) => {
      const t = (index + 1) / count;
      return { x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t };
    });
  }

  function interpolateCurve(from, controls, to, count) {
    return Array.from({ length: count }, (_, index) => {
      const t = (index + 1) / count;
      const u = 1 - t;
      if (controls.length === 1) {
        const [control] = controls;
        return {
          x: u * u * from.x + 2 * u * t * control.x + t * t * to.x,
          y: u * u * from.y + 2 * u * t * control.y + t * t * to.y,
        };
      }
      const [first, second] = controls;
      return {
        x: u ** 3 * from.x + 3 * u * u * t * first.x + 3 * u * t * t * second.x + t ** 3 * to.x,
        y: u ** 3 * from.y + 3 * u * u * t * first.y + 3 * u * t * t * second.y + t ** 3 * to.y,
      };
    });
  }

  function parse(source) {
    const tokens = tokenize(source);
    if (!tokens) return null;
    let index = 0;
    let command = null;
    let current = null;
    let subpathStart = null;
    const points = [];
    const readPoint = () => {
      const x = coordinate(tokens[index++]);
      const y = coordinate(tokens[index++]);
      return x == null || y == null ? null : { x, y };
    };
    while (index < tokens.length) {
      if (COMMAND.test(tokens[index])) command = tokens[index++].toUpperCase();
      if (!command) return null;
      if (command === "E") break;
      if (command === "Z") {
        if (!current || !subpathStart) return null;
        points.push(...interpolateLine(current, subpathStart));
        current = { ...subpathStart };
        command = null;
        continue;
      }
      if (command === "M") {
        const point = readPoint();
        if (!point) return null;
        current = point;
        subpathStart = { ...point };
        points.push(point);
        command = "L";
        continue;
      }
      if (!current) return null;
      if (command === "L") {
        const point = readPoint();
        if (!point) return null;
        points.push(...interpolateLine(current, point));
        current = point;
      } else if (command === "Q") {
        const control = readPoint();
        const point = readPoint();
        if (!control || !point) return null;
        points.push(...interpolateCurve(current, [control], point, 16));
        current = point;
      } else if (command === "C") {
        const first = readPoint();
        const second = readPoint();
        const point = readPoint();
        if (!first || !second || !point) return null;
        points.push(...interpolateCurve(current, [first, second], point, 24));
        current = point;
      } else {
        return null;
      }
      if (index < tokens.length && !COMMAND.test(tokens[index])) continue;
      command = null;
    }
    return points.length >= 2 ? points : null;
  }

  function frames(source, slideWidth, slideHeight, baseTransform = "") {
    const points = parse(source);
    const width = Number(slideWidth);
    const height = Number(slideHeight);
    if (!points || !(width > 0) || !(height > 0)) return null;
    const last = Math.max(1, points.length - 1);
    return points.map((point, index) => ({
      offset: index / last,
      transform: `${baseTransform} translate(${point.x * width}px,${point.y * height}px)`,
    }));
  }

  globalThis.UniPptMotionPath = Object.freeze({ parse, frames });
})();
