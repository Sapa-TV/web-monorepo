<script lang="ts">
	import { onMount } from "svelte";
	import {
		WAPI_BASE,
		WS_URL,
		apiFetch,
		type QueueEntry,
		type QueueStats,
	} from "#lib/api";
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
				entries = (await listRes.json()) as QueueEntry[];
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
			<span class={`key-badge ${keyState}`}>
				<IconKeyRound class="key-badge__icon" aria-hidden="true" />
				{keyState === "ok"
					? "ключ ок"
					: keyState === "missing"
						? "нет ключа"
						: "ключ неверный"}
			</span>
		{/if}
		<span class={`conn-dot ${connState}`} aria-hidden="true"></span>
		<button
			class="btn btn--sm"
			type="button"
			onclick={loadAll}
			title="Обновить"
			aria-label="Обновить"
		>
			<IconRefreshCw aria-hidden="true" />
		</button>
	</div>
</header>

<div class="toolbar">
	<button
		class="btn btn--primary"
		type="button"
		onclick={dequeueNext}
		disabled={nextBusy}
	>
		<IconPlay aria-hidden="true" />
		{dequeueLabel}
	</button>
	<span class="next-user">{nextUser}</span>
	<button
		class="btn btn--sm"
		type="button"
		onclick={() => void (showEnqueue = !showEnqueue)}
	>
		<IconPlus aria-hidden="true" />
		{showEnqueue ? "Закрыть" : "Добавить"}
	</button>
	<span class="spacer"></span>
	<button
		class="btn btn--sm"
		type="button"
		onclick={() => void (showLog = !showLog)}
	>
		<IconList aria-hidden="true" />
		{showLog ? "Скрыть лог" : "Лог"}
	</button>
</div>

{#if showEnqueue}
	<div class="section">
		<div class="inline-form">
			<label class="visually-hidden" for="enq-name">Имя зрителя</label>
			<input
				id="enq-name"
				type="text"
				placeholder="Имя зрителя"
				bind:value={enqName}
			/>
			<button
				class="btn btn--primary"
				type="button"
				onclick={enqueueEntry}
				disabled={enqBusy}
			>
				<IconPlus aria-hidden="true" />
				Добавить
			</button>
		</div>
	</div>
{/if}

<div class="section">
	<div class="section-title">Активные</div>
	<div class="table-wrap">
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
								><span class={`status-badge ${e.status.toLowerCase()}`}
									>{e.status}</span
								></td
							>
							<td
								>{e.status === "Spinning"
									? e.slot_name || e.result_slot_id || "—"
									: "—"}</td
							>
							<td class="actions-cell">
								{#if e.status === "Pending" || e.status === "Error"}
									<button
										class="btn btn--cancel"
										type="button"
										onclick={() => cancelEntry(e.id)}
										aria-label="Отменить"
									>
										<IconX aria-hidden="true" />
									</button>
								{:else if e.status === "Spinning"}
									<button
										class="btn btn--complete"
										type="button"
										onclick={() => completeEntry(e.id)}
										aria-label="Завершить"
									>
										<IconCheck aria-hidden="true" />
									</button>
								{/if}
							</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>
</div>

<div class="section">
	<div class="section-title">Завершённые / Отменённые</div>
	<div class="table-wrap">
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
								<span class={`status-badge ${e.status.toLowerCase()}`}
									>{e.status}</span
								></td
							>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>
</div>

{#if showLog}
	<div class="section">
		<div class="section-title">События</div>
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
	</div>
{/if}

<style>
	:global(html) {
		background: var(--background);
	}

	:global(body) {
		margin: 0;
		padding: 24px;
		font-family: "Inter", system-ui, sans-serif;
		background: var(--background);
		color: var(--on-surface);
		font-size: 14px;
	}

	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: -1px;
		padding: 0;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
		border: 0;
	}

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

	.conn-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.conn-dot.connected {
		background: var(--secondary);
	}

	.conn-dot.disconnected {
		background: var(--error);
	}

	.key-badge {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px;
		border-radius: 6px;
		font-size: 11px;
		font-weight: 600;
	}

	.key-badge__icon {
		width: 0.95rem;
		height: 0.95rem;
	}

	.key-badge.ok {
		background: color-mix(in oklch, var(--secondary) 16%, transparent);
		color: var(--secondary);
	}

	.key-badge.missing {
		background: color-mix(in oklch, var(--tertiary) 16%, transparent);
		color: var(--tertiary);
	}

	.key-badge.bad {
		background: color-mix(in oklch, var(--error) 16%, transparent);
		color: var(--error);
	}

	.toolbar {
		display: flex;
		gap: 10px;
		margin-bottom: 20px;
		flex-wrap: wrap;
		align-items: center;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 8px 16px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s,
			filter 0.15s;
		font-family: inherit;
		color: var(--on-surface);
	}

	.btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.btn--primary {
		background: var(--primary);
		border-color: transparent;
		color: var(--on-primary);
	}

	.btn--primary:hover:not(:disabled) {
		background: var(--primary-dim);
	}

	.btn--sm {
		padding: 6px 12px;
		background: var(--surface-container);
		border-color: var(--outline-variant);
		color: var(--on-surface);
	}

	.btn--sm:hover:not(:disabled) {
		border-color: var(--outline);
		background: var(--surface-container-high);
	}

	.btn--complete {
		background: var(--secondary);
		border-color: transparent;
		color: var(--on-secondary);
	}

	.btn--complete:hover {
		filter: brightness(1.1);
	}

	.btn--cancel {
		background: var(--surface-container);
		border-color: var(--outline-variant);
		color: var(--on-surface-variant);
	}

	.btn--cancel:hover {
		border-color: var(--error);
		color: var(--error);
	}

	.actions-cell .btn {
		padding: 5px 10px;
		border-radius: 8px;
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

	.inline-form input {
		padding: 8px 12px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface);
		color: var(--on-surface);
		font-size: 13px;
		outline: none;
		min-width: 140px;
		font-family: inherit;
	}

	.inline-form input:focus {
		border-color: var(--primary);
	}

	.section {
		margin-bottom: 24px;
	}

	.section-title {
		font-size: 12px;
		font-weight: 600;
		color: var(--on-surface-variant);
		margin-bottom: 10px;
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.table-wrap {
		overflow: hidden;
		border-radius: 12px;
		border: 1px solid var(--outline-variant);
	}

	table {
		width: 100%;
		border-collapse: collapse;
		background: var(--surface-container);
	}

	th {
		text-align: left;
		padding: 10px 14px;
		font-size: 11px;
		color: var(--on-surface-variant);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		border-bottom: 1px solid var(--outline-variant);
		background: var(--surface-container-high);
	}

	td {
		padding: 10px 14px;
		border-bottom: 1px solid var(--outline-variant);
	}

	tr:last-child td {
		border-bottom: none;
	}

	.status-badge {
		display: inline-block;
		padding: 2px 8px;
		border-radius: 6px;
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.status-badge.pending {
		background: color-mix(in oklch, var(--tertiary) 16%, transparent);
		color: var(--tertiary);
	}

	.status-badge.spinning {
		background: color-mix(in oklch, var(--primary) 16%, transparent);
		color: var(--primary);
	}

	.status-badge.completed {
		background: color-mix(in oklch, var(--secondary) 16%, transparent);
		color: var(--secondary);
	}

	.status-badge.error {
		background: color-mix(in oklch, var(--error) 16%, transparent);
		color: var(--error);
	}

	.status-badge.cancelled {
		background: color-mix(in oklch, var(--on-surface-variant) 14%, transparent);
		color: var(--on-surface-variant);
	}

	.empty-row {
		color: var(--on-surface-variant);
		font-style: italic;
		padding: 14px;
		text-align: center;
	}

	.actions-cell {
		display: flex;
		gap: 6px;
		justify-content: flex-end;
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

	.section .empty-row {
		background: var(--surface-container);
		border: 1px solid var(--outline-variant);
		border-radius: 12px;
	}
</style>
