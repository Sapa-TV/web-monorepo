<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/state";
	import { AuthCard } from "@sapa-tv-ru/ui-kit";
	import { completeCredsAuth } from "#lib/admin/creds";
	import { ApiError, ApiErrorKind } from "#lib/internal/api-error";

	const PLATFORMS = {
		twitch: { label: "Twitch" },
		"vk-video-live": null,
	} as const;

	const platformParam = page.params.platform as string;
	const known = platformParam in PLATFORMS;
	const label = known
		? (PLATFORMS[platformParam as keyof typeof PLATFORMS]?.label ?? null)
		: null;

	let busy = $state(true);
	let status = $state("");
	let error = $state("");

	async function closeIfPopup() {
		if (window.opener) window.close();
	}

	function describeCredsError(err: ApiError): string {
		switch (err.kind) {
			case ApiErrorKind.Unauthorized:
			case ApiErrorKind.Forbidden:
				return "Нет доступа: сессия истекла или у аккаунта нет прав root. Вернись на панель, перелогинься и попробуй снова.";
			case ApiErrorKind.BadRequest:
				return "Не удалось завершить авторизацию: попробуй на панели «Авторизовать» ещё раз.";
			default:
				return `Не удалось сохранить credentials. Попробуй ещё раз.`;
		}
	}

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const oauthError = params.get("error");
		const oauthErrorDescription = params.get("error_description");
		const code = params.get("code");
		const state = params.get("state");

		async function run() {
			if (!known) {
				status = "";
				error = "Неизвестная платформа.";
				return;
			}
			if (!label) {
				status = "";
				error = "Авторизация этой платформы пока недоступна.";
				return;
			}
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
			const res = await completeCredsAuth(code, state);
			if (res.isErr()) {
				error = describeCredsError(res.error);
				return;
			}
			status = `${label} credentials авторизованы.`;
			await closeIfPopup();
		}

		run().finally(() => (busy = false));
	});
</script>

<svelte:head>
	<title>Sapa TV | Авторизация</title>
</svelte:head>

<main class="creds">
	<AuthCard title="Sapa TV" subtitle="Подключение платформы" {error}>
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
