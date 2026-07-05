import js from "@eslint/js";
import tseslint from "typescript-eslint";

// Flat config. ESLint handles correctness; Prettier owns formatting, so we keep
// stylistic rules out of here.
export default tseslint.config(
  { ignores: ["dist/", "node_modules/", "src-tauri/", "target/"] },
  {
    files: ["**/*.ts"],
    extends: [js.configs.recommended, ...tseslint.configs.recommended],
  },
);
