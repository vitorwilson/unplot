import { invoke } from "@tauri-apps/api/core";
import type { Point } from "./viewport";

// Allowed perpendicular deviation (world units) when the core resamples a stroke.
const FIT_TOLERANCE = 0.05;

/** A knot in world coordinates: position plus an optional user-set tangent
 * (slope). `null` means the fitter chooses the slope; a number is a dragged
 * tangent handle. */
export interface Knot {
  x: number;
  y: number;
  tangent: number | null;
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
