# unplot

**Draw a function. Get the math.**

Desmos plots a formula you type. This does the reverse: you *draw* a smooth
curve `y = f(x)` on a Cartesian plane, and the app hands back the exact
function — as clean LaTeX you can differentiate, integrate, edit, and save.

What you get back is exactly what you drew: a shape-preserving spline that is a
valid function by construction, not a guess. An optional layer will *also*
suggest a compact closed form when your drawing really is a simple function —
always shown with its error, never presented as exact.

## Install

Download the installer for your platform from the
[latest release](https://github.com/vitorwilson/unplot/releases/latest):

- **Windows** — `.msi` or `.exe`
- **macOS** — `.dmg` (universal: Apple Silicon + Intel)
- **Linux** — `.AppImage`, `.deb`, or `.rpm`

The builds aren't code-signed yet, so your OS will warn about an unidentified
developer the first time you open the app. To get past it:

- **macOS** — right-click the app and choose **Open**, then **Open** again (only
  needed once). If it still refuses, clear the quarantine flag:
  `xattr -dr com.apple.quarantine /Applications/unplot.app`.
- **Windows** — on the SmartScreen prompt, click **More info → Run anyway**.

## Run from source

Needs Rust, Node, and [pnpm](https://pnpm.io); [`just`](https://github.com/casey/just)
runs the tasks.

```sh
just dev     # run the desktop app
just test    # run the full test suite (Rust core + frontend)
```

## Docs

- Roadmap and locked design decisions: [`docs/PLAN.md`](docs/PLAN.md)
