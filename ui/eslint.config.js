import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist/", "node_modules/"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    rules: {
      "no-console": ["error", { allow: ["warn", "error"] }],
    },
  },
  {
    files: ["src/**/*.tsx", "src/**/*.ts"],
    ignores: ["src/ipc/**", "src/main.tsx"],
    rules: {
      // R1 no front: `invoke` só em src/ipc/ (a porta única do backend).
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["@tauri-apps/api*", "@tauri-apps/*"],
              message:
                "R1: componente não fala com o backend direto — use ui/src/ipc/",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["tests/**"],
    rules: { "no-console": "off" },
  },
);
