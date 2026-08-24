<script lang="ts">
	import { WS_URL, wapi, type QueueEntry } from "#lib/api";
	import EnqueueForm from "#lib/components/dock/EnqueueForm.svelte";
	import EventLog from "#lib/components/dock/EventLog.svelte";
	import QueueTables from "#lib/components/dock/QueueTables.svelte";
	import { Badge, Button } from "@sapa-tv-ru/ui-kit";
	import { onMount } from "svelte";
	import IconKeyRound from "~icons/lucide/key-round";
	import IconList from "~icons/lucide/list";
	import IconPlay from "~icons/lucide/play";
	import IconPlus from "~icons/lucide/plus";
	import IconRefreshCw from "~icons/lucide/refresh-cw";
	import { describeApiError } from "#lib/api-error-text";
	import { ApiErrorKind, normalizeApiError } from "#lib/internal/api-error";

	const widgetAccessKey =
		typeof window !== "undefined"
			? (new URLSearchParams(window.location.search).get("widget_access_key") ??
				"")
			: "";

	const LOG_LIMIT = 50;
	const WS_RETRY_BASE_MS = 1_000;
	const WS_RETRY_MAX_MS = 15_000;
	const REFRESH_INTERVAL_MS = 10_000;

	const wapiAuth = {
		headers: { Authorization: `Bearer ${widgetAccessKey}` },
	};

	let entries = $state<QueueEntry[]>([]);
	let nextUser = $state("");
	let dequeueLabel = $state("▶ Dequeue");
	let showEnqueue = $state(false);
	let showLog = $state(false);
	let enqName = $state("");
	let enqBusy = $state(false);
	let nextBusy = $state(false);
	let keyState = $state<"ok" | "missing" | "bad" | null>(null);
	let connState = $state<"connected" | "disconnected">("disconnected");
	let widgetOnline = $state(false);
	let events = $state<
		{ time: string; text: string; cls: "start" | "complete" | "error" }[]
	>([]);

	const active = $derived(
		entries.filter(
			(e) =>
				e.status === "Pending" ||
				e.status === "Spinning" ||
				e.status === "Error",
		),
	);
	const done = $derived(
		entries.filter((e) => e.status === "Completed" || e.status === "Cancelled"),
	);

	function setKeyState(state: "ok" | "missing" | "bad") {
		keyState = state;
	}

	function addEvent(text: string, cls: "start" | "complete" | "error") {
		events = [
			{ time: new Date().toLocaleTimeString(), text, cls },
			...events,
		].slice(0, LOG_LIMIT);
	}

	function isUnauthorized(err: unknown): boolean {
		return normalizeApiError(err).kind === ApiErrorKind.Unauthorized;
	}

	function describeError(err: unknown): string {
		return describeApiError(err);
	}

	async function loadAll() {
		const [listRes, statsRes] = await Promise.all([
			wapi.list(undefined, wapiAuth),
			wapi.stats(wapiAuth),
		]);

		listRes.match(
			(data) => {
				setKeyState("ok");
				entries = data.entries;
				const hasSpinning = entries.some((e) => e.status === "Spinning");
				if (!hasSpinning) {
					const next = entries.find(
						(e) => e.status === "Error" || e.status === "Pending",
					);
					nextUser = next ? next.user_name || "" : "";
				}
			},
			(err) => {
				if (isUnauthorized(err))
					setKeyState(widgetAccessKey ? "bad" : "missing");
			},
		);

		statsRes.match(
			(s) => {
				dequeueLabel = `▶ Dequeue (${s.pending + s.error})`;
			},
			() => {},
		);
	}

	async function dequeueNext() {
		nextBusy = true;
		const res = await wapi.dequeueNext(wapiAuth);
		res.match(
			(data) =>
				addEvent(
					`🎰 ${data.slot?.name} → #${data.entry.id} (${data.entry.user_name})`,
					"start",
				),
			(err) => addEvent(`❌ Dequeue: ${describeError(err)}`, "error"),
		);
		await loadAll();
		nextBusy = false;
	}

	async function completeEntry(id: number) {
		const res = await wapi.complete(id, wapiAuth);
		res.match(
			() => addEvent(`✔ #${id} завершён`, "complete"),
			(err) => addEvent(`❌ Complete #${id}: ${describeError(err)}`, "error"),
		);
		await loadAll();
	}

	async function cancelEntry(id: number) {
		const res = await wapi.cancel(id, wapiAuth);
		res.match(
			() => addEvent(`✕ #${id} отменён`, "error"),
			(err) => addEvent(`❌ Cancel #${id}: ${describeError(err)}`, "error"),
		);
		await loadAll();
	}

	async function enqueueEntry() {
		const name = enqName.trim();
		if (!name) return;
		enqBusy = true;
		const res = await wapi.enqueueAnonymous({ name }, wapiAuth);
		res.match(
			() => {
				enqName = "";
				addEvent(`➕ ${name} добавлен`, "complete");
			},
			(err) => addEvent(`❌ Ошибка ${describeError(err)}`, "error"),
		);
		await loadAll();
		enqBusy = false;
	}

	let ws: WebSocket | null = null;
	let wsRejected = false;
	let wsRetryMs = WS_RETRY_BASE_MS;

	function connectWs() {
		if (wsRejected) return;
		ws = new WebSocket(WS_URL);
		ws.onopen = () => {
			connState = "connected";
			if (widgetAccessKey)
				ws?.send(
					JSON.stringify({
						type: "auth",
						token: widgetAccessKey,
						role: "dock",
					}),
				);
		};
		ws.onclose = () => {
			connState = "disconnected";
			widgetOnline = false;
			if (!wsRejected) {
				setTimeout(connectWs, wsRetryMs);
				wsRetryMs = Math.min(wsRetryMs * 2, WS_RETRY_MAX_MS);
			}
		};
		ws.onerror = () => ws?.close();
		ws.onmessage = (e) => {
			const d = JSON.parse(e.data) as {
				type:
					| "auth_ok"
					| "auth_err"
					| "spin_started"
					| "spin_completed"
					| "spin_error"
					| "presence";
				entry_id?: number;
				user_name?: string;
				slot_name?: string;
				dock?: boolean;
				widget_count?: number;
			};
			switch (d.type) {
				case "auth_ok":
					wsRetryMs = WS_RETRY_BASE_MS;
					break;
				case "auth_err":
					wsRejected = true;
					setKeyState(widgetAccessKey ? "bad" : "missing");
					break;
				case "presence":
					widgetOnline = (d.widget_count ?? 0) > 0;
					break;
				case "spin_started":
					addEvent(
						`🎰 #${d.entry_id} — ${d.user_name}: ${d.slot_name}`,
						"start",
					);
					loadAll();
					break;
				case "spin_completed":
					addEvent(`✔ #${d.entry_id} завершён`, "complete");
					loadAll();
					break;
				case "spin_error":
					addEvent(`⚠ #${d.entry_id} таймаут`, "error");
					loadAll();
					break;
			}
		};
	}

	onMount(() => {
		loadAll();
		const poll = setInterval(loadAll, REFRESH_INTERVAL_MS);
		connectWs();

		return () => {
			clearInterval(poll);
			ws?.close();
		};
	});
</script>

<svelte:head>
	<title>Док-панель</title>
</svelte:head>

<header class="panel-header">
	<h1>Док-панель</h1>
	<div class="panel-header__right">
		{#if keyState}
			<Badge tone={keyState}>
				<IconKeyRound class="icon-sm" aria-hidden="true" />
				{keyState === "ok"
					? "ключ ок"
					: keyState === "missing"
						? "нет ключа"
						: "ключ неверный"}
			</Badge>
		{/if}
		<Badge tone={connState} dot></Badge>
		{#if connState === "connected"}
			<Badge tone={widgetOnline ? "connected" : "disconnected"}>
				виджет {widgetOnline ? "онлайн" : "офлайн"}
			</Badge>
		{/if}
		<Button
			size="sm"
			icon
			title="Обновить"
			aria-label="Обновить"
			onclick={loadAll}
		>
			<IconRefreshCw aria-hidden="true" />
		</Button>
	</div>
</header>

<div class="toolbar">
	<Button
		variant="primary"
		type="button"
		onclick={dequeueNext}
		disabled={nextBusy}
	>
		<IconPlay aria-hidden="true" />
		{dequeueLabel}
	</Button>
	<span class="next-user">{nextUser}</span>
	<Button
		size="sm"
		type="button"
		onclick={() => void (showEnqueue = !showEnqueue)}
	>
		<IconPlus aria-hidden="true" />
		{showEnqueue ? "Закрыть" : "Добавить"}
	</Button>
	<span class="spacer"></span>
	<Button size="sm" type="button" onclick={() => void (showLog = !showLog)}>
		<IconList aria-hidden="true" />
		{showLog ? "Скрыть лог" : "Лог"}
	</Button>
</div>

{#if showEnqueue}
	<EnqueueForm bind:value={enqName} busy={enqBusy} onsubmit={enqueueEntry} />
{/if}

<QueueTables
	{active}
	{done}
	oncomplete={completeEntry}
	oncancel={cancelEntry}
/>

{#if showLog}
	<EventLog {events} />
{/if}

<style>
	h1 {
		font-size: 22px;
		margin-bottom: 20px;
		color: var(--on-background);
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 16px;
	}

	.panel-header h1 {
		font-size: 18px;
		color: var(--on-background);
		margin: 0;
	}

	.panel-header__right {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.panel-header__right :global(.icon-sm) {
		width: 0.95rem;
		height: 0.95rem;
	}

	.toolbar {
		display: flex;
		gap: 10px;
		margin-bottom: 20px;
		flex-wrap: wrap;
		align-items: center;
	}

	.next-user {
		font-size: 12px;
		color: var(--on-surface-variant);
		min-width: 80px;
	}

	.spacer {
		flex: 1;
	}
</style>
