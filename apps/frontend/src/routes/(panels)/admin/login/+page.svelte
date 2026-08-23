<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { AuthCard, Button } from "@sapa-tv-ru/ui-kit";
	import IconTwitch from "~icons/lucide/twitch";
	import {
		completeLogin,
		getSession,
		guardAdmin,
		GuardStatus,
		startLogin,
	} from "#lib/admin/session";
	import { ApiError, ApiErrorKind } from "#lib/internal/api-error";

	let busy = $state(true);
	let error = $state("");

	function loginErrorText(err: ApiError): string {
		switch (err.kind) {
			case ApiErrorKind.BadRequest:
				return "Ошибка запуска авторизации: отсутствует twitch config.";
			default:
				return "Не удалось завершить вход через Twitch. Попробуй ещё раз.";
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

	async function go(where: "panel" | "home" | null) {
		if (where === "panel")
			await goto(resolve("admin/panel"), { replaceState: true });
		else if (where === "home") await goto(resolve(""), { replaceState: true });
	}

	async function handleTwitchLogin() {
		busy = true;
		error = "";
		const res = await startLogin();
		if (res.isErr()) {
			error = loginErrorText(res.error);
			busy = false;
			return;
		}
		location.assign(res.value);
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
			const res = await completeLogin(code, state);
			if (res.isErr()) {
				error = loginErrorText(res.error);
				busy = false;
				return;
			}
			const where = await decideWhereToGo();
			if (!where) {
				error = "Не удалось определить, куда перенаправить.";
				busy = false;
				return;
			}
			await go(where);
		} else if (await getSession()) {
			const where = await decideWhereToGo();
			if (where) await go(where);
		}
		busy = false;
	});
</script>

<svelte:head>
	<title>Sapa TV | Админ-вход</title>
</svelte:head>

<main class="login">
	<AuthCard title="Sapa TV" subtitle="Вход в админ-панель" {error}>
		<Button variant="twitch" onclick={handleTwitchLogin} disabled={busy}>
			<IconTwitch aria-hidden="true" />
			{busy ? "Ожидание..." : "Войти через Twitch"}
		</Button>
	</AuthCard>
</main>

<style>
	.login {
		min-height: calc(100vh - 48px);
		display: flex;
		align-items: center;
		justify-content: center;
	}
</style>
