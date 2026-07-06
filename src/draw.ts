import {
  clampKnotDrag,
  nearestKnot,
  nearestTangentHandle,
  nearPolyline,
  offsetCurve,
  slopeFromHandleDrag,
  tangentHandleEnd,
} from "./edit";
import type { FittedCurve, Knot } from "./fit";
import { History } from "./history";
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

type DragKind = "knot" | "tangent" | "translate";

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
  onCurveChange: (curve: FittedCurve | null) => void,
): {
  redraw: () => void;
  undo: () => void;
  redo: () => void;
  currentCurve: () => FittedCurve | null;
  showDerived: (polyline: readonly Point[]) => void;
  clearDerived: () => void;
  loadCurve: (loaded: FittedCurve) => void;
} {
  let curve: FittedCurve | null = null;
  // A derived (differentiated/integrated) curve to show read-only instead of the
  // drawing; null in normal edit mode. While set, its polyline replaces the
  // plane's curve and pointer editing/drawing is suspended.
  let derived: readonly Point[] | null = null;
  // The curve last announced via onCurveChange; a reference change means the
  // curve itself changed (fit/edit/translate/load/undo) rather than a mere
  // repaint (pan/zoom/theme), so the Points mirror refreshes only when needed.
  let notifiedCurve: FittedCurve | null = null;
  let notified = false;
  let active: StrokeBuilder | null = null;
  let dragKind: DragKind | null = null;
  let dragIndex: number | null = null;
  let dragKnots: Knot[] | null = null;
  // Whether the current drag actually moved, so a click that doesn't move adds
  // no history entry and skips a pointless re-fit.
  let dragMoved = false;
  // Translate state: the curve as grabbed and the world point where the drag
  // began. Offsetting is exact, so no re-fit is involved.
  let translateBase: FittedCurve | null = null;
  let translateStart: Point | null = null;
  // Undo/redo over committed curve states; `null` is the empty canvas.
  const history = new History<FittedCurve | null>(null);
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
    if (!notified || curve !== notifiedCurve) {
      notified = true;
      notifiedCurve = curve;
      onCurveChange(curve);
    }
    redrawBackground();
    const colors = colorsOf();
    if (derived) {
      // Derived (calculus) view: read-only, just the transformed curve — no knots
      // or handles, since it is not the editable drawing.
      drawPolyline(ctx, vp, derived, colors.curve);
      return;
    }
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
    dragMoved = false;
    canvas.setPointerCapture(event.pointerId);
    return true;
  };

  const moveEditDrag = (event: PointerEvent): boolean => {
    if (dragIndex === null || !dragKnots) {
      return false;
    }
    dragMoved = true;
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
    if (!dragMoved) {
      // A click that didn't move — nothing changed, so don't re-fit or record it.
      dragKind = null;
      dragIndex = null;
      dragKnots = null;
      return true;
    }
    applyRefit(
      dragKnots.map((knot) => ({ ...knot })),
      () => {
        dragKind = null;
        dragIndex = null;
        dragKnots = null;
        history.push(curve);
      },
    );
    return true;
  };

  // Grab the curve body (away from any knot/handle) to translate the whole curve.
  const beginTranslate = (event: PointerEvent): boolean => {
    if (!curve) {
      return false;
    }
    if (!nearPolyline(curve.polyline, vp, eventToScreen(event), GRAB_RADIUS)) {
      return false;
    }
    dragKind = "translate";
    translateBase = curve;
    translateStart = eventToWorld(event);
    dragMoved = false;
    canvas.setPointerCapture(event.pointerId);
    return true;
  };

  const moveTranslate = (event: PointerEvent): boolean => {
    if (dragKind !== "translate" || !translateBase || !translateStart) {
      return false;
    }
    dragMoved = true;
    const now = eventToWorld(event);
    curve = offsetCurve(
      translateBase,
      now.x - translateStart.x,
      now.y - translateStart.y,
    );
    redraw();
    return true;
  };

  const endTranslate = (event: PointerEvent): boolean => {
    if (dragKind !== "translate") {
      return false;
    }
    canvas.releasePointerCapture(event.pointerId);
    if (dragMoved) {
      history.push(curve);
    }
    dragKind = null;
    translateBase = null;
    translateStart = null;
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
        history.push(curve);
        redraw();
      })
      .catch(() => {
        // Core rejected the stroke (too few points / backward resume); keep the
        // previous curve unchanged.
      });
  };

  canvas.addEventListener("pointerdown", (event) => {
    // Only the primary button edits/draws; others drive pan/zoom (navigate.ts).
    // A derived (calculus) view is read-only.
    if (event.button !== 0 || derived) {
      return;
    }
    if (beginEditDrag(event) || beginTranslate(event)) {
      return;
    }
    beginStroke(event);
  });

  canvas.addEventListener("pointermove", (event) => {
    if (derived) {
      return;
    }
    if (moveEditDrag(event) || moveTranslate(event)) {
      return;
    }
    if (active && active.tryAdd(eventToWorld(event))) {
      redraw();
    }
  });

  canvas.addEventListener("pointerup", (event) => {
    if (derived) {
      return;
    }
    if (endEditDrag(event) || endTranslate(event)) {
      return;
    }
    endStroke(event);
  });

  // Restore a history state, abandoning any in-progress stroke or drag.
  const restore = (state: FittedCurve | null): void => {
    curve = state;
    active = null;
    dragKind = null;
    dragIndex = null;
    dragKnots = null;
    translateBase = null;
    translateStart = null;
    redraw();
  };

  return {
    redraw,
    undo: () => restore(history.undo()),
    redo: () => restore(history.redo()),
    currentCurve: () => curve,
    showDerived: (polyline: readonly Point[]) => {
      derived = polyline;
      redraw();
    },
    clearDerived: () => {
      derived = null;
      redraw();
    },
    // Load an opened file as the editable curve: a fresh committed state that
    // replaces any in-progress stroke, drag, or derived view.
    loadCurve: (loaded: FittedCurve) => {
      derived = null;
      restore(loaded);
      history.push(loaded);
    },
  };
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
