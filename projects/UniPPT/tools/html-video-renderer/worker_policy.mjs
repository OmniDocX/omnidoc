import { availableParallelism, cpus } from "node:os";

export const MAX_RENDER_WORKERS = 8;

export function detectedParallelism() {
  try {
    return Math.max(1, Number(availableParallelism()) || 1);
  } catch (_) {
    return Math.max(1, cpus()?.length || 1);
  }
}

export function renderWorkerCount(slideCount, override, parallelism = detectedParallelism()) {
  const explicit = Number(override);
  const requested = Number.isFinite(explicit) && explicit > 0
    ? Math.floor(explicit)
    : Math.max(1, Math.floor(Number(parallelism) || 1));
  return Math.max(1, Math.min(Math.max(1, Number(slideCount) || 1), requested, MAX_RENDER_WORKERS));
}
