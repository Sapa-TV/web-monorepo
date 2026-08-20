<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		title: string;
		children?: Snippet;
	}

	let { title, children }: Props = $props();
</script>

<section class="section">
	<h2 class="section-title">{title}</h2>
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
</style>
