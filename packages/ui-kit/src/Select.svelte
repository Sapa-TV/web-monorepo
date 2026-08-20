<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		value?: string | number | null;
		id?: string;
		name?: string;
		required?: boolean;
		title?: string;
		children?: Snippet;
	}

	let { value = $bindable(""), children, ...rest }: Props = $props();
</script>

<select {...rest} bind:value class="field-input">
	{@render children?.()}
</select>

<style>
	.field-input {
		padding: 8px 12px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container);
		color: var(--on-surface);
		font: inherit;
		font-size: 13px;
		outline: none;
	}

	.field-input:focus {
		border-color: var(--primary);
	}
</style>
