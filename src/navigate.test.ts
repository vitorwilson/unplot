import { describe, expect, it } from "vitest";
import { interpretWheel } from "./navigate";

describe("interpretWheel", () => {
  it("pans on a plain two-finger scroll, opposite the delta", () => {
    const action = interpretWheel(false, 5, 10, 0);
    expect(action.kind).toBe("pan");
    expect(action.dx).toBe(-5);
    expect(action.dy).toBe(-10);
  });

  it("zooms in when pinching apart (ctrlKey, negative deltaY)", () => {
    const action = interpretWheel(true, 0, -10, 0);
    expect(action.kind).toBe("zoom");
    expect(action.factor).toBeGreaterThan(1);
  });

  it("zooms out when pinching together (ctrlKey, positive deltaY)", () => {
    const action = interpretWheel(true, 0, 10, 0);
    expect(action.kind).toBe("zoom");
    expect(action.factor).toBeLessThan(1);
  });

  it("scales line-mode deltas to pixels", () => {
    // deltaMode 1 (lines): a 3-line pan should move ~3*16 px.
    const action = interpretWheel(false, 0, 3, 1);
    expect(action.dy).toBe(-48);
  });
});
