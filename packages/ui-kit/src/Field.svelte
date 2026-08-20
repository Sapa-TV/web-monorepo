<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		label: string;
		children?: Snippet;
	}

	let { label, children }: Props = $props();
</script>

<label class="field">
	<span class="field__label">{label}</span>
	{@render children?.()}
</label>

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-size: 12px;
		color: var(--on-surface-variant);
	}

	.field__label {
		font-family: inherit;
	}
</style>
