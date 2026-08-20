<script lang="ts">
	import { onMount } from "svelte";
	import {
		WAPI_BASE,
		WS_URL,
		apiFetch,
		type QueueEntry,
		type QueueStats,
	} from "#lib/api";
	import { Badge, Button, Input, Section, TableWrap } from "@sapa-tv-ru/ui-kit";
	import IconKeyRound from "~icons/lucide/key-round";
	import IconRefreshCw from "~icons/lucide/refresh-cw";
	import IconPlay from "~icons/lucide/play";
	import IconPlus from "~icons/lucide/plus";
	import IconList from "~icons/lucide/list";
	import IconX from "~icons/lucide/x";
	import IconCheck from "~icons/lucide/check";

	const widgetAccessKey =
		typeof window !== "undefined"
			? (new URLSearchParams(window.location.search).get("widget_access_key") ??
				"")
			: "";

	const LOG_LIMIT = 50;
	const UNAUTHORIZED = 401;
	const WS_RETRY_BASE_MS = 1_000;
	const WS_RETRY_MAX_MS = 15_000;
	const REFRESH_INTERVAL_MS = 10_000;

	type BadgeTone =
		| "ok"
		| "missing"
		| "bad"
		| "pending"
		| "spinning"
		| "completed"
		| "error"
		| "cancelled"
		| "root"
		| "connected"
		| "disconnected";

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

	async function loadAll() {
		try {
			const [listRes, statsRes] = await Promise.all([
				apiFetch(`${WAPI_BASE}/queue`, {}, widgetAccessKey),
				apiFetch(`${WAPI_BASE}/queue/stats`, {}, widgetAccessKey),
			]);

			if (listRes.status === UNAUTHORIZED) {
				setKeyState(widgetAccessKey ? "bad" : "missing");
			} else {
				setKeyState("ok");
			}

			if (listRes.ok) {
				const data = (await listRes.json()) as {
					entries: QueueEntry[];
					next_cursor: number | null;
				};
				entries = data.entries;
			}

			const hasSpinning = entries.some((e) => e.status === "Spinning");
			if (!hasSpinning) {
				const next = entries.find(
					(e) => e.status === "Error" || e.status === "Pending",
				);
				nextUser = next ? next.user_name || "" : "";
			}

			if (statsRes.ok) {
				const s = (await statsRes.json()) as QueueStats;
				dequeueLabel = `▶ Dequeue (${s.pending + s.error})`;
			}
		} catch {
			// ignore
		}
	}

	async function dequeueNext() {
		nextBusy = true;
		try {
			const r = await apiFetch(
				`${WAPI_BASE}/queue/next`,
				{ method: "POST" },
				widgetAccessKey,
			);
			const data = (await r.json()) as {
				slot?: { name: string };
				entry?: { id: number; user_name: string };
			};
			if (r.ok) {
				addEvent(
					`🎰 ${data.slot?.name} → #${data.entry?.id} (${data.entry?.user_name})`,
					"start",
				);
			} else {
				addEvent(`❌ Dequeue: ${r.status}`, "error");
			}
			await loadAll();
		} catch (err) {
			addEvent(`❌ ${err instanceof Error ? err.message : err}`, "error");
		} finally {
			nextBusy = false;
		}
	}

	async function completeEntry(id: number) {
		try {
			const r = await apiFetch(
				`${WAPI_BASE}/queue/${id}/complete`,
				{ method: "POST" },
				widgetAccessKey,
			);
			if (r.ok) {
				addEvent(`✔ #${id} завершён`, "complete");
			} else {
				addEvent(`❌ Complete #${id}: ${r.status}`, "error");
			}
			await loadAll();
		} catch (err) {
			addEvent(`❌ ${err instanceof Error ? err.message : err}`, "error");
		}
	}

	async function cancelEntry(id: number) {
		try {
			const r = await apiFetch(
				`${WAPI_BASE}/queue/${id}/cancel`,
				{ method: "POST" },
				widgetAccessKey,
			);
			if (r.ok) {
				addEvent(`✕ #${id} отменён`, "error");
			} else {
				addEvent(`❌ Cancel #${id}: ${r.status}`, "error");
			}
			await loadAll();
		} catch (err) {
			addEvent(`❌ ${err instanceof Error ? err.message : err}`, "error");
		}
	}

	async function enqueueEntry() {
		const name = enqName.trim();
		if (!name) return;
		enqBusy = true;
		try {
			const r = await apiFetch(
				`${WAPI_BASE}/queue/anonymous`,
				{
					method: "POST",
					headers: { "Content-Type": "application/json" },
					body: JSON.stringify({ name }),
				},
				widgetAccessKey,
			);
			if (r.ok) {
				enqName = "";
				addEvent(`➕ ${name} добавлен`, "complete");
			} else {
				addEvent(`❌ Ошибка ${r.status}`, "error");
			}
			await loadAll();
		} catch (err) {
			addEvent(`❌ ${err instanceof Error ? err.message : err}`, "error");
		} finally {
			enqBusy = false;
		}
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
				ws?.send(JSON.stringify({ type: "auth", token: widgetAccessKey }));
		};
		ws.onclose = () => {
			connState = "disconnected";
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
					| "spin_error";
				entry_id?: number;
				user_name?: string;
				slot_name?: string;
			};
			switch (d.type) {
				case "auth_ok":
					wsRetryMs = WS_RETRY_BASE_MS;
					break;
				case "auth_err":
					wsRejected = true;
					setKeyState(widgetAccessKey ? "bad" : "missing");
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
	<Section title="Добавить в очередь">
		<div class="inline-form">
			<label class="visually-hidden" for="enq-name">Имя зрителя</label>
			<Input
				id="enq-name"
				type="text"
				placeholder="Имя зрителя"
				bind:value={enqName}
			/>
			<Button
				variant="primary"
				type="button"
				onclick={enqueueEntry}
				disabled={enqBusy}
			>
				<IconPlus aria-hidden="true" />
				Добавить
			</Button>
		</div>
	</Section>
{/if}

<Section title="Активные">
	<TableWrap>
		<table>
			<thead>
				<tr>
					<th>Имя</th>
					<th>Статус</th>
					<th>Слот</th>
					<th class="th-actions"></th>
				</tr>
			</thead>
			<tbody>
				{#if active.length === 0}
					<tr>
						<td colspan="4" class="empty-row">Нет записей</td>
					</tr>
				{:else}
					{#each active as e (e.id)}
						<tr>
							<td>{e.user_name || e.user_id}</td>
							<td
								><Badge tone={e.status.toLowerCase() as BadgeTone}
									>{e.status}</Badge
								></td
							>
							<td
								>{e.status === "Spinning"
									? e.slot_name || e.result_slot_id || "—"
									: "—"}</td
							>
							<td class="actions-cell">
								{#if e.status === "Pending" || e.status === "Error"}
									<Button
										variant="cancel"
										size="sm"
										icon
										type="button"
										onclick={() => cancelEntry(e.id)}
										aria-label="Отменить"
									>
										<IconX aria-hidden="true" />
									</Button>
								{:else if e.status === "Spinning"}
									<Button
										variant="complete"
										size="sm"
										icon
										type="button"
										onclick={() => completeEntry(e.id)}
										aria-label="Завершить"
									>
										<IconCheck aria-hidden="true" />
									</Button>
								{/if}
							</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</TableWrap>
</Section>

<Section title="Завершённые / Отменённые">
	<TableWrap>
		<table>
			<thead>
				<tr>
					<th>Имя</th>
					<th>Результат</th>
					<th>Статус</th>
				</tr>
			</thead>
			<tbody>
				{#if done.length === 0}
					<tr>
						<td colspan="3" class="empty-row">Нет записей</td>
					</tr>
				{:else}
					{#each [...done].reverse() as e (e.id)}
						<tr>
							<td>{e.user_name || e.user_id}</td>
							<td
								>{e.status === "Completed" ? e.slot_name || "✔" : "отменён"}</td
							>
							<td>
								<Badge tone={e.status.toLowerCase() as BadgeTone}
									>{e.status}</Badge
								></td
							>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</TableWrap>
</Section>

{#if showLog}
	<Section title="События">
		<div class="event-log" role="log">
			{#if events.length === 0}
				<div class="empty-row">Ожидание событий...</div>
			{:else}
				{#each events as e (e)}
					<div class={`ev ev-${e.cls}`}>
						<span class="ev-time">[{e.time}]</span>
						{e.text}
					</div>
				{/each}
			{/if}
		</div>
	</Section>
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

	.icon-sm {
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

	.inline-form {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		align-items: center;
	}

	.inline-form :global(.field-input) {
		min-width: 140px;
	}

	.empty-row {
		color: var(--on-surface-variant);
		font-style: italic;
		padding: 14px;
		text-align: center;
	}

	.th-actions {
		width: 1%;
	}

	.event-log {
		background: var(--surface-container);
		border: 1px solid var(--outline-variant);
		border-radius: 12px;
		padding: 14px;
		max-height: 200px;
		overflow-y: auto;
		font-family: "IBM Plex Mono", ui-monospace, monospace;
		font-size: 12px;
	}

	.event-log .ev {
		padding: 4px 0;
		border-bottom: 1px solid var(--outline-variant);
	}

	.event-log .ev:last-child {
		border-bottom: none;
	}

	.ev-time {
		color: var(--on-surface-variant);
		margin-right: 8px;
	}

	.ev-start {
		color: var(--primary);
	}

	.ev-complete {
		color: var(--secondary);
	}

	.ev-error {
		color: var(--error);
	}
</style>
