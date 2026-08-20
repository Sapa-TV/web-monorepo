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

	let busy = $state(true);
	let error = $state("");

	async function decideWhereToGo(): Promise<"panel" | "home" | null> {
		const guard = await guardAdmin();
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
