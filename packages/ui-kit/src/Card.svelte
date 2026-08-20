<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		children?: Snippet;
	}

	let { children }: Props = $props();
</script>

<section class="card">
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
</style>
