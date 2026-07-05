/// <reference types="vitest/config" />
import { defineConfig } from "vite";

// Tauri conventions: fixed dev port and no screen-clearing so Tauri's logs stay
// visible. Vite emits to dist/ at the repo root; the Tauri shell embeds that
// build (see src-tauri/tauri.conf.json -> build.frontendDist).
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
