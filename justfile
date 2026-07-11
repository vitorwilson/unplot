# unplot task runner. Run `just` with no arguments to list recipes.
# The same recipes are what CI runs, so "green locally" means "green in CI".

# List available recipes.
default:
    @just --list

# Run the desktop app in development (Vite dev server + Tauri window).
dev:
    cargo tauri dev

# Build production bundles (frontend + Tauri, per platform).
build:
    cargo tauri build

# Compile the web frontend to dist/ (embedded by the Tauri shell at build time).
web:
    pnpm build

# Run the full test suite: Rust core + frontend + packaging scripts.
test: web test-scripts
    cargo test
    pnpm test

# Unit-test the packaging helper scripts (bin/*.test.sh).
test-scripts:
    bash bin/unbundle-appimage-gl.test.sh
    bash bin/render-packaging.test.sh

# Lint everything: clippy (warnings as errors) + eslint.
lint: web
    cargo clippy --all-targets -- -D warnings
    pnpm lint

# Format all code in place (Rust + frontend).
fmt:
    cargo fmt
    pnpm format

# Verify formatting without writing — this is the CI check.
fmt-check:
    cargo fmt --check
    pnpm format:check

# Security / dependency audit (Rust + frontend).
audit:
    cargo audit
    pnpm audit --audit-level high

# Cut a release: validate, tag vX.Y.Z, and push (triggers the signed-build
# pipeline).
release:
    bin/deploy
