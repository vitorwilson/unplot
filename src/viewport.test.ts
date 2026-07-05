import { describe, expect, it } from "vitest";
import {
  pan,
  screenToWorld,
  worldToScreen,
  zoomAt,
  type Viewport,
} from "./viewport";

const vp: Viewport = { originX: 320, originY: 240, scale: 40 };

describe("worldToScreen / screenToWorld", () => {
  it("maps the world origin to its screen pixel", () => {
    expect(worldToScreen(vp, { x: 0, y: 0 })).toEqual({ x: 320, y: 240 });
  });

  it("flips the y axis so world-up is screen-up", () => {
    expect(worldToScreen(vp, { x: 0, y: 1 })).toEqual({ x: 320, y: 200 });
  });

  it("round-trips screen -> world -> screen", () => {
    const screen = { x: 123, y: 456 };
    const back = worldToScreen(vp, screenToWorld(vp, screen));
    expect(back.x).toBeCloseTo(123, 9);
    expect(back.y).toBeCloseTo(456, 9);
  });
});

describe("pan", () => {
  it("shifts the origin by the screen delta and keeps scale", () => {
    const moved = pan(vp, 10, -5);
    expect(moved).toEqual({ originX: 330, originY: 235, scale: 40 });
  });
});

describe("zoomAt", () => {
  it("keeps the world point under the pivot fixed", () => {
    const pivot = { x: 500, y: 100 };
    const before = screenToWorld(vp, pivot);
    const after = screenToWorld(zoomAt(vp, pivot, 2), pivot);
    expect(after.x).toBeCloseTo(before.x, 9);
    expect(after.y).toBeCloseTo(before.y, 9);
  });

  it("scales by the factor", () => {
    expect(zoomAt(vp, { x: 0, y: 0 }, 1.5).scale).toBeCloseTo(60, 9);
  });

  it("rejects a non-positive factor", () => {
    expect(() => zoomAt(vp, { x: 0, y: 0 }, 0)).toThrow(/positive/);
  });
});
