import { invoke } from "@tauri-apps/api/core";
import type { Point } from "./viewport";

// Allowed perpendicular deviation (world units) when the core resamples a stroke.
const FIT_TOLERANCE = 0.05;

/** A curve fitted by the core: its knots (for a later resume) and a dense
 * polyline of the smooth spline (for rendering), in world coordinates. */
export interface FittedCurve {
  knots: Point[];
  polyline: Point[];
}

interface RawCurve {
  knots: [number, number][];
  polyline: [number, number][];
}

const toPoints = (pairs: [number, number][]): Point[] =>
  pairs.map(([x, y]) => ({ x, y }));

const toPairs = (points: Point[]): [number, number][] =>
  points.map((p) => [p.x, p.y]);

/**
 * Fit a fresh stroke into a new curve via the Rust core. Rejects if the core
 * refuses the stroke (e.g. fewer than two distinct points).
 */
export async function fitStroke(samples: Point[]): Promise<FittedCurve> {
  const raw = await invoke<RawCurve>("fit_curve", {
    samples: toPairs(samples),
    tolerance: FIT_TOLERANCE,
  });
  return { knots: toPoints(raw.knots), polyline: toPoints(raw.polyline) };
}

/**
 * Resume: append a stroke to an existing curve's knots, joining C¹, via the Rust
 * core. Rejects if the new stroke does not continue strictly to the right.
 */
export async function extendStroke(
  existing: Point[],
  samples: Point[],
): Promise<FittedCurve> {
  const raw = await invoke<RawCurve>("extend_curve", {
    existing: toPairs(existing),
    samples: toPairs(samples),
    tolerance: FIT_TOLERANCE,
  });
  return { knots: toPoints(raw.knots), polyline: toPoints(raw.polyline) };
}
