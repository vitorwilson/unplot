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

// A tangent handle can't point straight up (infinite slope); keep its screen dx
// at least this many pixels so a near-vertical drag maps to a large finite slope
// (then clamped to the cap).
const MIN_HANDLE_DX = 4;

/**
 * Screen position of a knot's tangent-handle end: `lengthPx` from the knot along
 * its slope direction (forward in x). Screen y is flipped, so a positive slope
 * points up-right.
 */
export function tangentHandleEnd(
  knot: Knot,
  vp: Viewport,
  lengthPx: number,
): Point {
  const base = worldToScreen(vp, knot);
  const dx = 1;
  const dy = -knot.slope; // world (1, slope) with the screen y axis flipped
  const norm = Math.hypot(dx, dy);
  return {
    x: base.x + (dx / norm) * lengthPx,
    y: base.y + (dy / norm) * lengthPx,
  };
}

/** Index of the knot whose tangent-handle end is under `screen` within
 * `radiusPx`, or `null`. Checked before knot hit-testing so the handle wins. */
export function nearestTangentHandle(
  knots: readonly Knot[],
  vp: Viewport,
  screen: Point,
  lengthPx: number,
  radiusPx: number,
): number | null {
  let best: number | null = null;
  let bestDist = radiusPx;
  knots.forEach((knot, index) => {
    const end = tangentHandleEnd(knot, vp, lengthPx);
    const dist = Math.hypot(end.x - screen.x, end.y - screen.y);
    if (dist <= bestDist) {
      bestDist = dist;
      best = index;
    }
  });
  return best;
}

/**
 * Slope implied by dragging a tangent handle to `pointer`, given its knot's
 * screen position. The handle is kept pointing forward in x, and the result is
 * clamped to ±`maxSlope` (the no-spike rule applied to slope edits).
 */
export function slopeFromHandleDrag(
  knotScreen: Point,
  pointer: Point,
  maxSlope: number,
): number {
  const dx = Math.max(pointer.x - knotScreen.x, MIN_HANDLE_DX);
  const dy = pointer.y - knotScreen.y;
  return clamp(-dy / dx, -maxSlope, maxSlope); // screen y flipped
}
