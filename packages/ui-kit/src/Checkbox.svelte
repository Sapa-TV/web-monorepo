<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		checked?: boolean;
		id?: string;
		name?: string;
		disabled?: boolean;
		children?: Snippet;
	}

	let { checked = $bindable(false), children, ...rest }: Props = $props();
</script>

<label class="check">
	<input type="checkbox" bind:checked {...rest} />
	<span>{@render children?.()}</span>
</label>

<style>
	.check {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		color: var(--on-surface);
		cursor: pointer;
	}

	.check input {
		flex-shrink: 0;
	}
</style>
