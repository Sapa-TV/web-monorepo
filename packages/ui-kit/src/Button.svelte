<script lang="ts">
	import type { Snippet } from "svelte";

	type Variant =
		| "default"
		| "primary"
		| "danger"
		| "complete"
		| "cancel"
		| "twitch"
		| "brand";

	interface Props {
		variant?: Variant;
		size?: "sm" | "md";
		icon?: boolean;
		href?: string;
		target?: string;
		rel?: string;
		type?: "button" | "submit" | "reset";
		disabled?: boolean;
		title?: string;
		"aria-label"?: string;
		onclick?: (event: MouseEvent) => void;
		children?: Snippet;
	}

	let {
		variant = "default",
		size = "md",
		icon = false,
		href,
		target,
		rel,
		type,
		disabled,
		title,
		onclick,
		children,
		...rest
	}: Props = $props();

	const classes = $derived(
		"btn" +
			(variant !== "default" ? ` btn--${variant}` : "") +
			(size === "sm" ? " btn--sm" : "") +
			(icon ? " btn--icon" : ""),
	);
</script>

{#if href}
	<a {href} {target} {rel} class={classes} {...rest}>
		{@render children?.()}
	</a>
{:else}
	<button {type} {disabled} {title} {onclick} class={classes} {...rest}>
		{@render children?.()}
	</button>
{/if}

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 8px 14px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container);
		font: inherit;
		font-size: 13px;
		font-weight: 600;
		line-height: 1;
		color: var(--on-surface);
		cursor: pointer;
		text-decoration: none;
		transition:
			background 0.15s,
			border-color 0.15s,
			filter 0.15s,
			transform 0.16s ease-out;
	}

	.btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.btn--primary {
		background: var(--primary);
		border-color: transparent;
		color: var(--on-primary);
	}

	.btn--primary:hover:not(:disabled) {
		background: var(--primary-dim);
	}

	.btn--sm {
		padding: 6px 12px;
	}

	.btn--sm:hover:not(:disabled) {
		border-color: var(--outline);
		background: var(--surface-container-high);
	}

	.btn--icon {
		padding: 6px 8px;
	}

	.btn--danger:hover:not(:disabled) {
		border-color: var(--error);
		color: var(--error);
	}

	.btn--complete {
		background: var(--secondary);
		border-color: transparent;
		color: var(--on-secondary);
	}

	.btn--complete:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	.btn--cancel {
		color: var(--on-surface-variant);
	}

	.btn--cancel:hover:not(:disabled) {
		border-color: var(--error);
		color: var(--error);
	}

	.btn--twitch {
		background: var(--twitch-brand, #9146ff);
		border-color: transparent;
		color: var(--twitch-ink, #fff);
	}

	.btn--twitch:hover:not(:disabled) {
		filter: brightness(1.08);
	}

	.btn--brand {
		justify-content: space-between;
		background: var(--brand);
		color: var(--brand-ink);
		border-radius: 0.9rem;
		padding: 0.85rem 2rem;
		box-shadow:
			0 8px 20px -8px color-mix(in oklch, var(--brand) 60%, transparent),
			inset 0 0 0 1px color-mix(in oklch, var(--brand-ink) 18%, transparent);
	}

	.btn--brand:hover {
		transform: translateY(-2px);
		filter: brightness(1.06);
	}

	.btn--brand:active {
		transform: translateY(0) scale(0.985);
	}

	@media (min-width: 640px) {
		.btn--brand {
			padding: 1rem 2rem;
		}
	}
</style>
