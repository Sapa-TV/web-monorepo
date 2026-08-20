<script lang="ts">
	import { onMount } from "svelte";
	import { AuthCard } from "@sapa-tv-ru/ui-kit";
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
	<AuthCard title="Sapa TV" subtitle="Twitch credentials" {error}>
		{#if error}
			<p class="creds__status creds__status--error" role="alert">{error}</p>
		{:else if status}
			<p class="creds__status">{status}</p>
		{/if}

		{#if busy}
			<p class="creds__loading">Ожидание...</p>
		{/if}
	</AuthCard>
</main>

<style>
	.creds {
		min-height: calc(100vh - 48px);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.creds__status {
		margin: 0;
		font-size: 13px;
		line-height: 1.5;
		color: var(--on-surface);
	}

	.creds__status--error {
		padding: 10px 12px;
		border-radius: 10px;
		background: color-mix(in oklch, var(--error) 12%, transparent);
		color: var(--error);
		text-align: left;
	}

	.creds__loading {
		margin: 16px 0 0;
		color: var(--on-surface-variant);
		font-size: 12px;
	}
</style>
