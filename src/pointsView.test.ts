import { describe, expect, it } from "vitest";
import { pointsToggleLabel } from "./pointsView";

describe("pointsToggleLabel", () => {
  it("shows a collapsed triangle when closed", () => {
    expect(pointsToggleLabel(false)).toBe("▸ Points");
  });

  it("shows an open triangle when expanded", () => {
    expect(pointsToggleLabel(true)).toBe("▾ Points");
  });
});
