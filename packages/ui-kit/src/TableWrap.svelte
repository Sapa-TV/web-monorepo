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
</style>
