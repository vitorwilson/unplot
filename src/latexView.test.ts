import { describe, expect, it } from "vitest";
import { summaryLabel } from "./latexView";

describe("summaryLabel", () => {
  it("shows a collapsed triangle when not expanded", () => {
    expect(summaryLabel("3-segment spline over [0, 2]", false)).toBe(
      "▸ 3-segment spline over [0, 2]",
    );
  });

  it("shows an open triangle when expanded", () => {
    expect(summaryLabel("3-segment spline over [0, 2]", true)).toBe(
      "▾ 3-segment spline over [0, 2]",
    );
  });
});
