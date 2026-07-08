import { describe, expect, it } from "vitest";
import {
  isFullscreenShortcut,
  toggleFullscreen,
  type FullscreenWindow,
} from "./fullscreen";

/** An in-memory stand-in for the OS window that records the fullscreen flag, so
 * the toggle can be exercised headlessly without Tauri. */
class FakeWindow implements FullscreenWindow {
  constructor(public fullscreen = false) {}
  isFullscreen(): Promise<boolean> {
    return Promise.resolve(this.fullscreen);
  }
  setFullscreen(fullscreen: boolean): Promise<void> {
    this.fullscreen = fullscreen;
    return Promise.resolve();
  }
}

describe("toggleFullscreen", () => {
  it("enters fullscreen from a windowed start", async () => {
    const window = new FakeWindow(false);
    expect(await toggleFullscreen(window)).toBe(true);
    expect(window.fullscreen).toBe(true);
  });

  it("leaves fullscreen when already fullscreen", async () => {
    const window = new FakeWindow(true);
    expect(await toggleFullscreen(window)).toBe(false);
    expect(window.fullscreen).toBe(false);
  });

  it("round-trips back to windowed over two toggles", async () => {
    const window = new FakeWindow(false);
    await toggleFullscreen(window);
    await toggleFullscreen(window);
    expect(window.fullscreen).toBe(false);
  });
});

describe("isFullscreenShortcut", () => {
  const event = (over: Partial<KeyboardEvent>): KeyboardEvent =>
    ({ key: "", ctrlKey: false, metaKey: false, ...over }) as KeyboardEvent;

  it("fires on F11 with no modifier (Windows/Linux convention)", () => {
    expect(isFullscreenShortcut(event({ key: "F11" }))).toBe(true);
  });

  it("fires on Cmd+Ctrl+F (macOS convention)", () => {
    expect(
      isFullscreenShortcut(event({ key: "f", metaKey: true, ctrlKey: true })),
    ).toBe(true);
  });

  it("ignores a plain F key", () => {
    expect(isFullscreenShortcut(event({ key: "f" }))).toBe(false);
  });

  it("ignores Cmd+F alone (browser find, not fullscreen)", () => {
    expect(isFullscreenShortcut(event({ key: "f", metaKey: true }))).toBe(
      false,
    );
  });
});
