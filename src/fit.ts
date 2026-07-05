import { invoke } from "@tauri-apps/api/core";
import type { Point } from "./viewport";

// Allowed perpendicular deviation (world units) when the core resamples a stroke.
const FIT_TOLERANCE = 0.05;

interface FittedCurve {
  knots: [number, number][];
  polyline: [number, number][];
}

/**
 * Send a raw stroke to the Rust core to resample and fit, returning the smooth
 * spline as a dense polyline in world coordinates. Rejects if the core refuses
 * the stroke (e.g. fewer than two distinct points).
 *
 * This is the frontend's only seam to the fitting core; the rest of the drawing
 * code stays unaware of Tauri.
 */
export async function fitStroke(samples: Point[]): Promise<Point[]> {
  const fitted = await invoke<FittedCurve>("fit_curve", {
    samples: samples.map((p) => [p.x, p.y]),
    tolerance: FIT_TOLERANCE,
  });
  return fitted.polyline.map(([x, y]) => ({ x, y }));
}
