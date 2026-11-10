<script lang="ts">
	import type { Snippet } from "svelte";
	import { asset, resolve } from "$app/paths";
	import "../styles/theme.css";

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();
</script>

<svelte:head>
	<link rel="manifest" href={resolve("/manifest.webmanifest")} />
	<link rel="apple-touch-icon" href={asset("icons/apple-touch-icon.png")} />
	<meta name="apple-mobile-web-app-capable" content="yes" />
	<meta
		name="apple-mobile-web-app-status-bar-style"
		content="black-translucent"
	/>
	<meta name="apple-mobile-web-app-title" content="Sapa TV" />
</svelte:head>

{@render children()}
