<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		tone?: "error" | "success";
		children?: Snippet;
	}

	let { tone = "error", children, ...rest }: Props = $props();
</script>

<p
	class={`alert alert--${tone}`}
	role={tone === "error" ? "alert" : undefined}
	{...rest}
>
	{@render children?.()}
</p>

<style>
	.alert {
		max-width: 720px;
		margin: 0 0 12px;
		padding: 10px 14px;
		border-radius: 10px;
		font-size: 12px;
		line-height: 1.4;
	}

	.alert--error {
		background: color-mix(in oklch, var(--error) 12%, transparent);
		color: var(--error);
	}

	.alert--success {
		background: color-mix(in oklch, var(--secondary) 14%, transparent);
		color: var(--secondary);
	}
</style>
