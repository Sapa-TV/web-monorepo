<script lang="ts">
	import type { Snippet } from "svelte";

	type Tone =
		| "ok"
		| "missing"
		| "bad"
		| "pending"
		| "spinning"
		| "completed"
		| "error"
		| "cancelled"
		| "root"
		| "connected"
		| "disconnected";

	interface Props {
		tone?: Tone;
		dot?: boolean;
		children?: Snippet;
	}

	let { tone, dot = false, children }: Props = $props();
</script>

{#if dot}
	<span class={`badge badge--dot badge--${tone}`} aria-hidden="true"></span>
{:else}
	<span class={`badge badge--${tone}`}>{@render children?.()}</span>
{/if}

<style>
	.badge {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px;
		border-radius: 6px;
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		white-space: nowrap;
	}

	.badge--ok,
	.badge--completed,
	.badge--connected {
		background: color-mix(in oklch, var(--secondary) 16%, transparent);
		color: var(--secondary);
	}

	.badge--missing,
	.badge--pending {
		background: color-mix(in oklch, var(--tertiary) 16%, transparent);
		color: var(--tertiary);
	}

	.badge--bad,
	.badge--error,
	.badge--disconnected {
		background: color-mix(in oklch, var(--error) 16%, transparent);
		color: var(--error);
	}

	.badge--spinning,
	.badge--root {
		background: color-mix(in oklch, var(--primary) 16%, transparent);
		color: var(--primary);
	}

	.badge--cancelled {
		background: color-mix(in oklch, var(--on-surface-variant) 14%, transparent);
		color: var(--on-surface-variant);
	}

	.badge--dot {
		width: 8px;
		height: 8px;
		padding: 0;
		border-radius: 50%;
		flex-shrink: 0;
	}
</style>
