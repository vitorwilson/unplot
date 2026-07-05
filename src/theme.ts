// Light/dark theming for every rendered surface. CSS variables cover the page
// chrome (see styles.css); Canvas 2D can't read those, so the same palette is
// mirrored here as tokens the renderer queries. Pure resolution/token helpers
// are unit-tested; `installTheme` is the thin DOM/storage controller.

export type Theme = "light" | "dark";

/** Canvas colors for one theme. Handle colors are for Phase 3 edit handles. */
export interface CanvasColors {
  grid: string;
  axis: string;
  label: string;
  curve: string;
  handle: string;
  handleActive: string;
}

const LIGHT: CanvasColors = {
  grid: "#e3e3e3",
  axis: "#8a8a8a",
  label: "#5a5a5a",
  curve: "#1565c0",
  handle: "#1565c0",
  handleActive: "#0d47a1",
};

const DARK: CanvasColors = {
  grid: "#2a2d34",
  axis: "#565b66",
  label: "#9aa0a6",
  curve: "#4da3ff",
  handle: "#4da3ff",
  handleActive: "#a6d2ff",
};

const STORAGE_KEY = "back-desmos-theme";

/** Canvas palette for a theme. */
export function canvasColors(theme: Theme): CanvasColors {
  return theme === "dark" ? DARK : LIGHT;
}

/** The other theme. */
export function nextTheme(theme: Theme): Theme {
  return theme === "dark" ? "light" : "dark";
}

/**
 * Initial theme: an explicit stored choice wins; otherwise follow the system
 * preference.
 *
 * @example
 * resolveInitialTheme(null, true); // "dark" — no choice saved, system is dark
 */
export function resolveInitialTheme(
  stored: string | null,
  prefersDark: boolean,
): Theme {
  if (stored === "light" || stored === "dark") {
    return stored;
  }
  return prefersDark ? "dark" : "light";
}

/** Reads the current theme, its canvas colors, and toggles it (persisting the
 * choice and updating the document), notifying `onChange` so callers repaint. */
export interface ThemeController {
  current(): Theme;
  colors(): CanvasColors;
  toggle(): void;
}

export function installTheme(
  onChange: (theme: Theme) => void,
): ThemeController {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  let theme = resolveInitialTheme(
    localStorage.getItem(STORAGE_KEY),
    prefersDark,
  );
  document.documentElement.setAttribute("data-theme", theme);

  return {
    current: () => theme,
    colors: () => canvasColors(theme),
    toggle() {
      theme = nextTheme(theme);
      localStorage.setItem(STORAGE_KEY, theme);
      document.documentElement.setAttribute("data-theme", theme);
      onChange(theme);
    },
  };
}
