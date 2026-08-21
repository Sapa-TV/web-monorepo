import js from "@eslint/js";
import prettier from "eslint-config-prettier";
import { defineConfig } from "eslint/config";

export default defineConfig(js.configs.recommended, prettier, {
	languageOptions: {
		ecmaVersion: 2023,
		sourceType: "module",
	},
	rules: {
		"no-console": ["warn", { allow: ["warn", "error"] }],
	},
});
