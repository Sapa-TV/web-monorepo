<script lang="ts">
	import type { Snippet } from "svelte";
	import SiteNav from "#lib/components/SiteNav.svelte";
	import "../../styles/site.css";

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();
</script>

<svelte:head>
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link
		rel="preconnect"
		href="https://fonts.gstatic.com"
		crossorigin="anonymous"
	/>
	<link
		href="https://fonts.googleapis.com/css2?family=Archivo:wght@400;500;600;700;800;900&amp;family=IBM+Plex+Mono:wght@400;500;600&amp;display=swap"
		rel="stylesheet"
	/>
</svelte:head>

<div class="paper">
	<SiteNav />
	{@render children()}
</div>
