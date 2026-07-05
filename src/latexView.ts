import katex from "katex";
import type { CurveLatex } from "./fit";

// The collapsible LaTeX panel: a clickable summary line that expands to the full
// piecewise cases block, rendered by KaTeX. Collapsed by default so a
// hundred-segment curve stays legible (docs/PLAN.md, Phase 4).

/** The summary line's label, with a disclosure triangle for its state. Pure, so
 * it is unit-tested. */
export function summaryLabel(summary: string, expanded: boolean): string {
  return `${expanded ? "▾" : "▸"} ${summary}`;
}

/** Shows a curve's LaTeX in the panel. */
export interface LatexView {
  show(result: CurveLatex): void;
  message(text: string): void;
}

export function installLatexView(
  panel: HTMLElement,
  summaryButton: HTMLButtonElement,
  body: HTMLElement,
): LatexView {
  let current: CurveLatex | null = null;
  let expanded = false;

  const render = (): void => {
    if (!current) {
      return;
    }
    summaryButton.textContent = summaryLabel(current.summary, expanded);
    body.hidden = !expanded;
    if (expanded) {
      katex.render(current.latex, body, {
        displayMode: true,
        throwOnError: false,
      });
    }
  };

  summaryButton.addEventListener("click", () => {
    if (!current) {
      return;
    }
    expanded = !expanded;
    render();
  });

  return {
    show(result: CurveLatex): void {
      current = result;
      expanded = false;
      panel.hidden = false;
      render();
    },
    message(text: string): void {
      current = null;
      expanded = false;
      panel.hidden = false;
      body.hidden = true;
      summaryButton.textContent = text;
    },
  };
}
