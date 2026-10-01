import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";

/**
 * Regra local: texto de usuário fora do catálogo central.
 * Bloqueia JSXText com letras e literais em title/placeholder/aria-label/
 * label/alt — o texto de usuário nasce em src/i18n/messages.ts (§8 do doc de UI).
 * Expressões (identificadores, templates com interpolação, chamadas MSG.*)
 * passam; strings técnicas (value=, className=, style=) não são cobertas.
 */
const noUserLiterals = {
  rules: {
    "no-user-literals": {
      meta: {
        type: "problem",
        docs: {
          description:
            "Texto de usuário deve vir do catálogo MSG (src/i18n/messages.ts, §8 do UI_REFERENCE)",
        },
        schema: [],
        messages: {
          userLiteral:
            "Texto de usuário literal fora do catálogo — mova para MSG (src/i18n/messages.ts)",
        },
      },
      create(context) {
        const hasLetter = (s) => /[a-zA-ZÀ-ÿ]/.test(s ?? "");
        return {
          JSXText(node) {
            if (hasLetter(node.value.trim())) {
              context.report({ node, messageId: "userLiteral" });
            }
          },
          "JSXExpressionContainer > Literal"(node) {
            if (typeof node.value === "string" && hasLetter(node.value)) {
              context.report({ node, messageId: "userLiteral" });
            }
          },
          "JSXAttribute[name.name=/^(title|placeholder|aria-label|label|alt)$/] > Literal"(node) {
            if (typeof node.value === "string" && hasLetter(node.value)) {
              context.report({ node, messageId: "userLiteral" });
            }
          },
          "JSXAttribute[name.name=/^(title|placeholder|aria-label|label|alt)$/] > TemplateLiteral"(node) {
            if (node.expressions.length === 0 && node.quasis.some((q) => hasLetter(q.value.raw))) {
              context.report({ node, messageId: "userLiteral" });
            }
          },
          // Ternários com ramo literal nos MESMOS pontos de texto de usuário:
          // children JSX (`{cond ? "ok?" : "✕"}`) e atributos de texto
          // (`aria-label={on ? "a" : "b"}`) — ternários técnicos (className,
          // data-*, role, state) não são texto e ficam de fora.
          "JSXElement > JSXExpressionContainer > ConditionalExpression,\n          JSXAttribute[name.name=/^(title|placeholder|aria-label|label|alt)$/] > JSXExpressionContainer > ConditionalExpression"(node) {
            for (const branch of [node.consequent, node.alternate]) {
              if (branch.type === "Literal" && typeof branch.value === "string" && hasLetter(branch.value)) {
                context.report({ node: branch, messageId: "userLiteral" });
              }
            }
          },
        };
      },
    },
  },
};

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
    // Script de smoke do shell Tauri (CI): roda em Node puro e fala com o
    // operador via stdout — console.log é o log do job, não ruído de app.
    files: ["e2e/tauri.smoke.mjs"],
    languageOptions: { globals: { ...globals.node } },
    rules: { "no-console": "off" },
  },
  {
    // A casca não escreve texto de usuário inline — só via MSG.
    plugins: { local: noUserLiterals },
    files: ["src/components/**/*.tsx", "src/App.tsx"],
    rules: { "local/no-user-literals": "error" },
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
