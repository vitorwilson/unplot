import {
  clampKnotDrag,
  nearestKnot,
  nearestTangentHandle,
  slopeFromHandleDrag,
  tangentHandleEnd,
} from "./edit";
import type { FittedCurve, Knot } from "./fit";
import { StrokeBuilder } from "./stroke";
import type { CanvasColors } from "./theme";
import {
  screenToWorld,
  worldToScreen,
  type Point,
  type Viewport,
} from "./viewport";

const STROKE_WIDTH = 2;
const KNOT_RADIUS = 4;
const GRAB_RADIUS = 10; // px within which a click grabs a knot to drag it
const HANDLE_LEN = 36; // px length of a tangent handle
const HANDLE_END_RADIUS = 3; // px radius of the draggable handle tip

type DragKind = "knot" | "tangent";

/** How the surrounding app fits strokes through the Rust core: a fresh curve, a
 * C¹ resume of the existing one, or a re-fit of edited knots. */
export interface StrokeFitter {
  fit(samples: Point[]): Promise<FittedCurve>;
  extend(existing: Knot[], samples: Point[]): Promise<FittedCurve>;
  refit(knots: Knot[]): Promise<FittedCurve>;
}

/**
 * Install pointer-driven drawing and knot editing on the canvas. Left-drag over
 * empty space draws (hard-blocked: no backward x, no spike); left-drag on a knot
 * dot drags that point, re-fitting through the core and staying a valid function.
 *
 * The whole drawing is one function, built left to right: the first stroke fits
 * a new curve; each later stroke resumes it (C¹). `redrawBackground` repaints the
 * plane underneath.
 */
export function installDrawing(
  canvas: HTMLCanvasElement,
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  maxAbsSlope: number,
  redrawBackground: () => void,
  fitter: StrokeFitter,
  colorsOf: () => CanvasColors,
): { redraw: () => void } {
  let curve: FittedCurve | null = null;
  let active: StrokeBuilder | null = null;
  let dragKind: DragKind | null = null;
  let dragIndex: number | null = null;
  let dragKnots: Knot[] | null = null;
  // Monotonic token so out-of-order refit responses during a fast drag are
  // ignored — only the latest applies.
  let refitToken = 0;

  const eventToScreen = (event: PointerEvent): Point => {
    const rect = canvas.getBoundingClientRect();
    return { x: event.clientX - rect.left, y: event.clientY - rect.top };
  };
  const eventToWorld = (event: PointerEvent): Point =>
    screenToWorld(vp, eventToScreen(event));

  const redraw = (): void => {
    redrawBackground();
    const colors = colorsOf();
    if (curve) {
      drawPolyline(ctx, vp, curve.polyline, colors.curve);
      const knots = dragKnots ?? curve.knots;
      drawTangentHandles(ctx, vp, knots, colors.handle);
      drawKnots(ctx, vp, knots, colors.handle);
    }
    if (active) {
      drawPolyline(ctx, vp, active.samples(), colors.curve);
    }
  };

  const applyRefit = (knots: Knot[], onDone?: () => void): void => {
    const token = ++refitToken;
    const finish = (result: FittedCurve | null): void => {
      if (token !== refitToken) {
        return; // a newer drag frame superseded this one
      }
      if (result) {
        curve = result;
      }
      onDone?.();
      redraw();
    };
    void fitter
      .refit(knots)
      .then(finish)
      .catch(() => finish(null));
  };

  // A tangent handle (if grabbed) takes precedence over its knot, which sits at
  // the handle's base.
  const beginEditDrag = (event: PointerEvent): boolean => {
    if (!curve) {
      return false;
    }
    const screen = eventToScreen(event);
    const onHandle = nearestTangentHandle(
      curve.knots,
      vp,
      screen,
      HANDLE_LEN,
      GRAB_RADIUS,
    );
    const hit = onHandle ?? nearestKnot(curve.knots, vp, screen, GRAB_RADIUS);
    if (hit === null) {
      return false;
    }
    dragKind = onHandle !== null ? "tangent" : "knot";
    dragIndex = hit;
    dragKnots = curve.knots.map((knot) => ({ ...knot }));
    canvas.setPointerCapture(event.pointerId);
    return true;
  };

  const moveEditDrag = (event: PointerEvent): boolean => {
    if (dragIndex === null || !dragKnots) {
      return false;
    }
    if (dragKind === "tangent") {
      const knotScreen = worldToScreen(vp, dragKnots[dragIndex]);
      const slope = slopeFromHandleDrag(
        knotScreen,
        eventToScreen(event),
        maxAbsSlope,
      );
      dragKnots[dragIndex] = { ...dragKnots[dragIndex], tangent: slope, slope };
    } else {
      const at = clampKnotDrag(
        dragKnots,
        dragIndex,
        eventToWorld(event),
        maxAbsSlope,
      );
      dragKnots[dragIndex] = { ...dragKnots[dragIndex], x: at.x, y: at.y };
    }
    redraw();
    applyRefit(dragKnots.map((knot) => ({ ...knot })));
    return true;
  };

  const endEditDrag = (event: PointerEvent): boolean => {
    if (dragIndex === null || !dragKnots) {
      return false;
    }
    canvas.releasePointerCapture(event.pointerId);
    applyRefit(
      dragKnots.map((knot) => ({ ...knot })),
      () => {
        dragKind = null;
        dragIndex = null;
        dragKnots = null;
      },
    );
    return true;
  };

  const beginStroke = (event: PointerEvent): void => {
    // Resume from the previous curve's right endpoint, so the pen can't restart
    // behind where it left off.
    const anchor = curve ? (curve.knots.at(-1) ?? null) : null;
    active = new StrokeBuilder(maxAbsSlope, anchor);
    active.tryAdd(eventToWorld(event));
    canvas.setPointerCapture(event.pointerId);
    redraw();
  };

  const endStroke = (event: PointerEvent): void => {
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
        // Core rejected the stroke (too few points / backward resume); keep the
        // previous curve unchanged.
      });
  };

  canvas.addEventListener("pointerdown", (event) => {
    // Only the primary button edits/draws; others drive pan/zoom (navigate.ts).
    if (event.button !== 0) {
      return;
    }
    if (beginEditDrag(event)) {
      return;
    }
    beginStroke(event);
  });

  canvas.addEventListener("pointermove", (event) => {
    if (moveEditDrag(event)) {
      return;
    }
    if (active && active.tryAdd(eventToWorld(event))) {
      redraw();
    }
  });

  canvas.addEventListener("pointerup", (event) => {
    if (endEditDrag(event)) {
      return;
    }
    endStroke(event);
  });

  return { redraw };
}

function drawPolyline(
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  points: readonly Point[],
  color: string,
): void {
  if (points.length < 2) {
    return;
  }
  ctx.strokeStyle = color;
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

/** Draw a small filled dot at each knot — the grab targets for editing. */
function drawKnots(
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  knots: readonly Knot[],
  color: string,
): void {
  ctx.fillStyle = color;
  for (const knot of knots) {
    const screen = worldToScreen(vp, knot);
    ctx.beginPath();
    ctx.arc(screen.x, screen.y, KNOT_RADIUS, 0, 2 * Math.PI);
    ctx.fill();
  }
}

/** Draw each knot's tangent handle: a thin line at the slope angle ending in a
 * hollow tip that the user drags to set the slope. */
function drawTangentHandles(
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  knots: readonly Knot[],
  color: string,
): void {
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  for (const knot of knots) {
    const base = worldToScreen(vp, knot);
    const end = tangentHandleEnd(knot, vp, HANDLE_LEN);
    ctx.beginPath();
    ctx.moveTo(base.x, base.y);
    ctx.lineTo(end.x, end.y);
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(end.x, end.y, HANDLE_END_RADIUS, 0, 2 * Math.PI);
    ctx.stroke();
  }
}
