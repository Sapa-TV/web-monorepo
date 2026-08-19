<script lang="ts">
	import { onMount } from "svelte";
	import { completeCredsAuth } from "#lib/admin/creds";

	let busy = $state(true);
	let status = $state("");
	let error = $state("");

	async function closeIfPopup() {
		if (window.opener) window.close();
	}

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const oauthError = params.get("error");
		const oauthErrorDescription = params.get("error_description");
		const code = params.get("code");
		const state = params.get("state");

		async function run() {
			if (oauthError) {
				status = "Отказано в доступе.";
				error = oauthErrorDescription
					? `${oauthError}: ${oauthErrorDescription}`
					: oauthError;
				return;
			}
			if (!code || !state) {
				status = "Окно можно закрыть.";
				return;
			}
			status = "Авторизация...";
			try {
				await completeCredsAuth(code, state);
				status = "Twitch credentials авторизованы.";
				await closeIfPopup();
			} catch (err) {
				error = err instanceof Error ? err.message : String(err);
			}
		}

		run().finally(() => (busy = false));
	});
</script>

<svelte:head>
	<title>Sapa TV | Авторизация Twitch</title>
</svelte:head>

<main class="creds">
	<div class="card">
		<h1 class="card__title">Sapa TV</h1>
		<p class="card__subtitle">Twitch credentials</p>

		{#if error}
			<p class="card__status card__status--error" role="alert">{error}</p>
		{:else if status}
			<p class="card__status">{status}</p>
		{/if}

		{#if busy}
			<p class="card__loading">Ожидание...</p>
		{/if}
	</div>
</main>

<style>
	:global(html) {
		background: var(--background);
	}

	:global(body) {
		margin: 0;
		font-family: "Inter", system-ui, sans-serif;
		background: var(--background);
		color: var(--on-surface);
		font-size: 14px;
	}

	.creds {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
	}

	.card {
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

	.card__title {
		margin: 0;
		font-family: "Archivo", sans-serif;
		font-size: 1.6rem;
		font-weight: 900;
		letter-spacing: -0.03em;
		color: var(--on-background);
	}

	.card__subtitle {
		margin: 6px 0 24px;
		color: var(--on-surface-variant);
		font-size: 13px;
	}

	.card__status {
		margin: 0;
		font-size: 13px;
		line-height: 1.5;
		color: var(--on-surface);
	}

	.card__status--error {
		padding: 10px 12px;
		border-radius: 10px;
		background: color-mix(in oklch, var(--error) 12%, transparent);
		color: var(--error);
		text-align: left;
	}

	.card__loading {
		margin: 16px 0 0;
		color: var(--on-surface-variant);
		font-size: 12px;
	}
</style>
