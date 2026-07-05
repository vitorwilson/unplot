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
