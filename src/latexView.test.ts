import { describe, expect, it } from "vitest";
import { copyLabel, formatText, summaryLabel } from "./latexView";
import type { CurveLatex } from "./fit";

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

describe("copyLabel", () => {
  it("names the chosen target so the user sees what will copy", () => {
    expect(copyLabel("latex")).toBe("Copy LaTeX");
    expect(copyLabel("desmos")).toBe("Copy for Desmos");
    expect(copyLabel("wolfram")).toBe("Copy for Wolfram");
  });
});

describe("formatText", () => {
  const result: CurveLatex = {
    summary: "1-segment spline over [0, 2]",
    latex: "f(x) = \\begin{cases} 2x & 0 \\le x \\le 2 \\end{cases}",
    desmos: "\\left\\{0 \\le x \\le 2: 2x\\right\\}",
    wolfram: "Piecewise[{{2x, 0 <= x <= 2}}]",
  };

  it("picks the field matching the chosen target", () => {
    expect(formatText(result, "latex")).toBe(result.latex);
    expect(formatText(result, "desmos")).toBe(result.desmos);
    expect(formatText(result, "wolfram")).toBe(result.wolfram);
  });
});
