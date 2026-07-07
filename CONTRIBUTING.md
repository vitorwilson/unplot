# Contributing to unplot

Thanks for your interest! unplot is a desktop app that recovers the exact
function from a hand-drawn curve. This guide gets you from clone to pull request.

## Ground rules

- Be kind. This project follows a [Code of Conduct](CODE_OF_CONDUCT.md).
- Discuss non-trivial changes in an issue first, so effort isn't wasted.
- Every change keeps CI green and is independently useful.

## Development setup

You need [Rust](https://rustup.rs), [Node](https://nodejs.org) 22+,
[pnpm](https://pnpm.io), and [`just`](https://github.com/casey/just). On Linux
you also need the Tauri system dependencies (WebKitGTK etc.) — see the
[Tauri prerequisites](https://tauri.app/start/prerequisites/).

```sh
pnpm install        # frontend dependencies
just dev            # run the desktop app (Vite + Tauri)
just test           # full test suite: Rust core + frontend
just lint           # clippy (warnings as errors) + eslint
just fmt            # format everything (cargo fmt + prettier)
```

## Architecture in one minute

- `crates/curve-engine/` — the **headless Rust core**: curve model, spline
  fitting, calculus, the closed-form approximator, the symbolic layer, and file
  I/O. It never imports the UI and is fully unit-tested without Tauri.
- `src/` — the **web frontend**: Canvas 2D plane, drawing, editing, and the
  math panel (KaTeX). Pure logic is unit-tested (Vitest); DOM wiring is thin.
- `src-tauri/` — the **Tauri glue**: commands bridging the frontend to the core.

Design decisions and the roadmap live in [`docs/PLAN.md`](docs/PLAN.md).

## Conventions

These are enforced in review and, where possible, by CI:

- **TDD.** Write the test first. Every new function gets a test; every bug fix
  gets a regression test. Aim for at least as many lines of test as of code.
- **Small units.** Functions 4–20 lines; files under 500. One responsibility
  per module. Prefer early returns over deep nesting.
- **Explicit types.** No `any` in TypeScript; no untyped public functions in Rust.
- **No duplication.** Extract shared logic.
- **Honest output.** The recovered curve is exactly what was drawn; the
  "prettier function" is best-effort and always shown with its error. Never
  present an approximation as exact.
- **FOSS-only dependencies.** Permissively licensed only (this is a public,
  redistributed repo). No source-available/paid libraries.
- **Formatting is not a debate.** `cargo fmt` and `prettier` decide.

Run `just lint && just test` before you push — that's exactly what CI runs.

## Commits and pull requests

- Use [Conventional Commits](https://www.conventionalcommits.org):
  `feat(engine): …`, `fix(ui): …`, `docs: …`, `refactor: …`, `chore: …`.
- Keep commits small and focused; each should pass CI on its own.
- Update docs and `CHANGELOG.md` (the `[Unreleased]` section) in the same change
  that changes behavior.
- Open a PR against `main`, fill in the template, and make sure CI is green.

## Reporting bugs and requesting features

Use the [issue templates](https://github.com/vitorwilson/unplot/issues/new/choose).
For security issues, see [SECURITY.md](SECURITY.md) — please don't open a public
issue for those.
