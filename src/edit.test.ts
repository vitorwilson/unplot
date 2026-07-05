import { describe, expect, it } from "vitest";
import {
  clampKnotDrag,
  nearestKnot,
  nearestTangentHandle,
  slopeFromHandleDrag,
  tangentHandleEnd,
} from "./edit";
import type { Knot } from "./fit";
import { worldToScreen, type Viewport } from "./viewport";

const vp: Viewport = { originX: 320, originY: 240, scale: 40 };
const knot = (x: number, y: number, slope = 0): Knot => ({
  x,
  y,
  tangent: null,
  slope,
});

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

describe("clampKnotDrag", () => {
  const knots = [knot(0, 0), knot(1, 0), knot(2, 0)];

  it("keeps an interior knot strictly between its neighbors' x", () => {
    const past = clampKnotDrag(knots, 1, { x: 5, y: 0 }, 1000);
    expect(past.x).toBeGreaterThan(0);
    expect(past.x).toBeLessThan(2);
    const behind = clampKnotDrag(knots, 1, { x: -5, y: 0 }, 1000);
    expect(behind.x).toBeGreaterThan(0);
    expect(behind.x).toBeLessThan(2);
  });

  it("lets an endpoint move freely on its open side", () => {
    expect(clampKnotDrag(knots, 0, { x: -100, y: 0 }, 1000).x).toBeCloseTo(
      -100,
    );
    expect(clampKnotDrag(knots, 2, { x: 100, y: 0 }, 1000).x).toBeCloseTo(100);
  });

  it("limits y so the slope to a neighbor stays within the cap", () => {
    // Endpoint at x=0, neighbor at x=1: max |dy| over dx=1 is the cap.
    const clamped = clampKnotDrag(knots, 0, { x: 0, y: 100 }, 5);
    // Neighbor is (1,0); at x≈0 the reach is ~5, so y is pulled far below 100.
    expect(clamped.y).toBeLessThanOrEqual(5.01);
    expect(clamped.y).toBeGreaterThan(0);
  });

  it("leaves a gentle drag untouched", () => {
    const clamped = clampKnotDrag(knots, 1, { x: 1.2, y: 0.3 }, 1000);
    expect(clamped.x).toBeCloseTo(1.2);
    expect(clamped.y).toBeCloseTo(0.3);
  });
});

describe("tangentHandleEnd", () => {
  it("points straight right for a flat slope", () => {
    const base = worldToScreen(vp, knot(0, 0));
    const end = tangentHandleEnd(knot(0, 0, 0), vp, 40);
    expect(end.x).toBeCloseTo(base.x + 40);
    expect(end.y).toBeCloseTo(base.y);
  });

  it("points up-right for a positive slope (screen y flipped)", () => {
    const base = worldToScreen(vp, knot(0, 0));
    const end = tangentHandleEnd(knot(0, 0, 1), vp, 40);
    expect(end.x).toBeGreaterThan(base.x);
    expect(end.y).toBeLessThan(base.y); // up on screen
  });
});

describe("nearestTangentHandle", () => {
  it("finds the handle end under the pointer", () => {
    const knots = [knot(0, 0, 0), knot(2, 0, 0)];
    const end = tangentHandleEnd(knots[1], vp, 40);
    expect(nearestTangentHandle(knots, vp, end, 40, 8)).toBe(1);
  });

  it("returns null when the pointer is far from every handle", () => {
    const knots = [knot(0, 0, 0)];
    expect(
      nearestTangentHandle(knots, vp, { x: 999, y: 999 }, 40, 8),
    ).toBeNull();
  });
});

describe("slopeFromHandleDrag", () => {
  const origin = { x: 100, y: 100 };

  it("maps an up-and-right drag to a positive slope", () => {
    expect(slopeFromHandleDrag(origin, { x: 140, y: 60 }, 1000)).toBeCloseTo(1);
  });

  it("maps a down-and-right drag to a negative slope", () => {
    expect(slopeFromHandleDrag(origin, { x: 140, y: 140 }, 1000)).toBeCloseTo(
      -1,
    );
  });

  it("clamps to the slope cap", () => {
    expect(slopeFromHandleDrag(origin, { x: 101, y: -900 }, 5)).toBe(5);
  });
});
