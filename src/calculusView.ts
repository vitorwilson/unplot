import type { CalcCurve, CalcOp, CurveLatex, Knot } from "./fit";
import type { LatexView } from "./latexView";
import type { Point } from "./viewport";

// The calculus controls (Phase 5): d/dx and ∫ replace the shown curve with its
// derivative or integral and can be chained; Reset returns to the editable
// drawing. The result is read-only — editing applies only to what you drew — and
// the original knots stay the source of truth, so each click just replays the
// whole operation stack through the core.

const OP_SYMBOL: Record<CalcOp, string> = {
  differentiate: "d/dx",
  integrate: "∫",
};

/** Breadcrumb of the operation chain, e.g. `"f → d/dx → ∫"`. Pure — unit-tested. */
export function calcTitle(ops: CalcOp[]): string {
  return ["f", ...ops.map((op) => OP_SYMBOL[op])].join(" → ");
}

/** An honest note about the shown curve's smoothness, from the last operation: a
 * derivative is continuous but has corners at the knots (C⁰); an integral is
 * smooth (C²). Empty when nothing has been applied. Pure — unit-tested. */
export function calcNote(ops: CalcOp[]): string {
  const last = ops.at(-1);
  if (last === "differentiate") {
    return " — continuous, but with corners at the knots";
  }
  if (last === "integrate") {
    return " — continuous and smooth";
  }
  return "";
}

/** The panel summary for a derived curve: the chain breadcrumb, the core's own
 * segment/domain summary, and the smoothness note. */
function derivedSummary(result: CalcCurve, ops: CalcOp[]): CurveLatex {
  return {
    summary: `${calcTitle(ops)}: ${result.summary}${calcNote(ops)}`,
    latex: result.latex,
    desmos: result.desmos,
    wolfram: result.wolfram,
    approximation: result.approximation,
  };
}

/** The calculus buttons, injected so the controller has no id coupling. */
export interface CalculusElements {
  dxButton: HTMLButtonElement;
  integralButton: HTMLButtonElement;
  resetButton: HTMLButtonElement;
  doneButton: HTMLButtonElement;
}

/** Everything the controller needs from the rest of the app. */
export interface CalculusDeps {
  currentKnots: () => Knot[] | null;
  applyCalculus: (knots: Knot[], ops: CalcOp[]) => Promise<CalcCurve>;
  showDerived: (polyline: Point[]) => void;
  clearDerived: () => void;
  view: LatexView;
}

/** Controls a derived (calculus) view. `isDerived` lets the app suspend undo/redo
 * (which belongs to the drawing) while a result is shown; `reset` returns to the
 * drawing, e.g. when a file is opened. */
export interface CalculusController {
  isDerived: () => boolean;
  reset: () => void;
}

export function installCalculusView(
  el: CalculusElements,
  deps: CalculusDeps,
): CalculusController {
  let ops: CalcOp[] = [];

  // In a derived view, Done (which targets the drawing) gives way to Reset.
  const syncButtons = (): void => {
    const derived = ops.length > 0;
    el.doneButton.hidden = derived;
    el.resetButton.hidden = !derived;
  };

  const apply = (op: CalcOp): void => {
    const knots = deps.currentKnots();
    if (!knots || knots.length < 2) {
      deps.view.message("Draw a function first.");
      return;
    }
    const next = [...ops, op];
    void deps
      .applyCalculus(knots, next)
      .then((result) => {
        ops = next;
        deps.showDerived(result.polyline);
        deps.view.show(derivedSummary(result, ops));
        syncButtons();
      })
      .catch(() => deps.view.message("Couldn't compute that."));
  };

  const reset = (): void => {
    ops = [];
    deps.clearDerived();
    deps.view.hide();
    syncButtons();
  };

  el.dxButton.addEventListener("click", () => apply("differentiate"));
  el.integralButton.addEventListener("click", () => apply("integrate"));
  el.resetButton.addEventListener("click", reset);
  syncButtons();

  return { isDerived: () => ops.length > 0, reset };
}
