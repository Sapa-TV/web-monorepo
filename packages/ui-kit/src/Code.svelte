<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		block?: boolean;
		title?: string;
		children?: Snippet;
	}

	let { block = false, children, title }: Props = $props();
</script>

<code class={`code ${block ? "code--block" : ""}`} {title}>
	{@render children?.()}
</code>

<style>
	.code {
		font-family: "IBM Plex Mono", ui-monospace, monospace;
		font-size: 12px;
	}

	.code--block {
		flex: 1;
		overflow-x: auto;
		white-space: nowrap;
		padding: 10px 12px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface);
		color: var(--on-surface);
	}
</style>
