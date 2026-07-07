import katex from "katex";
import type { CurveLatex } from "./fit";

// The collapsible LaTeX panel: a clickable summary line that expands to the full
// piecewise cases block (rendered by KaTeX), plus a format picker and a Copy
// button. The picker chooses which form of the same function lands on the
// clipboard — raw LaTeX, Desmos, or Wolfram (docs/PLAN.md, Phase 4.5) — while the
// expanded math always shows the human-readable LaTeX. Collapsed by default so a
// hundred-segment curve stays legible (Phase 4).

const COPY_DONE = "Copied!";
const COPY_FAIL = "Copy failed";
const COPY_APPROX_IDLE = "Copy";
const COPY_RESET_MS = 1200;

/** A clipboard target. The string values are the `CurveLatex` field names, so
 * `result[format]` yields the text to copy. */
export type CopyFormat = "latex" | "desmos" | "wolfram";

const COPY_LABELS: Record<CopyFormat, string> = {
  latex: "Copy LaTeX",
  desmos: "Copy for Desmos",
  wolfram: "Copy for Wolfram",
};

/** The copy button's idle label for a chosen target. Pure, so it is unit-tested. */
export function copyLabel(format: CopyFormat): string {
  return COPY_LABELS[format];
}

/** The string to copy for a chosen target. Pure, so it is unit-tested. */
export function formatText(result: CurveLatex, format: CopyFormat): string {
  return result[format];
}

/** The summary line's label, with a disclosure triangle for its state. Pure, so
 * it is unit-tested. */
export function summaryLabel(summary: string, expanded: boolean): string {
  return `${expanded ? "▾" : "▸"} ${summary}`;
}

/** The approximation's error line, as percentages of the curve's y-range. Pure,
 * so it is unit-tested. */
export function approxErrorLabel(maxError: number, rmsError: number): string {
  const pct = (v: number): string => `${(v * 100).toFixed(2)}%`;
  return `approximation · max ${pct(maxError)} · RMS ${pct(rmsError)}`;
}

/** The panel's DOM elements, injected so the view has no id coupling. */
export interface LatexElements {
  panel: HTMLElement;
  summaryButton: HTMLButtonElement;
  formatSelect: HTMLSelectElement;
  copyButton: HTMLButtonElement;
  body: HTMLElement;
  math: HTMLElement;
  // The "prettier function" headline, shown only when the core offers one.
  approxPanel: HTMLElement;
  approxMath: HTMLElement;
  approxError: HTMLElement;
  approxCopy: HTMLButtonElement;
}

/** Shows a curve's LaTeX in the panel. */
export interface LatexView {
  show(result: CurveLatex): void;
  message(text: string): void;
  hide(): void;
}

export function installLatexView(el: LatexElements): LatexView {
  let current: CurveLatex | null = null;
  let expanded = false;

  const selectedFormat = (): CopyFormat => el.formatSelect.value as CopyFormat;

  const render = (): void => {
    if (!current) {
      return;
    }
    el.summaryButton.textContent = summaryLabel(current.summary, expanded);
    el.summaryButton.setAttribute("aria-expanded", String(expanded));
    el.copyButton.textContent = copyLabel(selectedFormat());
    el.body.hidden = !expanded;
    if (expanded) {
      katex.render(current.latex, el.math, {
        displayMode: true,
        throwOnError: false,
      });
    }
  };

  el.summaryButton.addEventListener("click", () => {
    if (!current) {
      return;
    }
    expanded = !expanded;
    render();
  });

  // Switching the target relabels the button so the user sees what will copy.
  el.formatSelect.addEventListener("change", () => {
    el.copyButton.textContent = copyLabel(selectedFormat());
  });

  const idleLabel = (): string => copyLabel(selectedFormat());
  el.copyButton.addEventListener("click", () => {
    if (!current) {
      return;
    }
    void navigator.clipboard
      .writeText(formatText(current, selectedFormat()))
      .then(() => flashCopy(el.copyButton, COPY_DONE, idleLabel))
      .catch(() => flashCopy(el.copyButton, COPY_FAIL, idleLabel));
  });

  // The "prettier function" headline: rendered above the exact output when the
  // core offers a trustworthy closed form, hidden otherwise.
  const showApproximation = (): void => {
    const approx = current?.approximation ?? null;
    el.approxPanel.hidden = !approx;
    if (approx) {
      katex.render(approx.latex, el.approxMath, {
        displayMode: true,
        throwOnError: false,
      });
      el.approxError.textContent = approxErrorLabel(
        approx.maxError,
        approx.rmsError,
      );
    }
  };

  const approxIdle = (): string => COPY_APPROX_IDLE;
  el.approxCopy.addEventListener("click", () => {
    const approx = current?.approximation;
    if (!approx) {
      return;
    }
    void navigator.clipboard
      .writeText(approx.latex)
      .then(() => flashCopy(el.approxCopy, COPY_DONE, approxIdle))
      .catch(() => flashCopy(el.approxCopy, COPY_FAIL, approxIdle));
  });

  return {
    show(result: CurveLatex): void {
      current = result;
      expanded = false;
      el.panel.hidden = false;
      el.copyButton.hidden = false;
      el.formatSelect.hidden = false;
      el.approxCopy.textContent = COPY_APPROX_IDLE;
      showApproximation();
      render();
    },
    message(text: string): void {
      current = null;
      expanded = false;
      el.panel.hidden = false;
      el.copyButton.hidden = true;
      el.formatSelect.hidden = true;
      el.body.hidden = true;
      el.approxPanel.hidden = true;
      el.summaryButton.textContent = text;
    },
    hide(): void {
      current = null;
      expanded = false;
      el.panel.hidden = true;
      el.approxPanel.hidden = true;
    },
  };
}

/** Briefly show `flash` on the copy button, then restore the idle label. `idle`
 * is read at reset time (not at flash time) so switching format mid-flash — which
 * relabels the button immediately — isn't clobbered when the timeout fires. */
function flashCopy(
  button: HTMLButtonElement,
  flash: string,
  idle: () => string,
): void {
  button.textContent = flash;
  setTimeout(() => {
    button.textContent = idle();
  }, COPY_RESET_MS);
}
