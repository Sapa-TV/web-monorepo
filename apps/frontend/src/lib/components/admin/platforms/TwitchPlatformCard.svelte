<script lang="ts">
	import { api } from "#lib/api";
	import { Alert, Badge, Button, Card, Section } from "@sapa-tv-ru/ui-kit";
	import { onDestroy, onMount } from "svelte";
	import IconTwitch from "~icons/lucide/twitch";
	import { describeApiError } from "#lib/api-error-text";
	import {
		ApiError,
		ApiErrorKind,
		normalizeApiError,
	} from "#lib/internal/api-error";

	let loaded = $state(false);
	let configured = $state(false);
	let error = $state("");
	let hint = $state("");
	let authorizeBusy = $state(false);

	let pollTimer: ReturnType<typeof setInterval> | null = null;
	const POLL_MS = 3000;
	const POLL_MAX_TRIES = 100;

	function setError(err: unknown) {
		error = describeApiError(err);
	}

	async function fetchConfigured(): Promise<boolean | ApiError> {
		const res = await api.getIngressCredentials();
		if (res.isErr()) return normalizeApiError(res.error);
		return res.value.configured;
	}

	async function load(): Promise<void> {
		const result = await fetchConfigured();
		if (result instanceof ApiError) {
			setError(result);
			return;
		}
		configured = result;
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
		pollTimer = setInterval(() => {
			void (async () => {
				tries += 1;
				const result = await fetchConfigured();
				if (!(result instanceof ApiError)) {
					error = "";
					configured = result;
					if (configured) {
						clearPoll();
						hint = "Twitch credentials авторизованы.";
					} else if (tries >= POLL_MAX_TRIES) {
						clearPoll();
						error =
							"Таймаут авторизации: подтверди доступ в окне Twitch и нажми «Авторизовать» ещё раз.";
					}
					return;
				}
				if (
					result.kind === ApiErrorKind.Unauthorized ||
					result.kind === ApiErrorKind.Forbidden
				) {
					clearPoll();
					error = "Сессия истекла: перелогинься и попробуй снова.";
				} else if (tries >= POLL_MAX_TRIES) {
					clearPoll();
					error = "Не удалось получить статус авторизации. Попробуй ещё раз.";
				}
			})();
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
		const res = await api.startTwitchAuth();
		if (res.isErr()) {
			win?.close();
			setError(res.error);
		} else if (win) {
			win.location.assign(res.value.auth_url);
			hint = "Авторизуйся во всплывающем окне — статус обновится сам.";
			startPoll();
		} else {
			hint =
				"Всплывающее окно заблокировано: разреши попапы для этого сайта и нажми «Авторизовать» ещё раз.";
		}
		authorizeBusy = false;
	}

	async function revoke() {
		if (
			!confirm(
				"Отозвать Twitch credentials? Интеграция с Twitch перестанет работать.",
			)
		)
			return;
		error = "";
		const res = await api.revokeIngressCredentials();
		if (res.isErr()) {
			setError(res.error);
			return;
		}
		await load();
		hint = "Credentials отозваны.";
	}

	onMount(() => {
		void load().finally(() => (loaded = true));
	});

	onDestroy(clearPoll);
</script>

<Card>
	<Section
		title="Twitch"
		hint="Учётка, от имени которой бекенд ходит в Twitch (стрим-статус, чтение чата)."
	>
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
