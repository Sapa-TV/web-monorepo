<script lang="ts">
	import { api } from "#lib/api";
	import { HttpError } from "@sapa-tv-ru/api-client";
	import { Alert, Badge, Button, Card, Section } from "@sapa-tv-ru/ui-kit";
	import { onDestroy, onMount } from "svelte";
	import IconTwitch from "~icons/lucide/twitch";

	let loaded = $state(false);
	let configured = $state(false);
	let error = $state("");
	let hint = $state("");
	let authorizeBusy = $state(false);

	let pollTimer: ReturnType<typeof setInterval> | null = null;
	const POLL_MS = 3000;
	const POLL_MAX_TRIES = 100;
	const UNAUTHORIZED = 401;
	const FORBIDDEN = 403;

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	async function load() {
		const res = await api.getIngressCredentials();
		if (res.isErr()) throw res.error;
		configured = res.value.configured;
	}

	function clearPoll() {
		if (pollTimer) {
			clearInterval(pollTimer);
			pollTimer = null;
		}
	}

	function startPoll() {
		clearPoll();
		let tries = 0;
		pollTimer = setInterval(async () => {
			tries += 1;
			try {
				await load();
				if (configured) {
					clearPoll();
					hint = "Twitch credentials авторизованы.";
				} else if (tries >= POLL_MAX_TRIES) {
					clearPoll();
					error =
						"Таймаут авторизации: подтверди доступ в окне Twitch и нажми «Авторизовать» ещё раз.";
				}
			} catch (err) {
				const http = err instanceof HttpError ? err : null;
				if (
					http &&
					(http.status === UNAUTHORIZED || http.status === FORBIDDEN)
				) {
					clearPoll();
					error = "Сессия истекла: перелогинься и попробуй снова.";
				} else if (tries >= POLL_MAX_TRIES) {
					clearPoll();
					error = "Не удалось получить статус авторизации. Попробуй ещё раз.";
				}
			}
		}, POLL_MS);
	}

	async function authorize() {
		authorizeBusy = true;
		error = "";
		const win = window.open(
			"",
			"sapa_twitch_auth",
			"popup,width=560,height=720",
		);
		try {
			const res = await api.startTwitchAuth();
			if (res.isErr()) throw res.error;
			if (win) {
				win.location.assign(res.value.auth_url);
				hint = "Авторизуйся во всплывающем окне — статус обновится сам.";
				startPoll();
			} else {
				hint =
					"Всплывающее окно заблокировано: разреши попапы для этого сайта и нажми «Авторизовать» ещё раз.";
			}
		} catch (err) {
			win?.close();
			setError(err);
		} finally {
			authorizeBusy = false;
		}
	}

	async function revoke() {
		if (
			!confirm(
				"Отозвать Twitch credentials? Интеграция с Twitch перестанет работать.",
			)
		)
			return;
		error = "";
		try {
			const res = await api.revokeIngressCredentials();
			if (res.isErr()) throw res.error;
			await load();
			hint = "Credentials отозваны.";
		} catch (err) {
			setError(err);
		}
	}

	onMount(() => {
		load()
			.catch(setError)
			.finally(() => (loaded = true));
	});

	onDestroy(clearPoll);
</script>

<Card>
	<Section title="Twitch">
		<p class="section-hint">
			Учётка, от имени которой бекенд ходит в Twitch (стрим-статус, чтение
			чата).
		</p>

		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		{#if loaded}
			<div class="twitch-row">
				<Badge tone={configured ? "ok" : "missing"}>
					{configured ? "авторизовано" : "не авторизовано"}
				</Badge>
				<Button
					variant="primary"
					type="button"
					onclick={authorize}
					disabled={authorizeBusy}
				>
					<IconTwitch aria-hidden="true" />
					{authorizeBusy ? "Открытие..." : "Авторизовать"}
				</Button>
				{#if configured}
					<Button size="sm" type="button" onclick={revoke}>Отозвать</Button>
				{/if}
			</div>
		{:else}
			<p class="loading">Загрузка...</p>
		{/if}
	</Section>
</Card>

<style>
	.twitch-row {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}
</style>
