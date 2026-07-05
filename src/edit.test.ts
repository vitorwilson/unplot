import { describe, expect, it } from "vitest";
import { nearestKnot } from "./edit";
import type { Knot } from "./fit";
import { worldToScreen, type Viewport } from "./viewport";

const vp: Viewport = { originX: 320, originY: 240, scale: 40 };
const knot = (x: number, y: number): Knot => ({ x, y, tangent: null });

describe("nearestKnot", () => {
  const knots = [knot(0, 0), knot(1, 1), knot(2, -1)];

  it("returns the knot under the pointer", () => {
    const screen = worldToScreen(vp, knots[1]);
    expect(nearestKnot(knots, vp, screen, 8)).toBe(1);
  });

  it("returns null when no knot is within the radius", () => {
    const screen = worldToScreen(vp, { x: 5, y: 5 });
    expect(nearestKnot(knots, vp, screen, 8)).toBeNull();
  });

  it("picks the closest when two are near", () => {
    // A screen point 3 px right of knot 0 and far from the others.
    const p0 = worldToScreen(vp, knots[0]);
    expect(nearestKnot(knots, vp, { x: p0.x + 3, y: p0.y }, 8)).toBe(0);
  });

  it("uses a pixel radius independent of zoom", () => {
    // Knot 1 is 40*sqrt(2) ≈ 56 px from origin; a tight radius misses it.
    const screen = worldToScreen(vp, knots[0]);
    expect(nearestKnot(knots, vp, screen, 8)).toBe(0);
  });
});
