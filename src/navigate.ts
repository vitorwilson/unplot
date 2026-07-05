import { pan, zoomAt, type Viewport } from "./viewport";

// Wheel-to-zoom (about the cursor) and middle-drag-to-pan. These mutate the
// shared viewport object in place — curves are stored in world coordinates, so
// navigating only changes how the same curve is projected — then repaint.

const ZOOM_PER_WHEEL_LINE = 1.0015;
const PAN_BUTTON = 1; // middle mouse

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
      const factor = ZOOM_PER_WHEEL_LINE ** -event.deltaY;
      apply(vp, zoomAt(vp, localPoint(event), factor));
      redraw();
    },
    { passive: false },
  );

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
    if (event.button !== PAN_BUTTON || !panning) {
      return;
    }
    panning = null;
    canvas.releasePointerCapture(event.pointerId);
  });
}
