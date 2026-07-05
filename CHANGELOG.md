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
- Trackpad-friendly pan: hold Space and left-drag to pan (works on a trackpad,
  unlike right-drag, which still works for mice).
- Draggable knot points (Phase 3): grab a knot dot and drag to reshape the curve,
  re-fitting live through the core. `clampKnotDrag` hard-blocks the drag — a knot
  can't cross a neighbor's x or exceed the slope cap — so the curve stays a valid
  function throughout.
