import { StrokeBuilder } from "./stroke";
import {
  screenToWorld,
  worldToScreen,
  type Point,
  type Viewport,
} from "./viewport";

const STROKE_COLOR = "#1565c0";
const STROKE_WIDTH = 2;

/**
 * Install pointer-driven, hard-blocked freehand drawing on the canvas. The pen
 * physically cannot go backward in x or exceed the slope cap (the "wall").
 * `redrawBackground` repaints the plane; strokes are drawn over it.
 *
 * Fitting the captured samples into a smooth spline happens in the Rust core on
 * stroke end (a later increment); for now the raw hard-blocked polyline is shown.
 */
export function installDrawing(
  canvas: HTMLCanvasElement,
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  maxAbsSlope: number,
  redrawBackground: () => void,
  fitStroke: (samples: Point[]) => Promise<Point[]>,
): void {
  const committed: Point[][] = [];
  let active: StrokeBuilder | null = null;

  const eventToWorld = (event: PointerEvent): Point => {
    const rect = canvas.getBoundingClientRect();
    return screenToWorld(vp, {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top,
    });
  };

  const redraw = (): void => {
    redrawBackground();
    for (const stroke of committed) {
      drawPolyline(ctx, vp, stroke);
    }
    if (active) {
      drawPolyline(ctx, vp, active.samples());
    }
  };

  canvas.addEventListener("pointerdown", (event) => {
    active = new StrokeBuilder(maxAbsSlope);
    active.tryAdd(eventToWorld(event));
    canvas.setPointerCapture(event.pointerId);
    redraw();
  });

  canvas.addEventListener("pointermove", (event) => {
    if (active && active.tryAdd(eventToWorld(event))) {
      redraw();
    }
  });

  canvas.addEventListener("pointerup", (event) => {
    if (!active) {
      return;
    }
    const raw = [...active.samples()];
    active = null;
    canvas.releasePointerCapture(event.pointerId);
    redraw();
    void fitStroke(raw)
      .then((smooth) => {
        committed.push(smooth);
        redraw();
      })
      .catch(() => {
        // The core rejected the stroke (e.g. too few points); drop it.
      });
  });
}

function drawPolyline(
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  points: readonly Point[],
): void {
  if (points.length < 2) {
    return;
  }
  ctx.strokeStyle = STROKE_COLOR;
  ctx.lineWidth = STROKE_WIDTH;
  ctx.beginPath();
  const start = worldToScreen(vp, points[0]);
  ctx.moveTo(start.x, start.y);
  for (const point of points.slice(1)) {
    const screen = worldToScreen(vp, point);
    ctx.lineTo(screen.x, screen.y);
  }
  ctx.stroke();
}
