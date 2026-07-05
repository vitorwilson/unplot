import { describe, expect, it } from "vitest";
import { advancesInX, StrokeBuilder, withinSlopeCap } from "./stroke";

describe("advancesInX", () => {
  it("is true only when x strictly increases", () => {
    expect(advancesInX(0, 1)).toBe(true);
    expect(advancesInX(1, 1)).toBe(false);
    expect(advancesInX(1, 0.5)).toBe(false);
  });
});

describe("withinSlopeCap", () => {
  it("accepts a gentle forward step", () => {
    expect(withinSlopeCap({ x: 0, y: 0 }, { x: 1, y: 1 }, 5)).toBe(true);
  });

  it("rejects a backward step", () => {
    expect(withinSlopeCap({ x: 1, y: 0 }, { x: 0.5, y: 0 }, 5)).toBe(false);
  });

  it("rejects a spike over the cap", () => {
    expect(withinSlopeCap({ x: 0, y: 0 }, { x: 0.1, y: 10 }, 5)).toBe(false);
  });
});

describe("StrokeBuilder", () => {
  it("accepts the first point unconditionally", () => {
    const stroke = new StrokeBuilder(5);
    expect(stroke.tryAdd({ x: 3, y: 9 })).toBe(true);
    expect(stroke.length).toBe(1);
  });

  it("keeps only points that clear the hard-block", () => {
    const stroke = new StrokeBuilder(5);
    stroke.tryAdd({ x: 0, y: 0 });
    expect(stroke.tryAdd({ x: 1, y: 1 })).toBe(true);
    expect(stroke.tryAdd({ x: 0.5, y: 1 })).toBe(false); // backward in x
    expect(stroke.tryAdd({ x: 1.1, y: 20 })).toBe(false); // spike
    expect(stroke.tryAdd({ x: 2, y: 2 })).toBe(true);
    expect(stroke.samples()).toEqual([
      { x: 0, y: 0 },
      { x: 1, y: 1 },
      { x: 2, y: 2 },
    ]);
  });

  it("gates the first resumed point against the anchor", () => {
    const stroke = new StrokeBuilder(5, { x: 2, y: 1 });
    expect(stroke.tryAdd({ x: 1.5, y: 1 })).toBe(false); // behind the anchor
    expect(stroke.tryAdd({ x: 2, y: 1 })).toBe(false); // exactly at the anchor
    expect(stroke.tryAdd({ x: 2.5, y: 1.2 })).toBe(true); // ahead of it
    expect(stroke.samples()).toEqual([{ x: 2.5, y: 1.2 }]);
  });
});
