import { describe, expect, it } from "vitest";
import { visibleGridLines } from "./grid";
import { type Viewport } from "./viewport";

const centered: Viewport = { originX: 320, originY: 240, scale: 40 };

describe("visibleGridLines", () => {
  it("lists the integer gridlines on screen for a centered viewport", () => {
    const { xs, ys } = visibleGridLines(centered, 640, 480, 1);
    // 320 px / 40 = 8 units each side of x; 240 / 40 = 6 units for y.
    expect(xs).toHaveLength(17);
    expect(xs[0]).toBe(-8);
    expect(xs[xs.length - 1]).toBe(8);
    expect(ys).toHaveLength(13);
    expect(ys[0]).toBe(-6);
    expect(ys[ys.length - 1]).toBe(6);
  });

  it("respects a coarser step", () => {
    const { xs } = visibleGridLines(centered, 640, 480, 2);
    expect(xs).toEqual([-8, -6, -4, -2, 0, 2, 4, 6, 8]);
  });

  it("rejects a non-positive step", () => {
    expect(() => visibleGridLines(centered, 640, 480, 0)).toThrow(/positive/);
  });
});
