# Development Plan — "Desmos Backwards"

> This plan is maintained as the project evolves. Phases may shift based on 
> learning and community feedback. See the releases and CHANGELOG for what's 
> shipped and what's changed.

A desktop app where you *draw* a function and it hands you back the math.

This is the priority-ordered roadmap, built on TDD, small commits, and green CI
per commit. Phases are ordered so that risk is sequenced, not front-loaded: the
honest, always-works engine comes first; the magical-but-unreliable "pretty
function" layer comes last, when the app is already complete and truthful without
it.

---

## Sequencing principles

- **The Rust curve engine is the source of truth. Build and test it headless,
  before any UI.** Everything else (rendering, calculus, LaTeX, files) derives
  from it. If the engine is right, the rest is plumbing.
- **Every phase ends CI-green and independently demoable.** Small releases.
  No phase leaves the app in a state that can't be shown to a user.
- **Risk last.** The exact piecewise output is the backbone and always works.
  The symbolic "prettier function" layer is the least predictable part; it
  ships last and *never* replaces the truthful output — it sits beside it as an
  optional, error-gated view.
- **Hard-block, not soft-correct.** Invalid input (backward motion, spikes) is
  refused at capture time, so the engine never holds an invalid curve. This is
  a decision, not a preference — see Locked Decisions.

---

## Locked decisions (from the design discussion)

These are settled. Don't re-litigate them in code without raising it first.

- **Representation:** a single-valued `y = f(x)` as a **shape-preserving
  piecewise cubic Hermite spline** (PCHIP / Fritsch–Carlson tangents). C¹
  everywhere, local editing support, no spurious overshoot between points. This
  is the editable source of truth — *not* a symbolic guess.
- **Domain:** honest **`[a, b]`** — the interval the user actually drew. **No
  auto-extension to ±∞.** (This reverses the earlier "auto-complete the edges"
  idea; the function is defined on `[a, b]`, full stop.)
- **Input:** **hard-block**. The pen cannot move backward in x, and cannot form
  a corner sharper than a spike threshold. The curve is always a valid
  differentiable function *by construction*.
- **Drawing in pieces:** lift the pen, pan the plane, resume from the end. The
  resume must join **C¹** (new segment's starting slope snaps to the previous
  segment's ending slope).
- **Editing before submit:** drag knot points and drag tangent handles to
  reshape slopes; translate the whole curve around the plane.
- **LaTeX output:** honest piecewise cases block, shown **collapsed/simplified
  by default** ("N-segment spline — expand"), fully expandable.
- **"Prettier function":** a real roadmap pillar (Phase 7), attempting a compact
  closed form. Preferred by the user *when it's trustworthy* — so it's **error-
  gated** and always falls back to the exact piecewise output. Built
  **FOSS-only — no Symbolica** (see Phase 7).
- **Stack:** Tauri v2 · headless Rust core · web frontend (Canvas 2D + KaTeX) ·
  versioned JSON document format.

---

## Phase 0 — Scaffolding & green CI

**Goal:** an empty-but-alive Tauri v2 app that opens a window, with CI passing
on the first commit.

- [x] Tauri v2 project: headless Rust core crate (`crates/…`) + web frontend
  (`src/`) + Tauri glue (`src-tauri/`). The core has zero UI dependencies.
- [x] `just` (or `make`) task runner exposing `just test`, `just lint`, `just fmt`.
- [x] CI on every push/PR: `cargo fmt --check`, `cargo clippy -D warnings`,
  `cargo test`, `cargo audit`; `prettier --check`, `eslint --max-warnings 0`,
  `vitest run`, `pnpm audit`. Red build blocks merge. All commits must pass
  the full suite.
- [x] `CHANGELOG.md` (Keep a Changelog), `config/*.env.example`, `.gitignore` for
  real env/secrets, empty `docs/` wired up.

**Done when:** the window opens on all three platforms and CI is green.

---

## Phase 1 — Curve engine (Rust core, headless) · *highest priority*

**Goal:** given sample points, produce the canonical spline and evaluate it —
entirely in Rust, entirely tested, no UI.

- [x] **Data model:** ordered `Knot { x, y, tangent }`, x **strictly increasing**
  (enforced at the constructor — reject any insert that violates it), plus the
  derived per-segment cubic coefficients and `domain = [x_first, x_last]`.
- [x] **Fit:** compute Fritsch–Carlson tangents from the knots → shape-preserving
  C¹ Hermite spline, no overshoot. A user-set tangent overrides the computed one
  at that knot (this is what powers the drag-the-slope feature later).
- [x] **Resample noisy input:** turn a few hundred non-uniform mouse samples into a
  clean, minimal set of knots (e.g. arc-length / curvature-based thinning), so
  the spline is faithful but not overfit to jitter.
- [x] **Evaluate** `f(x)` on `[a, b]`; define behavior at/just-outside the boundary
  (clamp to domain, no extrapolation).
- [x] **Hard-block validators (pure functions, reused by the UI later):** "is this
  next sample's x strictly greater?", "does this slope exceed the spike
  threshold?", "does this edit keep x strictly increasing?"
- [x] **C¹ join on resume:** appending a segment snaps its starting tangent to the
  previous segment's ending tangent.

**Done when:** property tests confirm C¹ continuity across every join, strictly
increasing x is impossible to violate, and eval round-trips known curves within
tolerance. Pick the spline crate (e.g. `peroxide`) and wrap it behind our own
interface so the rest of the core doesn't depend on the implementation.

---

## Phase 2 — Drawing canvas & hard-block input

**Goal:** draw a valid function on a Cartesian plane with the mouse; pieces
allowed.

- [ ] Cartesian plane on **HTML5 Canvas 2D** (grid, axes, labels, zoom/pan). Mind
  the **Linux / WebKitGTK canvas-performance gotcha** — budget for it here,
  not in QA.
- [x] Pointer capture with **hard-block** wired to the Phase 1 validators: the line
  physically will not go backward in x, and will not form a spike (the pen
  "hits a wall").
- [x] **Lift & resume:** unclick, scroll/pan right/up/down, grab the end of the
  line, keep drawing — new stroke joins C¹.
- [x] On stroke end, send samples to the Rust core (Tauri command) → receive the
  fitted spline → render it. **Fitting/validation live in Rust; rendering lives
  in the frontend.**
- [ ] High-DPI correctness; consistent pointer handling across Win/macOS/Linux.

**Done when:** a user can draw a multi-piece curve that is guaranteed valid, at
a smooth frame rate on all three platforms.

---

## Phase 3 — Editing: points, tangents, translate

**Goal:** reshape the curve before committing.

- [ ] **Draggable knot points** → re-fit locally (Hermite locality means one drag
  touches only neighboring segments), still hard-blocked (can't drag a point
  past its neighbors' x, can't create a spike).
- [ ] **Draggable tangent handles** → override a knot's slope; this is the direct,
  intuitive "move the slope" interaction the representation was chosen for.
- [ ] **Translate the whole curve** around the plane (offset all knots; re-derive
  `[a, b]`).
- [ ] Undo/redo for edits.

**Done when:** every edit keeps the curve a valid C¹ function, and dragging feels
immediate.

---

## Phase 4 — "Done" → LaTeX output

**Goal:** press Done, get the exact function as pretty math.

- [ ] Render the piecewise spline as **LaTeX via KaTeX** (offline-bundled).
- [ ] **Collapsed/simplified by default** ("23-segment spline over [a, b] — expand"),
  with full per-segment cases on expand. Design the collapse so a hundred-segment
  curve is still legible.
- [ ] The LaTeX is derived deterministically from the engine, so the same file
  always yields the same output.

**Done when:** any drawn curve renders as correct, readable LaTeX, collapsed and
expanded.

---

## Phase 5 — Calculus: differentiate & integrate

**Goal:** one button turns the curve into its derivative or integral.

- [ ] **Differentiate:** each cubic segment → its quadratic derivative. The result
  is a continuous piecewise-quadratic. (Honest note: the derivative of a C¹
  spline is C⁰ — continuous but with corners at knots — so a differentiated
  curve may not itself be C¹-smooth. Surface this rather than hide it.)
- [ ] **Integrate:** each cubic segment → its quartic antiderivative, accumulating
  the definite integral up to each knot as the per-segment constant so the
  integral is **continuous across joins** (and C², nicely smooth).
- [ ] The result is itself a displayable curve with its own LaTeX; chaining
  (differentiate the integral, etc.) works.
- [ ] All calculus is analytic and lives in the Rust core, fully unit-tested against
  known closed forms.

**Done when:** derivative and integral are exact per segment, continuous where
they must be, and re-render as LaTeX.

---

## Phase 6 — File format: save / import / export

**Goal:** save a curve and reopen it as a fully editable curve, on any OS.

- [ ] **Versioned JSON** with an explicit `schema_version` from v1. Store the
  **source of truth**: ordered knots (x, y, tangents), domain `[a, b]`,
  metadata — *not* a rendered image — so files reopen editable and re-derive
  LaTeX and calculus deterministically.
- [ ] Save / import / export; a distinct file extension wrapping JSON.
- [ ] **Round-trip tests:** save → load → byte-for-byte equivalent curve. Old
  `schema_version` files still open (define the forward/backward-compat
  strategy).
- [ ] Optional: embed a cached SVG/PNG preview for thumbnails.

**Done when:** files created on one platform open identically on the others, and
a v1 file is guaranteed to open in every later version.

---

## Phase 7 — "Prettier function" symbolic layer · *highest risk, ships last*

**Goal:** when possible, offer a compact closed-form function that approximates
the drawing — beside, never instead of, the exact output.

Sequenced last on purpose: it's the least reliable part, and by now the app is
fully functional and honest without it. Attack it cheapest-and-most-reliable
first:

1. **Least-squares against a fixed basis dictionary** (polynomials, sin/cos, exp,
   log) with sparsity — cheap, deterministic; nails "this is basically x²" or
   "…basically a sine."
2. **Chebyshev / Fourier series** → readable closed form *when it collapses to a
   few terms*.
3. **Padé / rational** approximation for pole-shaped curves.
4. **Only then**, an opt-in "try harder" mode via heavier symbolic regression
   (e.g. a PySR sidecar) — possibly slow, clearly optional.

- [ ] **Always compute and display fit error** (max and RMS over `[a, b]`). Only
  *offer* the pretty form when error is below a threshold; otherwise stay silent
  and keep the piecewise output. Never present an approximation as exact.
- [ ] **FOSS-only — Symbolica is banned.** This is a public, redistributed repo, so
  the symbolic layer uses only permissively-licensed tools: pure-Rust numerical
  fitting (`nalgebra` / `ndarray`) for the steps above, and — only if
  named-function recognition is added — an open-source CAS (`cas-rs`) or a
  SymPy (BSD) / PySR (Apache-2.0) Python sidecar. Symbolica is source-available
  (not open-source), paid beyond hobbyist use, and its redistribution needs
  written permission — disqualifying here. Wrap any symbolic library behind the
  core's own interface so the engine never depends on which one is behind it.

**Done when:** for curves that genuinely are simple functions, the app proposes a
clean form with an honest error readout; for arbitrary squiggles it degrades
gracefully to the exact piecewise output with no false promises.

---

## Phase 8 — Packaging, signing, release

**Goal:** shippable, signed installers on a version tag.

- [ ] Tauri bundles: Windows `.msi`/`.exe`, macOS `.dmg`/`.app` (notarized), Linux
  `.deb`/`.AppImage`. Build once per architecture; reuse the artifact.
- [ ] `bin/deploy` builds signed bundles and attaches them to the GitHub Release;
  the release body is the current `CHANGELOG.md` section. Tag-triggered, never
  by hand.
- [ ] Signing certs / notarization creds via gitignored config; never committed.

**Done when:** pushing `vX.Y.Z` produces signed, installable builds for all three
platforms.

---

## Cross-cutting (applies to every phase)

- **Linux / WebKitGTK canvas performance** is a known weak spot — profile it
  early (Phase 2), not at the end.
- **High-DPI and cross-platform pointer events** — verify on real Win/macOS/Linux,
  not just one dev machine.
- **The Rust core stays headless and 100% unit-testable** without Tauri —
  fit, eval, calculus, and serialization never import the UI.
- **Honest caveats, by design:** "exactly what you drew" holds *as long as what
  you drew was a function* — the hard-block is what makes that true. The pretty-
  function layer is best-effort and error-gated. Keep both truths visible to the
  user; don't oversell.

---

## Suggested milestones

- **v0.1 — "It draws and prints."** Phases 0–4: draw a valid function, edit it,
  get exact LaTeX. This alone is a usable, honest product.
- **v0.2 — "It does calculus and saves."** Phases 5–6.
- **v0.3 — "It guesses pretty."** Phase 7, gated and optional.
- **v1.0 — "It ships."** Phase 8 hardened; docs complete.