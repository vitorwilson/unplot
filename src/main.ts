import { canvasPixelSize } from "./dpr";
import { installDrawing } from "./draw";
import { fitStroke } from "./fit";
import { visibleGridLines } from "./grid";
import { worldToScreen, type Viewport } from "./viewport";

// Phase 2: a Cartesian plane on Canvas 2D. Pointer capture, hard-block drawing,
// and lift-and-resume build on this next (see docs/PLAN.md).

const CSS_WIDTH = 640;
const CSS_HEIGHT = 480;
const GRID_STEP = 1;
const GRID_COLOR = "#e3e3e3";
const AXIS_COLOR = "#8a8a8a";
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

function drawGrid(ctx: CanvasRenderingContext2D, vp: Viewport): void {
  ctx.strokeStyle = GRID_COLOR;
  ctx.lineWidth = 1;
  const { xs, ys } = visibleGridLines(vp, CSS_WIDTH, CSS_HEIGHT, GRID_STEP);
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

function drawAxes(ctx: CanvasRenderingContext2D, vp: Viewport): void {
  const origin = worldToScreen(vp, { x: 0, y: 0 });
  ctx.strokeStyle = AXIS_COLOR;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.moveTo(0, origin.y);
  ctx.lineTo(CSS_WIDTH, origin.y);
  ctx.moveTo(origin.x, 0);
  ctx.lineTo(origin.x, CSS_HEIGHT);
  ctx.stroke();
}

function drawPlane(ctx: CanvasRenderingContext2D, vp: Viewport): void {
  ctx.clearRect(0, 0, CSS_WIDTH, CSS_HEIGHT);
  drawGrid(ctx, vp);
  drawAxes(ctx, vp);
}

const canvas = document.querySelector<HTMLCanvasElement>("#plane");
if (canvas) {
  const ctx = setupCanvas(canvas);
  const viewport = centeredViewport();
  const redrawBackground = () => drawPlane(ctx, viewport);
  redrawBackground();
  installDrawing(canvas, ctx, viewport, MAX_SLOPE, redrawBackground, fitStroke);
}
