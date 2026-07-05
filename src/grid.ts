import { screenToWorld, type Viewport } from "./viewport";

/** World coordinates of the gridlines currently on screen. */
export interface GridLines {
  xs: number[];
  ys: number[];
}

/**
 * World coordinates of every gridline (a multiple of `step`) visible in a
 * `width`×`height` CSS-pixel canvas under `vp`. Used to draw the Cartesian grid
 * without walking off-screen coordinates.
 */
export function visibleGridLines(
  vp: Viewport,
  width: number,
  height: number,
  step: number,
): GridLines {
  if (step <= 0) {
    throw new Error(`visibleGridLines: step must be positive, got ${step}`);
  }
  // Screen (0,0) is the top-left: minimum world x, maximum world y.
  const topLeft = screenToWorld(vp, { x: 0, y: 0 });
  const bottomRight = screenToWorld(vp, { x: width, y: height });
  return {
    xs: multiplesInRange(topLeft.x, bottomRight.x, step),
    ys: multiplesInRange(bottomRight.y, topLeft.y, step),
  };
}

function multiplesInRange(min: number, max: number, step: number): number[] {
  const out: number[] = [];
  for (let k = Math.ceil(min / step); k <= Math.floor(max / step); k++) {
    out.push(k * step);
  }
  return out;
}

/**
 * A "nice" world-space grid step (1, 2, or 5 × a power of ten) giving roughly
 * `targetPx` pixels between gridlines at the current `scale`. Keeps the grid
 * legible as the user zooms.
 *
 * @example
 * tickStep(40, 80); // 2 — at 40 px/unit, 2-unit spacing is ~80 px
 */
export function tickStep(scale: number, targetPx: number): number {
  if (scale <= 0 || targetPx <= 0) {
    throw new Error(
      `tickStep: scale and targetPx must be positive, got ${scale}, ${targetPx}`,
    );
  }
  const rawStep = targetPx / scale;
  const magnitude = 10 ** Math.floor(Math.log10(rawStep));
  const normalized = rawStep / magnitude;
  const niceNormalized = normalized < 1.5 ? 1 : normalized < 3.5 ? 2 : 5;
  return niceNormalized * magnitude;
}
