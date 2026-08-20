import { includeIgnoreFile } from "@eslint/compat";
import js from "@eslint/js";
import vitest from "@vitest/eslint-plugin";
import prettier from "eslint-config-prettier";
import playwright from "eslint-plugin-playwright";
import svelte from "eslint-plugin-svelte";
import { defineConfig } from "eslint/config";
import globals from "globals";
import path from "node:path";
import ts from "typescript-eslint";

const gitignorePath = path.resolve(import.meta.dirname, ".gitignore");

export default defineConfig(
	{
		ignores: ["html/**"],
	},
	includeIgnoreFile(gitignorePath),
	js.configs.recommended,
	...ts.configs.recommended,
	...svelte.configs.recommended,
	prettier,
	...svelte.configs.prettier,
	{
		languageOptions: { globals: { ...globals.browser, ...globals.node } },
		rules: {
			"no-undef": "off",
			"no-unused-vars": [
				"error",
				{ argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
			],
			"no-console": ["warn", { allow: ["warn", "error"] }],
			"svelte/no-navigation-without-resolve": "off",
			"@typescript-eslint/no-magic-numbers": [
				"error",
				{
					enforceConst: true,
					ignoreDefaultValues: true,
					ignore: [0, 1, 2],
				},
			],
			"@typescript-eslint/no-unused-vars": [
				"error",
				{
					argsIgnorePattern: "^_",
					varsIgnorePattern: "^_",
					caughtErrorsIgnorePattern: "^_",
				},
			],
		},
	},
	{
		files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
		languageOptions: {
			parserOptions: {
				projectService: true,
				tsconfigRootDir: import.meta.dirname,
				extraFileExtensions: [".svelte"],
				parser: ts.parser,
			},
		},
	},
	{
		files: ["**/constants.ts", "**/constants/*.ts"],
		rules: {
			"no-magic-numbers": "off",
			"@typescript-eslint/no-magic-numbers": "off",
		},
	},
	{
		files: ["scripts/**"],
		rules: {
			"no-magic-numbers": "off",
			"@typescript-eslint/no-magic-numbers": "off",
			"no-console": "off",
		},
	},
	{
		files: ["src/**/*.test.ts", "src/**/*.spec.ts"],
		plugins: { vitest },
		rules: {
			...vitest.configs.recommended.rules,

			"vitest/no-focused-tests": "error",
			"vitest/no-disabled-tests": "warn",
			"vitest/expect-expect": "error",
			"vitest/no-conditional-expect": "error",
			"vitest/require-hook": "warn",
			"vitest/consistent-test-it": [
				"error",
				{ fn: "it", withinDescribe: "it" },
			],
			"vitest/require-top-level-describe": "error",
			"vitest/no-identical-title": "error",

			"no-magic-numbers": "off",
			"@typescript-eslint/no-magic-numbers": "off",
		},
	},
	{
		files: ["tests/**/*.spec.ts"],
		...playwright.configs["flat/recommended"],
		rules: {
			"playwright/require-hook": "warn",
		},
	},
);
