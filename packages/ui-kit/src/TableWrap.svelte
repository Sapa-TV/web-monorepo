<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		children?: Snippet;
	}

	let { children }: Props = $props();
</script>

<div class="table-wrap">
	{@render children?.()}
</div>

<style>
	.table-wrap {
		overflow: hidden;
		border-radius: 12px;
		border: 1px solid var(--outline-variant);
	}

	.table-wrap :global(table) {
		width: 100%;
		border-collapse: collapse;
		background: var(--surface-container);
	}

	.table-wrap :global(th) {
		text-align: left;
		padding: 10px 14px;
		font-size: 11px;
		color: var(--on-surface-variant);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		border-bottom: 1px solid var(--outline-variant);
		background: var(--surface-container-high);
	}

	.table-wrap :global(td) {
		padding: 10px 14px;
		border-bottom: 1px solid var(--outline-variant);
	}

	.table-wrap :global(tr:last-child td) {
		border-bottom: none;
	}

	.table-wrap :global(.mono) {
		font-family: var(--font-mono);
		font-size: 12px;
	}

	.table-wrap :global(.actions-cell) {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		width: 1%;
		white-space: nowrap;
	}

	.table-wrap :global(.actions-cell) :global(.btn) {
		margin-left: 4px;
	}
</style>
