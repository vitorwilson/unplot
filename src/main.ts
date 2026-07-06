import "katex/dist/katex.min.css";
// styles.css is loaded render-blocking via a <link> in index.html (avoids a
// flash of unstyled layout on reload), so it is not imported here.
import { canvasPixelSize } from "./dpr";
import { installDrawing } from "./draw";
import {
  applyCalculus,
  curveLatex,
  extendStroke,
  fitStroke,
  refitCurve,
  type FittedCurve,
} from "./fit";
import { installCalculusView, type CalculusController } from "./calculusView";
import { openCurveDialog, saveCurveDialog } from "./files";
import { tickStep, visibleGridLines } from "./grid";
import { installLatexView, type LatexView } from "./latexView";
import { installViewportControls } from "./navigate";
import { installPointsView, type PointsController } from "./pointsView";
import { installTheme, type CanvasColors } from "./theme";
import { worldToScreen, type Point, type Viewport } from "./viewport";

// Phase 2: a Cartesian plane on Canvas 2D — grid, axes, labels, wheel-zoom and
// right-drag-pan — with hard-block drawing and lift-and-resume on top.

const CSS_WIDTH = 860;
const CSS_HEIGHT = 600;
const TARGET_GRID_PX = 64; // aim for ~this many pixels between gridlines
const LABEL_FONT = "11px sans-serif";
// Spike cap for the drawing hard-block, in world units of |dy/dx|.
const MAX_SLOPE = 50;

function centeredViewport(): Viewport {
  return { originX: CSS_WIDTH / 2, originY: CSS_HEIGHT / 2, scale: 40 };
}

function setupCanvas(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const dpr = window.devicePixelRatio || 1;
  const { width, height } = canvasPixelSize(CSS_WIDTH, CSS_HEIGHT, dpr);
  canvas.width = width;
  canvas.height = height;
  canvas.style.width = `${CSS_WIDTH}px`;
  canvas.style.height = `${CSS_HEIGHT}px`;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("setupCanvas: 2D canvas context unavailable");
  }
  ctx.scale(dpr, dpr);
  return ctx;
}

function drawGrid(
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  step: number,
  color: string,
): void {
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  const { xs, ys } = visibleGridLines(vp, CSS_WIDTH, CSS_HEIGHT, step);
  for (const wx of xs) {
    const sx = worldToScreen(vp, { x: wx, y: 0 }).x;
    ctx.beginPath();
    ctx.moveTo(sx, 0);
    ctx.lineTo(sx, CSS_HEIGHT);
    ctx.stroke();
  }
  for (const wy of ys) {
    const sy = worldToScreen(vp, { x: 0, y: wy }).y;
    ctx.beginPath();
    ctx.moveTo(0, sy);
    ctx.lineTo(CSS_WIDTH, sy);
    ctx.stroke();
  }
}

function drawAxes(
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  color: string,
): void {
  const origin = worldToScreen(vp, { x: 0, y: 0 });
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(0, origin.y);
  ctx.lineTo(CSS_WIDTH, origin.y);
  ctx.moveTo(origin.x, 0);
  ctx.lineTo(origin.x, CSS_HEIGHT);
  ctx.stroke();
}

/** Numeric labels along the axes at each gridline (skipping the origin). */
function drawLabels(
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  step: number,
  color: string,
): void {
  const { xs, ys } = visibleGridLines(vp, CSS_WIDTH, CSS_HEIGHT, step);
  const origin = worldToScreen(vp, { x: 0, y: 0 });
  const decimals = Math.max(0, -Math.floor(Math.log10(step)));
  const label = (v: number) => v.toFixed(decimals);
  const clamp = (v: number, lo: number, hi: number) =>
    Math.min(Math.max(v, lo), hi);

  ctx.fillStyle = color;
  ctx.font = LABEL_FONT;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  const axisY = clamp(origin.y + 3, 3, CSS_HEIGHT - 14);
  for (const wx of xs) {
    if (wx !== 0) {
      ctx.fillText(label(wx), worldToScreen(vp, { x: wx, y: 0 }).x, axisY);
    }
  }

  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  const axisX = clamp(origin.x + 4, 4, CSS_WIDTH - 28);
  for (const wy of ys) {
    if (wy !== 0) {
      ctx.fillText(label(wy), axisX, worldToScreen(vp, { x: 0, y: wy }).y);
    }
  }
}

function drawPlane(
  ctx: CanvasRenderingContext2D,
  vp: Viewport,
  colors: CanvasColors,
): void {
  const step = tickStep(vp.scale, TARGET_GRID_PX);
  ctx.clearRect(0, 0, CSS_WIDTH, CSS_HEIGHT);
  drawGrid(ctx, vp, step, colors.grid);
  drawAxes(ctx, vp, colors.axis);
  drawLabels(ctx, vp, step, colors.label);
}

/** Wire the math panel: "Done" renders the drawn curve's LaTeX; d/dx and ∫
 * replace it with the derivative/integral (chainable) and Reset returns to the
 * drawing — all sharing one panel view. Returns the calculus controller (or
 * `null` if the DOM is missing) so the caller can suspend undo/redo while a
 * derived curve is shown. */
function installMathControls(
  currentCurve: () => FittedCurve | null,
  showDerived: (polyline: Point[]) => void,
  clearDerived: () => void,
): { view: LatexView; calc: CalculusController } | null {
  const doneBtn = document.querySelector<HTMLButtonElement>("#done-btn");
  const dxBtn = document.querySelector<HTMLButtonElement>("#dx-btn");
  const integralBtn =
    document.querySelector<HTMLButtonElement>("#integral-btn");
  const resetBtn = document.querySelector<HTMLButtonElement>("#calc-reset-btn");
  const panel = document.querySelector<HTMLElement>("#latex-panel");
  const summaryButton =
    document.querySelector<HTMLButtonElement>("#latex-summary");
  const formatSelect =
    document.querySelector<HTMLSelectElement>("#latex-format");
  const copyButton = document.querySelector<HTMLButtonElement>("#latex-copy");
  const body = document.querySelector<HTMLElement>("#latex-body");
  const math = document.querySelector<HTMLElement>("#latex-math");
  const approxPanel = document.querySelector<HTMLElement>("#latex-approx");
  const approxMath = document.querySelector<HTMLElement>("#latex-approx-math");
  const approxError = document.querySelector<HTMLElement>(
    "#latex-approx-error",
  );
  const approxCopy =
    document.querySelector<HTMLButtonElement>("#latex-approx-copy");
  if (
    !doneBtn ||
    !dxBtn ||
    !integralBtn ||
    !resetBtn ||
    !panel ||
    !summaryButton ||
    !formatSelect ||
    !copyButton ||
    !body ||
    !math ||
    !approxPanel ||
    !approxMath ||
    !approxError ||
    !approxCopy
  ) {
    return null;
  }
  const view = installLatexView({
    panel,
    summaryButton,
    formatSelect,
    copyButton,
    body,
    math,
    approxPanel,
    approxMath,
    approxError,
    approxCopy,
  });
  doneBtn.addEventListener("click", () => {
    const curve = currentCurve();
    if (!curve || curve.knots.length < 2) {
      view.message("Draw a function first.");
      return;
    }
    void curveLatex(curve.knots)
      .then((result) => view.show(result))
      .catch(() => view.message("Couldn't render the function."));
  });
  const calc = installCalculusView(
    {
      dxButton: dxBtn,
      integralButton: integralBtn,
      resetButton: resetBtn,
      doneButton: doneBtn,
    },
    {
      currentKnots: () => currentCurve()?.knots ?? null,
      applyCalculus,
      showDerived,
      clearDerived,
      view,
    },
  );
  return { view, calc };
}

/** Wire the Save and Open buttons. Save writes the drawn curve as a `.unplot`
 * file; Open loads one as the editable curve, first clearing any derived
 * (calculus) view. Errors surface in the shared panel. */
function installFileControls(
  view: LatexView,
  currentCurve: () => FittedCurve | null,
  loadCurve: (curve: FittedCurve) => void,
  calc: CalculusController,
): void {
  const saveBtn = document.querySelector<HTMLButtonElement>("#save-btn");
  const openBtn = document.querySelector<HTMLButtonElement>("#open-btn");
  if (!saveBtn || !openBtn) {
    return;
  }
  saveBtn.addEventListener("click", () => {
    const curve = currentCurve();
    if (!curve || curve.knots.length < 2) {
      view.message("Draw a function first.");
      return;
    }
    void saveCurveDialog(curve.knots).catch(() =>
      view.message("Couldn't save the file."),
    );
  });
  openBtn.addEventListener("click", () => {
    void openCurveDialog()
      .then((loaded) => {
        if (loaded) {
          calc.reset();
          loadCurve(loaded);
        }
      })
      .catch(() => view.message("Couldn't open that file."));
  });
}

/** Wire the Points panel: typing `x, y` per line and pressing Plot rebuilds the
 * curve from those points. Returns the controller (or `null` if the DOM is
 * missing) so the caller can mirror curve changes back into the field. */
function installPointsControls(
  loadCurve: (curve: FittedCurve) => void,
  resetDerived: () => void,
): PointsController | null {
  const toggleButton =
    document.querySelector<HTMLButtonElement>("#points-toggle");
  const body = document.querySelector<HTMLElement>("#points-body");
  const textarea = document.querySelector<HTMLTextAreaElement>("#points-input");
  const plotButton = document.querySelector<HTMLButtonElement>("#points-plot");
  const message = document.querySelector<HTMLElement>("#points-message");
  if (!toggleButton || !body || !textarea || !plotButton || !message) {
    return null;
  }
  return installPointsView(
    { toggleButton, body, textarea, plotButton, message },
    { refit: refitCurve, loadCurve, resetDerived },
  );
}

const canvas = document.querySelector<HTMLCanvasElement>("#plane");
if (canvas) {
  const ctx = setupCanvas(canvas);
  const viewport = centeredViewport();

  const toggle = document.querySelector<HTMLButtonElement>("#theme-toggle");
  // `repaint` is filled in once installDrawing returns its redraw; the theme's
  // onChange closes over it so toggling recolors the canvas too.
  let repaint = () => {};
  const theme = installTheme((next) => {
    if (toggle) {
      toggle.textContent = next === "dark" ? "☀ Light" : "☾ Dark";
    }
    repaint();
  });
  if (toggle) {
    toggle.textContent = theme.current() === "dark" ? "☀ Light" : "☾ Dark";
    toggle.addEventListener("click", () => theme.toggle());
  }

  const redrawBackground = () => drawPlane(ctx, viewport, theme.colors());
  redrawBackground();
  // Late-bound so installDrawing can announce curve changes to the Points panel,
  // which is created afterward (it needs loadCurve from installDrawing).
  let syncPoints: (curve: FittedCurve | null) => void = () => {};
  const {
    redraw,
    undo,
    redo,
    currentCurve,
    showDerived,
    clearDerived,
    loadCurve,
  } = installDrawing(
    canvas,
    ctx,
    viewport,
    MAX_SLOPE,
    redrawBackground,
    { fit: fitStroke, extend: extendStroke, refit: refitCurve },
    () => theme.colors(),
    (curve) => syncPoints(curve),
  );
  repaint = redraw;
  installViewportControls(canvas, viewport, redraw);
  const math = installMathControls(currentCurve, showDerived, clearDerived);
  if (math) {
    installFileControls(math.view, currentCurve, loadCurve, math.calc);
  }
  const points = installPointsControls(loadCurve, () => math?.calc.reset());
  if (points) {
    syncPoints = points.syncFromCurve;
    points.syncFromCurve(currentCurve()); // seed the empty field
  }

  // Undo/redo: Ctrl/Cmd+Z, and Ctrl/Cmd+Shift+Z or Ctrl+Y to redo.
  window.addEventListener("keydown", (event) => {
    if (!event.ctrlKey && !event.metaKey) {
      return;
    }
    // Undo/redo belongs to the drawing; a derived (calculus) view ignores it.
    if (math?.calc.isDerived()) {
      return;
    }
    const key = event.key.toLowerCase();
    if (key === "z") {
      event.preventDefault();
      if (event.shiftKey) {
        redo();
      } else {
        undo();
      }
    } else if (key === "y") {
      event.preventDefault();
      redo();
    }
  });

  const hint = document.querySelector("#controls-hint");
  if (hint) {
    hint.textContent =
      "Draw: left-drag · Edit: drag a dot or handle · Move: drag the curve · Undo: Ctrl+Z · Pan: two-finger · Zoom: pinch";
  }
}
