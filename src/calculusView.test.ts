import { describe, expect, it } from "vitest";
import { calcNote, calcTitle } from "./calculusView";

describe("calcTitle", () => {
  it("is just f with no operations applied", () => {
    expect(calcTitle([])).toBe("f");
  });

  it("shows the operation chain as a breadcrumb", () => {
    expect(calcTitle(["differentiate"])).toBe("f → d/dx");
    expect(calcTitle(["differentiate", "integrate"])).toBe("f → d/dx → ∫");
  });
});

describe("calcNote", () => {
  it("is empty when nothing has been applied", () => {
    expect(calcNote([], false)).toBe("");
    expect(calcNote([], true)).toBe("");
  });

  it("warns that a numeric derivative has corners at the knots", () => {
    expect(calcNote(["differentiate"], false)).toContain("corners");
  });

  it("notes that a numeric integral is smooth", () => {
    expect(calcNote(["integrate"], false)).toContain("smooth");
  });

  it("reflects the last operation in a numeric chain", () => {
    expect(calcNote(["integrate", "differentiate"], false)).toContain(
      "corners",
    );
    expect(calcNote(["differentiate", "integrate"], false)).toContain("smooth");
  });

  it("calls an exact symbolic result a clean closed form, with no corners", () => {
    const note = calcNote(["differentiate"], true);
    expect(note).toContain("exact closed form");
    expect(note).not.toContain("corners");
  });
});
