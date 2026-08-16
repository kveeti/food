import { defineConfig } from "oxlint";
import eslintPluginTailwindcss from "eslint-plugin-tailwindcss";

const tailwindRecommended = eslintPluginTailwindcss.configs.recommended;
if (Array.isArray(tailwindRecommended)) {
  throw new Error("Expected one Tailwind recommended config");
}

export default defineConfig({
  plugins: ["typescript", "unicorn", "oxc"],
  jsPlugins: ["eslint-plugin-tailwindcss"],
  settings: {
    tailwindcss: {
      cssConfigPath: "./src/app.css",
    },
  },
  categories: {
    correctness: "error",
  },
  rules: tailwindRecommended.rules,
  env: {
    builtin: true,
  },
});
