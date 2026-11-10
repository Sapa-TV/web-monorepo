import { asset } from "$app/paths";
import type { RequestHandler } from "./$types";

export const prerender = true;

export const GET: RequestHandler = () => {
	const sw = asset("sw.js");
	const scope = sw.slice(0, sw.lastIndexOf("/") + 1);
	const manifest = {
		id: scope,
		name: "Sapa TV",
		short_name: "Sapa TV",
		description: "Стримы, игры и фильмы Sapa TV",
		lang: "ru",
		dir: "ltr",
		start_url: scope,
		scope,
		display: "standalone",
		display_override: ["standalone", "minimal-ui", "browser"],
		orientation: "portrait-primary",
		background_color: "#121212",
		theme_color: "#121212",
		icons: [
			{
				src: asset("icons/icon-192.png"),
				sizes: "192x192",
				type: "image/png",
				purpose: "any",
			},
			{
				src: asset("icons/icon-512.png"),
				sizes: "512x512",
				type: "image/png",
				purpose: "any",
			},
			{
				src: asset("icons/icon-maskable-512.png"),
				sizes: "512x512",
				type: "image/png",
				purpose: "maskable",
			},
		],
	};

	return new Response(JSON.stringify(manifest), {
		headers: {
			"content-type": "application/manifest+json",
			"cache-control": "public, max-age=3600",
		},
	});
};
