import type { FittedCurve } from "./fit";
import { StrokeBuilder } from "./stroke";
import {
  screenToWorld,
  worldToScreen,
  type Point,
  type Viewport,
} from "./viewport";

const STROKE_COLOR = "#1565c0";
const STROKE_WIDTH = 2;

/** How the surrounding app fits a drawn stroke through the Rust core: a fresh
 * curve, or a C¹ resume of the existing one. */
export interface StrokeFitter {
  fit(samples: Point[]): Promise<FittedCurve>;
  extend(existing: Point[], samples: Point[]): Promise<FittedCurve>;
}

/**
 * Install pointer-driven, hard-blocked freehand drawing on the canvas. The pen
 * physically cannot go backward in x or exceed the slope cap (the "wall").
 *
 * The whole drawing is one function, built left to right: the first stroke fits
 * a new curve; each later stroke resumes it (lift the pen, pan, keep drawing),
 * joining C¹ through the core. `redrawBackground` repaints the plane underneath.
 */
export function installDrawing(
  canvas: HTMLCanvasElement,
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  maxAbsSlope: number,
  redrawBackground: () => void,
  fitter: StrokeFitter,
): void {
  let curve: FittedCurve | null = null;
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
    if (curve) {
      drawPolyline(ctx, vp, curve.polyline);
    }
    if (active) {
      drawPolyline(ctx, vp, active.samples());
    }
  };

  canvas.addEventListener("pointerdown", (event) => {
    // Resume from the previous curve's right endpoint, so the pen can't restart
    // behind where it left off.
    const anchor = curve ? (curve.knots.at(-1) ?? null) : null;
    active = new StrokeBuilder(maxAbsSlope, anchor);
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
    const existing = curve;
    active = null;
    canvas.releasePointerCapture(event.pointerId);
    redraw();

    const fitted = existing
      ? fitter.extend(existing.knots, raw)
      : fitter.fit(raw);
    void fitted
      .then((result) => {
        curve = result;
        redraw();
      })
      .catch(() => {
        // The core rejected the stroke (too few points, or a backward resume);
        // keep the previous curve unchanged.
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
