import { canvasPixelSize } from "./dpr";

// Phase 0: prove the app is alive by drawing a Cartesian grid placeholder on a
// high-DPI canvas. The interactive plane and hard-block input land in Phase 2
// (see docs/PLAN.md).

const CSS_WIDTH = 640;
const CSS_HEIGHT = 480;
const GRID_STEP = 32;

function drawPlaceholderGrid(canvas: HTMLCanvasElement): void {
  const dpr = window.devicePixelRatio || 1;
  const { width, height } = canvasPixelSize(CSS_WIDTH, CSS_HEIGHT, dpr);
  canvas.width = width;
  canvas.height = height;
  canvas.style.width = `${CSS_WIDTH}px`;
  canvas.style.height = `${CSS_HEIGHT}px`;

  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("drawPlaceholderGrid: 2D canvas context unavailable");
  }
  ctx.scale(dpr, dpr);
  ctx.strokeStyle = "#ccc";
  for (let x = 0; x <= CSS_WIDTH; x += GRID_STEP) {
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, CSS_HEIGHT);
    ctx.stroke();
  }
  for (let y = 0; y <= CSS_HEIGHT; y += GRID_STEP) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(CSS_WIDTH, y);
    ctx.stroke();
  }
}

const canvas = document.querySelector<HTMLCanvasElement>("#plane");
if (canvas) {
  drawPlaceholderGrid(canvas);
}
