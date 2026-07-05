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
