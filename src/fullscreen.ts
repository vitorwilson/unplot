import { getCurrentWindow } from "@tauri-apps/api/window";

// Desktop platforms — Windows in particular — offer no built-in gesture to make
// an app window fullscreen (tiling WMs like Hyprland do it themselves, so the
// user never notices the gap). unplot provides its own toggle, bound to F11 in
// main.ts. This module wraps the Tauri window API behind a small interface so
// nothing else imports it and the toggle stays unit-testable with a fake.

/** The slice of the Tauri window the fullscreen toggle needs. */
export interface FullscreenWindow {
  isFullscreen(): Promise<boolean>;
  setFullscreen(fullscreen: boolean): Promise<void>;
}

/**
 * Flip the window between fullscreen and windowed, returning the new state.
 *
 * @example
 * await toggleFullscreen(appWindow()); // windowed -> fullscreen, resolves true
 */
export async function toggleFullscreen(
  window: FullscreenWindow,
): Promise<boolean> {
  const next = !(await window.isFullscreen());
  await window.setFullscreen(next);
  return next;
}

/** A keydown that should toggle fullscreen: F11 everywhere (the Windows and
 * Linux convention), or ⌘⌃F on macOS. */
export function isFullscreenShortcut(event: {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
}): boolean {
  if (event.key === "F11") {
    return true;
  }
  return event.metaKey && event.ctrlKey && event.key.toLowerCase() === "f";
}

/** The live application window, adapted to {@link FullscreenWindow}. */
export function appWindow(): FullscreenWindow {
  return getCurrentWindow();
}
