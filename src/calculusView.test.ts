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
    expect(calcNote([])).toBe("");
  });

  it("warns that a derivative has corners at the knots", () => {
    expect(calcNote(["differentiate"])).toContain("corners");
  });

  it("notes that an integral is smooth", () => {
    expect(calcNote(["integrate"])).toContain("smooth");
  });

  it("reflects the last operation in a chain", () => {
    expect(calcNote(["integrate", "differentiate"])).toContain("corners");
    expect(calcNote(["differentiate", "integrate"])).toContain("smooth");
  });
});
