<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		wide?: boolean;
		children?: Snippet;
	}

	let { wide = false, children }: Props = $props();
</script>

<section class="card" class:card--wide={wide}>
	{@render children?.()}
</section>

<style>
	.card {
		background: var(--surface-container);
		border: 1px solid var(--outline-variant);
		border-radius: 12px;
		padding: 18px;
		margin-bottom: 20px;
		max-width: 720px;
	}

	.card--wide {
		max-width: none;
	}
</style>
