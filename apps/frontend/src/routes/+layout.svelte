<script lang="ts">
	import type { Snippet } from "svelte";
	import "../styles/theme.css";

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();
</script>

{@render children()}
