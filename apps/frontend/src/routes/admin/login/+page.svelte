<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import IconTwitch from "~icons/lucide/twitch";
	import {
		completeLogin,
		getSession,
		guardAdmin,
		GuardStatus,
		startLogin,
	} from "#lib/admin/session";

	let busy = $state(true);
	let error = $state("");

	async function decideWhereToGo(): Promise<"panel" | "home" | null> {
		const guard = await guardAdmin();
		if (guard.status === GuardStatus.Admin) return "panel";
		if (guard.status === GuardStatus.NotAdmin) return "home";
		return null;
	}

	async function go(where: "panel" | "home" | null) {
		if (where === "panel") await goto("/admin/panel", { replaceState: true });
		else if (where === "home") await goto("/", { replaceState: true });
	}

	async function handleTwitchLogin() {
		busy = true;
		error = "";
		try {
			const authUrl = await startLogin();
			location.assign(authUrl);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			busy = false;
		}
	}

	onMount(async () => {
		const params = new URLSearchParams(window.location.search);
		const oauthError = params.get("error");
		const oauthErrorDescription = params.get("error_description");
		if (oauthError) {
			error = oauthErrorDescription
				? `${oauthError}: ${oauthErrorDescription}`
				: oauthError;
			busy = false;
			return;
		}
		const code = params.get("code");
		const state = params.get("state");
		try {
			if (code && state) {
				await completeLogin(code, state);
				const where = await decideWhereToGo();
				if (!where) throw new Error("Нет доступа к панели.");
				await go(where);
			} else if (await getSession()) {
				const where = await decideWhereToGo();
				if (where) await go(where);
			}
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			busy = false;
		}
	});
</script>

<svelte:head>
	<title>Sapa TV | Админ-вход</title>
</svelte:head>

<main class="login">
	<div class="card">
		<h1 class="card__title">Sapa TV</h1>
		<p class="card__subtitle">Вход в админ-панель</p>

		<button
			class="btn btn--twitch"
			type="button"
			onclick={handleTwitchLogin}
			disabled={busy}
		>
			<IconTwitch aria-hidden="true" />
			{busy ? "Ожидание..." : "Войти через Twitch"}
		</button>

		{#if error}
			<p class="card__error" role="alert">{error}</p>
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

	.login {
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

	.card__error {
		margin: 16px 0 0;
		padding: 10px 12px;
		border-radius: 10px;
		background: color-mix(in oklch, var(--error) 12%, transparent);
		color: var(--error);
		font-size: 12px;
		line-height: 1.4;
		text-align: left;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		width: 100%;
		padding: 12px 16px;
		border-radius: 12px;
		border: 1px solid transparent;
		font-size: 14px;
		font-weight: 700;
		font-family: inherit;
		cursor: pointer;
		transition:
			transform 0.15s ease-out,
			filter 0.15s ease-out;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn--twitch {
		background: var(--twitch-brand, #9146ff);
		color: var(--twitch-ink, #fff);
	}

	.btn--twitch:hover:not(:disabled) {
		filter: brightness(1.08);
	}

	.btn--twitch:active:not(:disabled) {
		transform: scale(0.985);
	}
</style>
