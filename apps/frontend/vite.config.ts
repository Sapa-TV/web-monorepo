import { defineConfig } from "vitest/config";
import { playwright } from "@vitest/browser-playwright";
import adapter from "@sveltejs/adapter-static";
import { sveltekit } from "@sveltejs/kit/vite";
import Icons from "unplugin-icons/vite";

const backendTarget = process.env.VITE_BACKEND_URL ?? "http://localhost:3000";

function normalizeBase(input: string): string {
	if (!input) return "";
	let base = input;
	if (!base.startsWith("/")) base = `/${base}`;
	base = base.replace(/\/+$/, "");
	return base === "/" ? "" : base;
}

export default defineConfig({
	server: {
		proxy: {
			"/api": { target: backendTarget, changeOrigin: true },
			"/wapi": { target: backendTarget, changeOrigin: true, ws: true },
		},
	},
	plugins: [
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes("node_modules") ? undefined : true,
				experimental: { async: true },
			},
			adapter: adapter(),
			paths: {
				base: normalizeBase(process.env.VITE_BASE_PATH ?? ""),
				relative: false,
			},
			experimental: { remoteFunctions: true },
		}),
		Icons({
			compiler: "svelte",
		}),
	],
	test: {
		expect: { requireAssertions: true },
		projects: [
			{
				extends: "./vite.config.ts",
				test: {
					name: "client",
					browser: {
						enabled: true,
						provider: playwright(),
						instances: [{ browser: "chromium", headless: true }],
					},
					include: ["src/**/*.svelte.{test,spec}.{js,ts}"],
					exclude: ["src/lib/server/**"],
				},
			},

			{
				extends: "./vite.config.ts",
				test: {
					name: "server",
					environment: "node",
					include: ["src/**/*.{test,spec}.{js,ts}"],
					exclude: ["src/**/*.svelte.{test,spec}.{js,ts}"],
				},
			},
		],
	},
});
