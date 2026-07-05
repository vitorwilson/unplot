/**
 * Device-pixel dimensions for a CSS-sized canvas, so drawing stays crisp on
 * high-DPI displays.
 *
 * @example
 * canvasPixelSize(640, 480, 2); // { width: 1280, height: 960 }
 */
export function canvasPixelSize(
  cssWidth: number,
  cssHeight: number,
  devicePixelRatio: number,
): { width: number; height: number } {
  if (cssWidth < 0 || cssHeight < 0) {
    throw new Error(
      `canvasPixelSize: dimensions must be non-negative, got ${cssWidth}x${cssHeight}`,
    );
  }
  // A zero or negative ratio (e.g. from a headless/mocked environment) would
  // collapse the canvas; fall back to 1:1.
  const ratio = devicePixelRatio > 0 ? devicePixelRatio : 1;
  return {
    width: Math.round(cssWidth * ratio),
    height: Math.round(cssHeight * ratio),
  };
}
