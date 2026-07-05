/** A point in either world (function) or screen (CSS pixel) coordinates. */
export interface Point {
  x: number;
  y: number;
}

/**
 * Maps world coordinates (the function's x, y) to screen pixels and back.
 * `originX/originY` is the screen pixel of world (0, 0); `scale` is pixels per
 * world unit. Screen y grows downward, so the world y axis is flipped.
 */
export interface Viewport {
  originX: number;
  originY: number;
  scale: number;
}

/**
 * @example
 * worldToScreen({ originX: 320, originY: 240, scale: 40 }, { x: 0, y: 1 });
 * // { x: 320, y: 200 } — one world unit up is 40 px up the screen
 */
export function worldToScreen(vp: Viewport, world: Point): Point {
  return {
    x: vp.originX + world.x * vp.scale,
    y: vp.originY - world.y * vp.scale,
  };
}

export function screenToWorld(vp: Viewport, screen: Point): Point {
  return {
    x: (screen.x - vp.originX) / vp.scale,
    y: (vp.originY - screen.y) / vp.scale,
  };
}

/** Slide the plane by a screen-pixel delta (drag-to-pan). */
export function pan(
  vp: Viewport,
  dxScreen: number,
  dyScreen: number,
): Viewport {
  return {
    ...vp,
    originX: vp.originX + dxScreen,
    originY: vp.originY + dyScreen,
  };
}

/** Zoom by `factor` about a fixed screen point (scroll-to-zoom under cursor). */
export function zoomAt(vp: Viewport, pivot: Point, factor: number): Viewport {
  if (factor <= 0) {
    throw new Error(`zoomAt: factor must be positive, got ${factor}`);
  }
  const world = screenToWorld(vp, pivot);
  const scale = vp.scale * factor;
  return {
    scale,
    originX: pivot.x - world.x * scale,
    originY: pivot.y + world.y * scale,
  };
}
