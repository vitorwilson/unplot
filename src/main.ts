import "./styles.css";
import { canvasPixelSize } from "./dpr";
import { installDrawing } from "./draw";
import { extendStroke, fitStroke, refitCurve } from "./fit";
import { tickStep, visibleGridLines } from "./grid";
import { installViewportControls } from "./navigate";
import { installTheme, type CanvasColors } from "./theme";
import { worldToScreen, type Viewport } from "./viewport";

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
  const { redraw } = installDrawing(
    canvas,
    ctx,
    viewport,
    MAX_SLOPE,
    redrawBackground,
    { fit: fitStroke, extend: extendStroke, refit: refitCurve },
    () => theme.colors(),
  );
  repaint = redraw;
  installViewportControls(canvas, viewport, redraw);

  const hint = document.querySelector("#controls-hint");
  if (hint) {
    hint.textContent =
      "Draw: left-drag · Move a point: drag a dot · Pan: two-finger or right-drag · Zoom: pinch or Ctrl-scroll";
  }
}
