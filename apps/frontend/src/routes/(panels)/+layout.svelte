<script lang="ts">
	import type { Snippet } from "svelte";
	import "../../styles/panel.css";

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();
</script>

{@render children()}

<style>
	:global(body) {
		padding: 24px;
		font-size: 14px;
		background: var(--background);
	}
</style>
