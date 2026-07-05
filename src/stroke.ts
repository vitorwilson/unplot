import type { Point } from "./viewport";

// Real-time hard-block, mirrored from the Rust core's `validate` module. This
// gives the pen an instant "wall"; the Rust core re-validates authoritatively
// when the finished stroke is submitted, so the two must stay in lockstep.

/** A candidate point may extend a stroke only if x strictly increases. */
export function advancesInX(prevX: number, nextX: number): boolean {
  return nextX > prevX;
}

/** The step from `prev` to `next` must advance in x and keep `|slope|` at or
 * below `maxAbsSlope` (blocks near-vertical spikes). */
export function withinSlopeCap(
  prev: Point,
  next: Point,
  maxAbsSlope: number,
): boolean {
  if (next.x <= prev.x) {
    return false;
  }
  return Math.abs((next.y - prev.y) / (next.x - prev.x)) <= maxAbsSlope;
}

/**
 * Accumulates one stroke's world-space samples, refusing any that fail the
 * hard-block — so the captured points are a valid function by construction.
 */
export class StrokeBuilder {
  private readonly points: Point[] = [];

  constructor(private readonly maxAbsSlope: number) {}

  /** Try to append `p`. The first point is always accepted; later ones only if
   * they clear the hard-block against the last accepted point. Returns whether
   * it was added. */
  tryAdd(p: Point): boolean {
    const last = this.points.at(-1);
    if (last && !withinSlopeCap(last, p, this.maxAbsSlope)) {
      return false;
    }
    this.points.push(p);
    return true;
  }

  samples(): readonly Point[] {
    return this.points;
  }

  get length(): number {
    return this.points.length;
  }
}
