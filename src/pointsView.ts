import type { FittedCurve, Knot } from "./fit";
import { formatPoints, parsePoints } from "./points";

// The collapsible Points panel: type `x, y` per line and Plot to (re)build the
// curve from those points; the field mirrors the curve, refreshing whenever the
// curve changes and the field is not focused (so it never clobbers mid-typing).
// Drawing and typing are two editors of the same knot set.

const MIN_POINTS = 2;

/** The disclosure label for the panel toggle. Pure, so it is unit-tested. */
export function pointsToggleLabel(expanded: boolean): string {
  return `${expanded ? "▾" : "▸"} Points`;
}

/** The panel's DOM elements, injected so the view has no id coupling. */
export interface PointsElements {
  toggleButton: HTMLButtonElement;
  body: HTMLElement;
  textarea: HTMLTextAreaElement;
  plotButton: HTMLButtonElement;
  message: HTMLElement;
}

/** What the panel needs from the rest of the app. */
export interface PointsDeps {
  refit: (knots: Knot[]) => Promise<FittedCurve>;
  loadCurve: (curve: FittedCurve) => void;
  resetDerived: () => void;
}

/** Keeps the field in step with the curve. */
export interface PointsController {
  syncFromCurve: (curve: FittedCurve | null) => void;
}

export function installPointsView(
  el: PointsElements,
  deps: PointsDeps,
): PointsController {
  let expanded = false;

  const setExpanded = (next: boolean): void => {
    expanded = next;
    el.toggleButton.textContent = pointsToggleLabel(expanded);
    el.toggleButton.setAttribute("aria-expanded", String(expanded));
    el.body.hidden = !expanded;
  };

  const say = (text: string): void => {
    el.message.textContent = text;
  };

  el.toggleButton.addEventListener("click", () => setExpanded(!expanded));

  el.plotButton.addEventListener("click", () => {
    const result = parsePoints(el.textarea.value);
    if (!result.ok) {
      say(result.error);
      return;
    }
    if (result.points.length < MIN_POINTS) {
      say(`Enter at least ${MIN_POINTS} points.`);
      return;
    }
    const knots: Knot[] = result.points.map((p) => ({
      x: p.x,
      y: p.y,
      tangent: null,
      slope: 0,
    }));
    void deps
      .refit(knots)
      .then((fitted) => {
        deps.resetDerived();
        deps.loadCurve(fitted);
        say("");
      })
      .catch(() => say("Couldn't plot those points."));
  });

  setExpanded(false);

  return {
    syncFromCurve: (curve: FittedCurve | null): void => {
      if (document.activeElement === el.textarea) {
        return; // don't overwrite what the user is typing
      }
      el.textarea.value = curve ? formatPoints(curve.knots) : "";
    },
  };
}
