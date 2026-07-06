import { invoke } from "@tauri-apps/api/core";
import type { Point } from "./viewport";

// Allowed perpendicular deviation (world units) when the core resamples a stroke.
const FIT_TOLERANCE = 0.05;

/** A knot in world coordinates: position, an optional user-set tangent (`null`
 * = fitter chooses, a number = a dragged tangent handle), and the effective
 * `slope` in the fitted curve (used to draw the tangent handle). */
export interface Knot {
  x: number;
  y: number;
  tangent: number | null;
  slope: number;
}

/** A curve fitted by the core: its knots (for editing/resume) and a dense
 * polyline of the smooth spline (for rendering), in world coordinates. */
export interface FittedCurve {
  knots: Knot[];
  polyline: Point[];
}

interface RawCurve {
  knots: Knot[];
  polyline: [number, number][];
}

const toPoints = (pairs: [number, number][]): Point[] =>
  pairs.map(([x, y]) => ({ x, y }));

const toPairs = (points: Point[]): [number, number][] =>
  points.map((p) => [p.x, p.y]);

const shape = (raw: RawCurve): FittedCurve => ({
  knots: raw.knots,
  polyline: toPoints(raw.polyline),
});

/**
 * Fit a fresh stroke into a new curve via the Rust core. Rejects if the core
 * refuses the stroke (e.g. fewer than two distinct points).
 */
export async function fitStroke(samples: Point[]): Promise<FittedCurve> {
  return shape(
    await invoke<RawCurve>("fit_curve", {
      samples: toPairs(samples),
      tolerance: FIT_TOLERANCE,
    }),
  );
}

/**
 * Resume: append a stroke to an existing curve's knots, joining C¹, via the Rust
 * core. Rejects if the new stroke does not continue strictly to the right.
 */
export async function extendStroke(
  existing: Knot[],
  samples: Point[],
): Promise<FittedCurve> {
  return shape(
    await invoke<RawCurve>("extend_curve", {
      existing,
      samples: toPairs(samples),
      tolerance: FIT_TOLERANCE,
    }),
  );
}

/**
 * Re-fit an edited set of knots (dragged points or tangent handles) via the Rust
 * core. Rejects if the edit is not a valid function (e.g. a knot past a neighbor).
 */
export async function refitCurve(knots: Knot[]): Promise<FittedCurve> {
  return shape(await invoke<RawCurve>("refit_curve", { knots }));
}

/** A compact closed-form approximation of the curve (Phase 7), offered only when
 * trustworthy. `maxError`/`rmsError` are fractions of the curve's y-range. */
export interface Approximation {
  latex: string;
  maxError: number;
  rmsError: number;
}

/** The curve's exact function in every copy target: a one-line `summary`, the
 * full piecewise `latex` cases block (also shown by KaTeX), and the Desmos /
 * Wolfram paste forms. All are derived from one fit by the core. `approximation`
 * is a compact closed form when one is trustworthy, else `null`. */
export interface CurveLatex {
  summary: string;
  latex: string;
  desmos: string;
  wolfram: string;
  approximation: Approximation | null;
}

/** Ask the core for the current curve's LaTeX (the "Done" action). */
export async function curveLatex(knots: Knot[]): Promise<CurveLatex> {
  return invoke<CurveLatex>("curve_latex", { knots });
}

/** A calculus operation to chain onto the drawn curve. */
export type CalcOp = "differentiate" | "integrate";

/** A calculus result for display: the transformed curve's polyline (to draw), its
 * math in every copy format, and a closed-form approximation of the derivative /
 * integral itself when one is trustworthy. Not editable — the drawn knots stay
 * the source of truth and the operation stack is replayed on each request. */
export interface CalcCurve {
  polyline: Point[];
  summary: string;
  latex: string;
  desmos: string;
  wolfram: string;
  approximation: Approximation | null;
}

interface RawCalcCurve {
  polyline: [number, number][];
  summary: string;
  latex: string;
  desmos: string;
  wolfram: string;
  approximation: Approximation | null;
}

/** Fit the drawn `knots`, apply each `op` in order through the core, and return
 * the resulting curve (polyline + math). Rejects if the core refuses the knots. */
export async function applyCalculus(
  knots: Knot[],
  ops: CalcOp[],
): Promise<CalcCurve> {
  const raw = await invoke<RawCalcCurve>("apply_calculus", { knots, ops });
  return { ...raw, polyline: toPoints(raw.polyline) };
}

/** Write the drawn curve to `path` as a versioned `.unplot` document. */
export async function saveCurve(path: string, knots: Knot[]): Promise<void> {
  await invoke("save_curve", { path, knots });
}

/** Read a `.unplot` document from `path` into an editable fitted curve. Rejects
 * if the file is malformed, from a newer schema, or not a valid function. */
export async function openCurve(path: string): Promise<FittedCurve> {
  return shape(await invoke<RawCurve>("open_curve", { path }));
}
