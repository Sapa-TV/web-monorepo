<script lang="ts">
	import type { Snippet } from "svelte";
	import SiteNav from "#lib/components/SiteNav.svelte";
	import "../../styles/site.css";

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();
</script>

<div class="paper">
	<SiteNav />
	{@render children()}
</div>

<style>
	.paper {
		position: relative;
		isolation: isolate;
	}

	.paper::before {
		content: "";
		position: fixed;
		inset: 0;
		z-index: -1;
		pointer-events: none;
		opacity: 0.5;
		background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='140' height='140'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='2' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)' opacity='0.05'/%3E%3C/svg%3E");
		mix-blend-mode: multiply;
	}

	:global(:root[data-theme="dark"]) .paper::before {
		mix-blend-mode: overlay;
	}
</style>
