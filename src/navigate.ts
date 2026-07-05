import { pan, zoomAt, type Viewport } from "./viewport";

// Trackpad-first navigation: a two-finger scroll pans the plane; a pinch (or
// Ctrl + two-finger scroll, a reliable fallback on WebKitGTK) zooms about the
// cursor. Both arrive as `wheel` events — pinch sets `ctrlKey`. Right-drag pans
// too, for mice. These mutate the shared viewport in place (curves are stored in
// world coordinates, so navigating only reprojects them) and repaint.

const ZOOM_PER_WHEEL_LINE = 1.0015;
const LINE_HEIGHT_PX = 16; // deltaMode === 1 (lines) → approximate pixels
const PAN_BUTTON = 2; // right mouse

/** What a wheel event should do. `dx`/`dy` are screen-pixel pan amounts; `factor`
 * is the zoom multiplier. */
export interface WheelAction {
  kind: "pan" | "zoom";
  dx: number;
  dy: number;
  factor: number;
}

/**
 * Decide whether a wheel event pans or zooms: a pinch or Ctrl+scroll zooms; a
 * plain two-finger scroll (or mouse wheel) pans. Pure, so it is unit-tested.
 *
 * @example
 * interpretWheel(false, 0, 10, 0); // { kind: "pan", dx: 0, dy: -10, ... }
 */
export function interpretWheel(
  ctrlKey: boolean,
  deltaX: number,
  deltaY: number,
  deltaMode: number,
): WheelAction {
  const unit = deltaMode === 1 ? LINE_HEIGHT_PX : 1;
  if (ctrlKey) {
    return {
      kind: "zoom",
      dx: 0,
      dy: 0,
      factor: ZOOM_PER_WHEEL_LINE ** (-deltaY * unit),
    };
  }
  // Pan opposite the scroll so the plane follows the fingers.
  return { kind: "pan", dx: -deltaX * unit, dy: -deltaY * unit, factor: 1 };
}

/** Copy `next`'s fields onto the shared `vp` so every closure holding `vp` sees
 * the change without needing a new reference. */
function apply(vp: Viewport, next: Viewport): void {
  vp.originX = next.originX;
  vp.originY = next.originY;
  vp.scale = next.scale;
}

export function installViewportControls(
  canvas: HTMLCanvasElement,
  vp: Viewport,
  redraw: () => void,
): void {
  const localPoint = (event: { clientX: number; clientY: number }) => {
    const rect = canvas.getBoundingClientRect();
    return { x: event.clientX - rect.left, y: event.clientY - rect.top };
  };

  canvas.addEventListener(
    "wheel",
    (event) => {
      event.preventDefault();
      const action = interpretWheel(
        event.ctrlKey,
        event.deltaX,
        event.deltaY,
        event.deltaMode,
      );
      if (action.kind === "zoom") {
        apply(vp, zoomAt(vp, localPoint(event), action.factor));
      } else {
        apply(vp, pan(vp, action.dx, action.dy));
      }
      redraw();
    },
    { passive: false },
  );

  // Suppress the browser context menu so right-drag can pan uninterrupted.
  canvas.addEventListener("contextmenu", (event) => event.preventDefault());

  let panning: { x: number; y: number } | null = null;

  canvas.addEventListener("pointerdown", (event) => {
    if (event.button !== PAN_BUTTON) {
      return;
    }
    event.preventDefault();
    panning = localPoint(event);
    canvas.setPointerCapture(event.pointerId);
  });

  canvas.addEventListener("pointermove", (event) => {
    if (!panning) {
      return;
    }
    const here = localPoint(event);
    apply(vp, pan(vp, here.x - panning.x, here.y - panning.y));
    panning = here;
    redraw();
  });

  canvas.addEventListener("pointerup", (event) => {
    if (!panning) {
      return;
    }
    panning = null;
    canvas.releasePointerCapture(event.pointerId);
  });
}
