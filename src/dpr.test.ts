import { describe, expect, it } from "vitest";
import { canvasPixelSize } from "./dpr";

describe("canvasPixelSize", () => {
  it("scales CSS dimensions by the device pixel ratio", () => {
    expect(canvasPixelSize(640, 480, 2)).toEqual({ width: 1280, height: 960 });
  });

  it("rounds to whole device pixels", () => {
    expect(canvasPixelSize(100, 100, 1.5)).toEqual({ width: 150, height: 150 });
  });

  it("falls back to ratio 1 for a non-positive ratio", () => {
    expect(canvasPixelSize(300, 200, 0)).toEqual({ width: 300, height: 200 });
  });

  it("rejects negative dimensions", () => {
    expect(() => canvasPixelSize(-1, 10, 2)).toThrow(/non-negative/);
  });
});
