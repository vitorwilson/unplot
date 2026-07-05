import katex from "katex";
import type { CurveLatex } from "./fit";

// The collapsible LaTeX panel: a clickable summary line that expands to the full
// piecewise cases block (rendered by KaTeX), plus a Copy button that puts the
// LaTeX source on the clipboard to paste elsewhere. Collapsed by default so a
// hundred-segment curve stays legible (docs/PLAN.md, Phase 4).

const COPY_IDLE = "Copy LaTeX";
const COPY_DONE = "Copied!";
const COPY_FAIL = "Copy failed";
const COPY_RESET_MS = 1200;

/** The summary line's label, with a disclosure triangle for its state. Pure, so
 * it is unit-tested. */
export function summaryLabel(summary: string, expanded: boolean): string {
  return `${expanded ? "▾" : "▸"} ${summary}`;
}

/** The panel's DOM elements, injected so the view has no id coupling. */
export interface LatexElements {
  panel: HTMLElement;
  summaryButton: HTMLButtonElement;
  copyButton: HTMLButtonElement;
  body: HTMLElement;
  math: HTMLElement;
}

/** Shows a curve's LaTeX in the panel. */
export interface LatexView {
  show(result: CurveLatex): void;
  message(text: string): void;
}

export function installLatexView(el: LatexElements): LatexView {
  let current: CurveLatex | null = null;
  let expanded = false;

  const render = (): void => {
    if (!current) {
      return;
    }
    el.summaryButton.textContent = summaryLabel(current.summary, expanded);
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

  el.copyButton.addEventListener("click", () => {
    if (!current) {
      return;
    }
    void navigator.clipboard
      .writeText(current.latex)
      .then(() => flashCopy(el.copyButton, COPY_DONE))
      .catch(() => flashCopy(el.copyButton, COPY_FAIL));
  });

  return {
    show(result: CurveLatex): void {
      current = result;
      expanded = false;
      el.panel.hidden = false;
      el.copyButton.hidden = false;
      render();
    },
    message(text: string): void {
      current = null;
      expanded = false;
      el.panel.hidden = false;
      el.copyButton.hidden = true;
      el.body.hidden = true;
      el.summaryButton.textContent = text;
    },
  };
}

/** Briefly show `label` on the copy button, then restore it. */
function flashCopy(button: HTMLButtonElement, label: string): void {
  button.textContent = label;
  setTimeout(() => {
    button.textContent = COPY_IDLE;
  }, COPY_RESET_MS);
}
