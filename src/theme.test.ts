import { describe, expect, it } from "vitest";
import { canvasColors, nextTheme, resolveInitialTheme } from "./theme";

describe("resolveInitialTheme", () => {
  it("honors an explicit stored choice over the system preference", () => {
    expect(resolveInitialTheme("light", true)).toBe("light");
    expect(resolveInitialTheme("dark", false)).toBe("dark");
  });

  it("falls back to the system preference when nothing is stored", () => {
    expect(resolveInitialTheme(null, true)).toBe("dark");
    expect(resolveInitialTheme(null, false)).toBe("light");
  });

  it("ignores a malformed stored value", () => {
    expect(resolveInitialTheme("purple", true)).toBe("dark");
  });
});

describe("nextTheme", () => {
  it("flips between light and dark", () => {
    expect(nextTheme("light")).toBe("dark");
    expect(nextTheme("dark")).toBe("light");
  });
});

describe("canvasColors", () => {
  it("gives distinct palettes per theme", () => {
    expect(canvasColors("light").grid).not.toBe(canvasColors("dark").grid);
    expect(canvasColors("light").curve).not.toBe(canvasColors("dark").curve);
  });
});
