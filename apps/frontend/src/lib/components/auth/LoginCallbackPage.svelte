<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { AuthCard, Button } from "@sapa-tv-ru/ui-kit";
	import IconTwitch from "~icons/lucide/twitch";
	import { guardAdmin, GuardStatus } from "#lib/admin/session";
	import {
		confirmMerge,
		finishProvider,
		providerLabel,
		startProvider,
		type Provider,
	} from "#lib/auth/provider";
	import { ApiError, ApiErrorKind } from "#lib/internal/api-error";

	interface Props {
		provider?: Provider;
		/** Platforms with a working site-login flow. */
		loginSupported?: boolean;
	}

	let { provider = "twitch", loginSupported = true }: Props = $props();

	let busy = $state(true);
	let error = $state("");
	let pendingTicket = $state<string | null>(null);

	function loginErrorText(err: ApiError): string {
		switch (err.kind) {
			case ApiErrorKind.BadRequest:
				return "Ссылка входа устарела. Начните вход заново.";
			default:
				return "Не удалось войти. Попробуйте ещё раз.";
		}
	}

	async function decideWhereToGo(): Promise<"panel" | "home" | null> {
		const guardRes = await guardAdmin();
		if (guardRes.isErr()) {
			error = loginErrorText(guardRes.error);
			return null;
		}
		const guard = guardRes.value;
		if (guard.status === GuardStatus.Admin) return "panel";
		if (guard.status === GuardStatus.NotAdmin) return "home";
		return null;
	}

	async function go(where: "panel" | "home" | "account") {
		if (where === "panel") {
			await goto(resolve("admin/panel"), { replaceState: true });
			return;
		}
		if (where === "account") {
			await goto(resolve("account"), { replaceState: true });
			return;
		}
		await goto(resolve(""), { replaceState: true });
	}

	async function afterLogin() {
		const returnToAccount = sessionStorage.getItem("auth_return") === "account";
		sessionStorage.removeItem("auth_return");
		if (provider === "vk" || returnToAccount) {
			await go("account");
			return;
		}
		const where = await decideWhereToGo();
		if (!where) {
			error = "Не удалось определить, куда перейти.";
			busy = false;
			return;
		}
		await go(where);
	}

	async function handleLogin() {
		busy = true;
		error = "";
		const res = await startProvider(provider);
		if (res.isErr()) {
			error = loginErrorText(res.error);
			busy = false;
			return;
		}
		location.assign(res.value);
	}

	async function handleMerge() {
		if (!pendingTicket) return;
		busy = true;
		error = "";
		const res = await confirmMerge(pendingTicket);
		if (res.isErr()) {
			error = loginErrorText(res.error);
			busy = false;
			return;
		}
		pendingTicket = null;
		await afterLogin();
	}

	async function handleCancelMerge() {
		pendingTicket = null;
		await go("account");
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
		if (code && state) {
			const res = await finishProvider(provider, code, state);
			if (res.isErr()) {
				error = loginErrorText(res.error);
				busy = false;
				return;
			}
			if (res.value.kind === "merge") {
				pendingTicket = res.value.ticket;
				busy = false;
				return;
			}
			await afterLogin();
			return;
		}
		busy = false;
	});
</script>

<svelte:head>
	<title>Sapa TV | Вход</title>
</svelte:head>

<main class="login">
	<AuthCard title="Sapa TV" subtitle="Вход в аккаунт" {error}>
		{#if pendingTicket}
			<p class="login__status">
				Этот аккаунт {providerLabel(provider)} уже привязан к другому профилю. Объединить
				его с текущим? Заявки и подписки переедут в текущий аккаунт.
			</p>
			<div class="login__actions">
				<Button variant="primary" onclick={handleMerge} disabled={busy}>
					Объединить
				</Button>
				<Button variant="cancel" onclick={handleCancelMerge} disabled={busy}>
					Отмена
				</Button>
			</div>
		{:else if !loginSupported}
			<p class="login__status">Вход через этот сервис пока недоступен.</p>
		{:else}
			<Button
				variant={provider === "twitch" ? "twitch" : "default"}
				onclick={handleLogin}
				disabled={busy}
			>
				{#if provider === "twitch"}
					<IconTwitch aria-hidden="true" />
				{:else}
					<span class="login__vk">VK</span>
				{/if}
				{busy ? "Подождите..." : `Войти через ${providerLabel(provider)}`}
			</Button>
		{/if}
	</AuthCard>
</main>

<style>
	.login {
		min-height: calc(100vh - 48px);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.login__status {
		margin: 0 0 16px;
		font-size: 13px;
		line-height: 1.5;
		color: var(--on-surface);
	}

	.login__actions {
		display: flex;
		gap: 8px;
		justify-content: center;
	}

	.login__vk {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		border-radius: 4px;
		background: var(--vk-brand);
		color: var(--vk-ink);
		font-size: 9px;
		font-weight: 800;
	}
</style>
