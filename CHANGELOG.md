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
