import { includeIgnoreFile } from "@eslint/compat";
import js from "@eslint/js";
import prettier from "eslint-config-prettier";
import svelte from "eslint-plugin-svelte";
import { defineConfig } from "eslint/config";
import globals from "globals";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript-eslint";
import {
	colorLiterals,
	propsInlineType,
} from "@sapa-tv-ru/custom-eslint-rules";

const gitignorePath = path.resolve(
	fileURLToPath(new URL(".", import.meta.url)),
	".gitignore",
);

export default defineConfig(
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
			"no-console": ["warn", { allow: ["warn", "error"] }],
		},
	},
	{ files: ["**/*.svelte"], ...colorLiterals },
	{ files: ["src/**/*.svelte"], ...propsInlineType },
	{
		files: ["**/*.svelte"],
		languageOptions: {
			parserOptions: {
				projectService: true,
				tsconfigRootDir: import.meta.dirname,
				extraFileExtensions: [".svelte"],
				parser: ts.parser,
			},
		},
		rules: {
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
);
