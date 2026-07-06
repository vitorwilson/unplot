import { describe, expect, it } from "vitest";
import { formatNumber, formatPoints, parsePoints } from "./points";
import type { Knot } from "./fit";

const knot = (x: number, y: number, tangent: number | null = null): Knot => ({
  x,
  y,
  tangent,
  slope: 0,
});

describe("parsePoints", () => {
  it("parses comma-separated points", () => {
    const result = parsePoints("0, 0\n1, 2\n2, 1");
    expect(result).toEqual({
      ok: true,
      points: [
        { x: 0, y: 0 },
        { x: 1, y: 2 },
        { x: 2, y: 1 },
      ],
    });
  });

  it("accepts whitespace or tab separators and ignores blank lines", () => {
    const result = parsePoints("  0 0 \n\n1\t2\n");
    expect(result).toEqual({
      ok: true,
      points: [
        { x: 0, y: 0 },
        { x: 1, y: 2 },
      ],
    });
  });

  it("sorts out-of-order points by x", () => {
    const result = parsePoints("2, 1\n0, 0\n1, 2");
    expect(result.ok && result.points.map((p) => p.x)).toEqual([0, 1, 2]);
  });

  it("rejects a line without exactly two fields", () => {
    const result = parsePoints("0, 0\n1 2 3");
    expect(result.ok).toBe(false);
    expect(!result.ok && result.error).toContain("Line 2");
  });

  it("rejects a non-numeric field", () => {
    const result = parsePoints("0, 0\nx, 2");
    expect(result.ok).toBe(false);
    expect(!result.ok && result.error).toContain("Line 2");
  });

  it("rejects two points that share an x (not a function)", () => {
    const result = parsePoints("1, 2\n1, 5");
    expect(result.ok).toBe(false);
    expect(!result.ok && result.error).toContain("x = 1");
  });

  it("allows negative and decimal coordinates", () => {
    const result = parsePoints("-2.5, -1\n0, 0.25");
    expect(result).toEqual({
      ok: true,
      points: [
        { x: -2.5, y: -1 },
        { x: 0, y: 0.25 },
      ],
    });
  });

  it("returns an empty list for empty text", () => {
    expect(parsePoints("   \n  ")).toEqual({ ok: true, points: [] });
  });
});

describe("formatPoints", () => {
  it("renders knots as x, y lines without tangents", () => {
    expect(formatPoints([knot(0, 0), knot(1, 2, 0.5)])).toBe("0, 0\n1, 2");
  });

  it("round-trips with parsePoints", () => {
    const text = "0, 0\n1, 2\n2.5, -1";
    const parsed = parsePoints(text);
    expect(parsed.ok).toBe(true);
    if (parsed.ok) {
      const knots = parsed.points.map((p) => knot(p.x, p.y));
      expect(formatPoints(knots)).toBe(text);
    }
  });
});

describe("formatNumber", () => {
  it("trims trailing zeros and normalizes negative zero", () => {
    expect(formatNumber(2)).toBe("2");
    expect(formatNumber(-3.5)).toBe("-3.5");
    expect(formatNumber(0.25)).toBe("0.25");
    expect(formatNumber(-0)).toBe("0");
    expect(formatNumber(120)).toBe("120");
  });
});
