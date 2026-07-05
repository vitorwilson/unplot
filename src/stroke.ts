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

  /** `anchor` is the endpoint of an existing curve when resuming: the first
   * point must then clear the hard-block against it, so a resumed stroke cannot
   * begin at or behind where the previous one ended. */
  constructor(
    private readonly maxAbsSlope: number,
    private readonly anchor: Point | null = null,
  ) {}

  /** Try to append `p`. The first point is accepted unconditionally when drawing
   * fresh, or gated against the anchor when resuming; later points are gated
   * against the last accepted point. Returns whether it was added. */
  tryAdd(p: Point): boolean {
    const last = this.points.at(-1) ?? this.anchor;
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
