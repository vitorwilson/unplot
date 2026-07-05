# back-desmos

**Draw a function. Get the math.**

Desmos plots a formula you type. This does the reverse: you *draw* a smooth
curve `y = f(x)` on a Cartesian plane, and the app hands back the exact
function — as clean LaTeX you can differentiate, integrate, edit, and save.

What you get back is exactly what you drew: a shape-preserving spline that is a
valid function by construction, not a guess. An optional layer will *also*
suggest a compact closed form when your drawing really is a simple function —
always shown with its error, never presented as exact.

## Quickstart

> Desktop app for Windows, macOS, and Linux. Scaffolding is in progress
> (Phase 0), so these commands come online as the app takes shape.

```sh
just dev     # run the desktop app
just test    # run the full test suite (Rust core + frontend)
```

## Docs

- Roadmap and locked design decisions: [`docs/PLAN.md`](docs/PLAN.md)
