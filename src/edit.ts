import type { Knot } from "./fit";
import { worldToScreen, type Point, type Viewport } from "./viewport";

// Pure geometry for editing the curve: which knot (if any) the pointer grabbed.
// Kept separate from the DOM wiring in draw.ts so it can be unit-tested.

/**
 * Index of the knot whose screen position is nearest `screen` and within
 * `radiusPx`, or `null` if the pointer isn't over any knot. Hit-testing is done
 * in screen space so the grab radius is a constant pixel distance at any zoom.
 *
 * @example
 * nearestKnot([{ x: 0, y: 0, tangent: null }], vp, screenOfOrigin, 8); // 0
 */
export function nearestKnot(
  knots: readonly Knot[],
  vp: Viewport,
  screen: Point,
  radiusPx: number,
): number | null {
  let best: number | null = null;
  let bestDist = radiusPx;
  knots.forEach((knot, index) => {
    const p = worldToScreen(vp, knot);
    const dist = Math.hypot(p.x - screen.x, p.y - screen.y);
    if (dist <= bestDist) {
      bestDist = dist;
      best = index;
    }
  });
  return best;
}

// Keep a moved knot strictly between its neighbors' x by this fraction of the
// gap, so the curve stays a valid (strictly-increasing) function during a drag.
const X_MARGIN = 1e-3;

/**
 * Clamp a drag `target` for the knot at `index` so the curve stays valid: x is
 * held strictly between the neighbors' x, and y is limited so the slope to each
 * neighbor never exceeds `maxSlope` (the no-spike hard-block, applied to edits).
 * Endpoints are constrained only by their single inner neighbor.
 */
export function clampKnotDrag(
  knots: readonly Knot[],
  index: number,
  target: Point,
  maxSlope: number,
): Point {
  const prev = knots[index - 1];
  const next = knots[index + 1];
  const x = clamp(
    target.x,
    prev ? prev.x + X_MARGIN * (next ? next.x - prev.x : 1) : -Infinity,
    next ? next.x - X_MARGIN * (prev ? next.x - prev.x : 1) : Infinity,
  );

  let yLo = -Infinity;
  let yHi = Infinity;
  for (const neighbor of [prev, next]) {
    if (!neighbor) {
      continue;
    }
    const reach = maxSlope * Math.abs(x - neighbor.x);
    yLo = Math.max(yLo, neighbor.y - reach);
    yHi = Math.min(yHi, neighbor.y + reach);
  }
  return { x, y: clamp(target.y, yLo, yHi) };
}

function clamp(value: number, lo: number, hi: number): number {
  return Math.min(Math.max(value, lo), hi);
}
