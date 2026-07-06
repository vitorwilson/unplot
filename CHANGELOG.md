# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Development roadmap (`docs/PLAN.md`).
- Rust workspace with the headless `curve-engine` core crate (Phase 0 scaffold).
- Repo hygiene: `.gitignore`, `README.md`, `config/deploy.env.example`.
- Web frontend scaffold (Vite + TypeScript + Vitest + ESLint/Prettier) with a
  high-DPI canvas grid placeholder and a tested `canvasPixelSize` helper.
- Tauri v2 desktop shell (`src-tauri/`) wired to the core through an
  `engine_version` command, establishing the frontend → shell → core path.
- `just` task runner (`dev`/`build`/`test`/`lint`/`fmt`/`audit`) and GitHub
  Actions CI running the full formatter/linter/test/audit suite on every push.
- Curve engine (Phase 1): a validated `Knot`/`Curve` model plus shape-preserving
  PCHIP spline fitting (`Curve::fit`) and domain-clamped evaluation
  (`Spline::eval`) — C¹ across joins, no overshoot, headless and unit-tested.
- Hard-block validators (`advances_in_x`, `within_slope_cap`, `edit_keeps_order`):
  pure predicates the drawing UI uses to refuse invalid input at capture time.
- Resume drawing: `Curve::extend` appends a stroke that joins C¹ (pinning the
  join to the previous ending slope); `Spline::start_slope`/`end_slope` expose
  the boundary slopes.
- Resample: `resample` thins dense, noisy pen samples to a minimal knot set via
  Ramer–Douglas–Peucker (curvature-aware), so the spline stays faithful without
  overfitting jitter.
- Cartesian plane (Phase 2): a Canvas 2D grid and axes driven by pure,
  unit-tested world↔screen viewport transforms (`worldToScreen`, `screenToWorld`,
  `pan`, `zoomAt`) and visible-gridline math.
- Hard-block freehand drawing (Phase 2): pointer capture with an instant "wall" —
  the pen cannot reverse in x or exceed the slope cap (`StrokeBuilder`, mirroring
  the Rust validators) — drawn live on the plane.
- Fit through the core (Phase 2): on stroke end the frontend sends samples to the
  Rust `fit_curve` command (resample → validate → fit) and renders the returned
  smooth spline (`Spline::polyline`) instead of the raw polyline.
- Lift & resume (Phase 2): later strokes extend the current curve through the
  `extend_curve` command, joining C¹; the pen hard-blocks against the previous
  endpoint so a resumed stroke can't restart behind where it left off.
- Plane navigation (Phase 2): wheel-zoom about the cursor and right-drag pan,
  with a zoom-aware "nice" grid step (`tickStep`) and numeric axis labels, so the
  user can pan right to resume drawing off-screen.
- Centered, styled layout (`styles.css`): the plane sits centered as a white
  panel with a title and an on-screen controls hint, replacing the unstyled
  left-aligned page.
- Light/dark theming (cross-cutting foundation): shared color tokens for both the
  CSS chrome and the Canvas 2D surfaces (grid, axes, labels, curve, edit handles),
  a persisted toggle, and a system-preference default. Every later phase's UI is
  theme-aware by construction.
- Editing foundation (Phase 3): knots now carry their tangents across IPC, a
  `refit_curve` command re-fits edited knots without resampling, knot points are
  drawn as grab dots, and `nearestKnot` hit-tests them in screen space.
- UI polish: larger plane (860×600) in a roomier window, more breathing room
  between the plane and its labels, and an inline theme bootstrap that sets the
  palette before first paint so a reload no longer flashes the wrong colors.
- Trackpad navigation: two-finger scroll pans the plane; pinch (or Ctrl + scroll,
  a WebKitGTK-safe fallback) zooms about the cursor. Right-drag still pans for
  mice. Replaces the Space+drag pan, which didn't work on trackpads.
- Draggable knot points (Phase 3): grab a knot dot and drag to reshape the curve,
  re-fitting live through the core. `clampKnotDrag` hard-blocks the drag — a knot
  can't cross a neighbor's x or exceed the slope cap — so the curve stays a valid
  function throughout.
- Draggable tangent handles (Phase 3): each knot shows a handle at its slope;
  drag the tip to set the slope directly (the "move the slope" interaction).
  The engine reports the effective slope per knot (`Spline::knot_slopes`), and
  the drag is clamped to the slope cap.
- Translate the whole curve (Phase 3): grab the curve body (away from any knot or
  handle) and drag to move all knots together. A rigid translation preserves the
  shape exactly, so the knots and polyline shift locally with no re-fit
  (`offsetCurve`); `nearPolyline` hit-tests the curve body.
- Undo/redo (Phase 3): Ctrl/Cmd+Z undoes and Ctrl/Cmd+Shift+Z or Ctrl+Y redoes
  every committed change — draw, resume, point/tangent edit, translate — over a
  snapshot `History`. A click that doesn't move records nothing.
- LaTeX generation (Phase 4): the engine renders the fitted spline as an exact
  piecewise `cases` block plus a one-line summary, deterministically from its own
  coefficients (no CAS). Exposed via the `curve_latex` command.
- LaTeX output UI (Phase 4): a "Done" button renders the exact function as pretty
  math via offline-bundled KaTeX, shown collapsed (the summary line) and
  expandable to the full piecewise cases block.
- Copy LaTeX: a button copies the function's LaTeX source to the clipboard, to
  paste into other tools.
- Export to Desmos & Wolfram (Phase 4.5): the core also serializes the fitted
  spline as Desmos piecewise LaTeX (`\left\{cond: expr, …\right\}`) and Wolfram
  `Piecewise[{{expr, cond}, …}]` — deterministically, headless-tested per target
  (`export::desmos`, `export::wolfram`). A format picker beside the Copy button
  chooses which form (raw LaTeX / Desmos / Wolfram) lands on the clipboard, so a
  drawn curve pastes straight into either tool. Shared number/polynomial
  formatting moved into one `coeffs` module used by all three targets.
- Calculus engine (Phase 5): the core differentiates and integrates a fitted
  curve analytically, per segment, with no CAS (`calculus::differentiate`,
  `calculus::integrate`). The derivative is the exact continuous piecewise
  polynomial one degree lower; the integral is the antiderivative with
  `F(x_first) = 0`, its per-segment constant accumulating the area so it stays
  continuous across joins (and C²). Results are themselves splines, so they
  evaluate, render as LaTeX/Desmos/Wolfram, and chain (differentiate the
  integral, …). Segment coefficients generalized to variable degree to hold the
  higher-degree results.
- Calculus UI (Phase 5): `d/dx` and `∫ dx` buttons replace the drawn curve on
  the plane with its derivative or integral and show its math in the panel. The
  result is read-only and chainable (differentiate the integral, …), and "Reset
  to drawing" returns to the editable original. The panel labels the chain (e.g.
  `f → d/dx`) and flags, honestly, that a derivative is continuous but has
  corners at the knots while an integral is smooth.
- Document format (Phase 6): the core serializes a curve to and from a versioned
  `.unplot` JSON document (`document` module) — the source-of-truth knots (x, y,
  optional tangent) and domain, never a rendered image — so a saved curve reopens
  fully editable and re-derives its LaTeX and calculus. `from_json` checks the
  schema version, refusing files newer than it understands and leaving one place
  to migrate older ones forward. Round-trip tested.
- Save / open files (Phase 6): Save and Open buttons write and read `.unplot`
  documents through the native file dialog; opening a file loads it as the fully
  editable current curve (re-fit and re-derivable) and clears any calculus view.
  A saved curve created on one platform reopens identically on the others.
- Point input (data plotting): a Points panel lets you type `x, y` per line and
  Plot to build the curve from those points — sorted by x, with duplicate x
  rejected (a function has one y per x). It mirrors the drawing two-way: drawing
  or editing shows up as points, and editing the text and re-plotting updates the
  curve, so typing and drawing combine on one point set. The field refreshes from
  the canvas only when it is not focused, so it never clobbers mid-typing.
- Prettier function (Phase 7, engine): when the drawn curve is basically a simple
  function, the core can now propose a compact closed form (`approximate` module).
  It samples the fitted spline and searches sparse least-squares fits over a fixed
  basis `{1, x, x², x³, sin x, cos x, eˣ, ln x}` (log/exp auto-skipped where they
  are undefined or overflow on the domain), snaps coefficients toward round values,
  and reports honest max/RMS error — offering the fewest-term form only when its
  error is within 3% of the curve's range, otherwise staying silent so the exact
  piecewise output stands alone. Pure-Rust (nalgebra), deterministic, no CAS.
- Prettier function — waves (Phase 7, step 2): the approximator now also fits a
  free-frequency sinusoid `A·sin(ωx) + B·cos(ωx)` by sweeping ω (a periodogram),
  catching a drawn wave of any frequency that the fixed frequency-1 trig basis
  misses — e.g. `sin(2x)` or `3cos(1.5x)`. The fixed-basis and sinusoid strategies
  compete and the simplest trustworthy form wins, still error-gated.
- Prettier function UI (Phase 7): pressing Done now also asks the core for a
  closed form; when one is offered it appears as the headline of the math panel —
  `f(x) ≈ 2x²` rendered by KaTeX, with its max/RMS error as a percentage of the
  range and an "approximation" tag — above the exact piecewise output, which stays
  put and unchanged. The clean form has its own Copy button. When nothing is
  trustworthy the headline is simply absent. The same headline appears for a
  derivative or integral (e.g. d/dx of a parabola shows `f(x) ≈ 2x`).

### Fixed

- Reload no longer flashes: `styles.css` now loads render-blocking via a `<link>`
  in `index.html` (instead of being injected by JS after first paint), and the
  canvas reserves its size in CSS, so a reload no longer shows an instant of
  unstyled layout or a jump from the browser-default canvas size.
