import type { Knot } from "./fit";
import type { Point } from "./viewport";

// Parse and format the text form of a curve's points (Phase: data-entry). A knot
// set and a `x, y`-per-line text block are two views of the same thing, so the
// Points panel and the canvas stay in sync. Kept pure and separate from the DOM
// so it is unit-tested.

/** A successful parse (points sorted by x) or a human-readable failure. */
export type PointsResult =
  { ok: true; points: Point[] } | { ok: false; error: string };

/** Parse a `x, y`-per-line block into points sorted by x. Fields may be split by
 * a comma and/or whitespace (so pasted spreadsheet rows work); blank lines are
 * ignored. Fails on a malformed line or on two points sharing an x — a function
 * has one y per x, which is why out-of-order input is sorted but duplicates are
 * not. */
export function parsePoints(text: string): PointsResult {
  const points: Point[] = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (trimmed === "") {
      continue;
    }
    const fields = trimmed.split(/[\s,]+/).filter((f) => f !== "");
    if (fields.length !== 2) {
      return {
        ok: false,
        error: `Line ${i + 1}: expected "x, y", got "${trimmed}"`,
      };
    }
    const x = Number(fields[0]);
    const y = Number(fields[1]);
    if (!Number.isFinite(x) || !Number.isFinite(y)) {
      return {
        ok: false,
        error: `Line ${i + 1}: "${trimmed}" is not a pair of numbers`,
      };
    }
    points.push({ x, y });
  }
  const sorted = [...points].sort((a, b) => a.x - b.x);
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i].x === sorted[i - 1].x) {
      return {
        ok: false,
        error: `Two points share x = ${formatNumber(sorted[i].x)} (a function has one y per x)`,
      };
    }
  }
  return { ok: true, points: sorted };
}

/** Render a curve's knots as `x, y` lines for the Points field. Tangents are not
 * shown — the text form is x/y data; slopes are a canvas-only refinement. */
export function formatPoints(knots: readonly Knot[]): string {
  return knots
    .map((k) => `${formatNumber(k.x)}, ${formatNumber(k.y)}`)
    .join("\n");
}

/** A coordinate for display: at most four decimals, trailing zeros trimmed,
 * negative zero normalized to `0`. */
export function formatNumber(value: number): string {
  const fixed = (Object.is(value, -0) ? 0 : value).toFixed(4);
  const trimmed = fixed.replace(/\.?0+$/, "");
  return trimmed === "" || trimmed === "-0" ? "0" : trimmed;
}
