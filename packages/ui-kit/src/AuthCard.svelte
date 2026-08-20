<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		title: string;
		subtitle: string;
		error?: string;
		children?: Snippet;
	}

	let { title, subtitle, error = "", children }: Props = $props();
</script>

<div class="auth-card">
	<h1 class="auth-card__title">{title}</h1>
	<p class="auth-card__subtitle">{subtitle}</p>

	{@render children?.()}

	{#if error}
		<p class="auth-card__error" role="alert">{error}</p>
	{/if}
</div>

<style>
	.auth-card {
		width: 100%;
		max-width: 360px;
		background: var(--surface-container);
		border: 1px solid var(--outline-variant);
		border-radius: 16px;
		padding: 32px 28px;
		text-align: center;
		box-shadow:
			0 1px 0 var(--surface-bright) inset,
			0 18px 40px -18px color-mix(in oklch, var(--on-surface) 45%, transparent);
	}

	.auth-card__title {
		margin: 0;
		font-family: "Archivo", sans-serif;
		font-size: 1.6rem;
		font-weight: 900;
		letter-spacing: -0.03em;
		color: var(--on-background);
	}

	.auth-card__subtitle {
		margin: 6px 0 24px;
		color: var(--on-surface-variant);
		font-size: 13px;
	}

	.auth-card__error {
		margin: 16px 0 0;
		padding: 10px 12px;
		border-radius: 10px;
		background: color-mix(in oklch, var(--error) 12%, transparent);
		color: var(--error);
		font-size: 12px;
		line-height: 1.4;
		text-align: left;
	}
</style>
