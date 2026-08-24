<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		title: string;
		hint?: string;
		children?: Snippet;
	}

	let { title, hint, children }: Props = $props();
</script>

<section class="section">
	<h2 class="section-title">{title}</h2>
	{#if hint}
		<p class="section-hint">{hint}</p>
	{/if}
	{@render children?.()}
</section>

<style>
	.section {
		margin-bottom: 24px;
	}

	.section-title {
		font-size: 12px;
		font-weight: 600;
		color: var(--on-surface-variant);
		margin: 0 0 10px;
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.section-hint {
		margin: 0 0 12px;
		color: var(--on-surface-variant);
		font-size: 12px;
		line-height: 1.5;
	}
</style>
