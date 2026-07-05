import { screenToWorld, type Viewport } from "./viewport";

/** World coordinates of the gridlines currently on screen. */
export interface GridLines {
  xs: number[];
  ys: number[];
}

/**
 * World coordinates of every gridline (a multiple of `step`) visible in a
 * `width`×`height` CSS-pixel canvas under `vp`. Used to draw the Cartesian grid
 * without walking off-screen coordinates.
 */
export function visibleGridLines(
  vp: Viewport,
  width: number,
  height: number,
  step: number,
): GridLines {
  if (step <= 0) {
    throw new Error(`visibleGridLines: step must be positive, got ${step}`);
  }
  // Screen (0,0) is the top-left: minimum world x, maximum world y.
  const topLeft = screenToWorld(vp, { x: 0, y: 0 });
  const bottomRight = screenToWorld(vp, { x: width, y: height });
  return {
    xs: multiplesInRange(topLeft.x, bottomRight.x, step),
    ys: multiplesInRange(bottomRight.y, topLeft.y, step),
  };
}

function multiplesInRange(min: number, max: number, step: number): number[] {
  const out: number[] = [];
  for (let k = Math.ceil(min / step); k <= Math.floor(max / step); k++) {
    out.push(k * step);
  }
  return out;
}
