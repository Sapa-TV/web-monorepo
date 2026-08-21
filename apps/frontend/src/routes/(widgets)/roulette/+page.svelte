<script lang="ts">
	import { onMount } from "svelte";
	import { WAPI_BASE, WS_URL, apiFetch } from "#lib/api";

	type ConnState = "connected" | "disconnected";

	const AUTO_MS = 10_000;
	const IDLE_AFTER_MS = 4_000;
	const WS_RETRY_BASE_MS = 1_000;
	const WS_RETRY_MAX_MS = 15_000;
	const UNAUTHORIZED = 401;

	const widgetAccessKey =
		typeof window !== "undefined"
			? (new URLSearchParams(window.location.search).get("widget_access_key") ??
				"")
			: "";

	let phase = $state<"idle" | "spinning" | "completed" | "error" | "denied">(
		"idle",
	);
	let stateLabel = $state("Ожидание");
	let idleText = $state("Ожидание следующего спина...");
	let conn = $state<{ state: ConnState; label: string }>({
		state: "disconnected",
		label: "",
	});
	let badge = $state<{
		cls: "ok" | "bad" | "missing";
		label: string;
		visible: boolean;
	}>({
		cls: "missing",
		label: "",
		visible: false,
	});
	let spin = $state<{
		entry_id: number;
		user_name: string;
		slot_name: string;
		slot_rarity: string;
	} | null>(null);

	let currentEntryId: number | null = null;
	let idleTimer: ReturnType<typeof setTimeout> | undefined;
	let ws: WebSocket | null = null;
	let wsQueued = false;
	let wsRejected = false;
	let wsRetryMs = WS_RETRY_BASE_MS;
	let keyOk = false;

	function setConn(state: ConnState, label: string) {
		conn = { state, label };
	}

	function setKeyBadge(cls: "ok" | "bad" | "missing", label = "") {
		badge = { cls, label, visible: true };
	}

	function failAuth() {
		if (ws) {
			ws.onclose = null;
			ws.close();
			ws = null;
		}
		clearTimeout(idleTimer);
		setConn("disconnected", "не авторизован");
		setKeyBadge(
			widgetAccessKey ? "bad" : "missing",
			widgetAccessKey ? "ключ неверный" : "нет ключа",
		);
		phase = "denied";
		stateLabel = "Доступ запрещён";
		idleText = "Ошибка подключения виджета.";
	}

	function setIdle() {
		clearTimeout(idleTimer);
		stateLabel = "Ожидание";
		idleText = "Ожидание следующего спина...";
		phase = "idle";
		spin = null;
	}

	function setSpinning(data: NonNullable<typeof spin>) {
		clearTimeout(idleTimer);
		currentEntryId = data.entry_id;
		stateLabel = "Крутится!";
		phase = "spinning";
		spin = data;
		idleTimer = setTimeout(() => {
			if (currentEntryId === data.entry_id) {
				apiFetch(
					`${WAPI_BASE}/queue/${currentEntryId}/complete`,
					{ method: "POST" },
					widgetAccessKey,
				).catch(() => {});
				setCompleted();
			}
		}, AUTO_MS);
	}

	function setCompleted() {
		clearTimeout(idleTimer);
		stateLabel = "✔ Завершён";
		phase = "completed";
		idleTimer = setTimeout(setIdle, IDLE_AFTER_MS);
	}

	function setSpinError() {
		clearTimeout(idleTimer);
		stateLabel = "⚠ Ошибка";
		phase = "error";
		idleTimer = setTimeout(setIdle, IDLE_AFTER_MS);
	}

	function connectWs() {
		if (!keyOk || wsQueued || wsRejected) return;
		wsQueued = true;
		ws = new WebSocket(WS_URL);

		ws.onopen = () => {
			setConn("connected", "подключено");
			if (widgetAccessKey)
				ws?.send(JSON.stringify({ type: "auth", token: widgetAccessKey }));
		};

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
				slot_rarity?: string;
			};
			switch (d.type) {
				case "auth_ok":
					wsRetryMs = WS_RETRY_BASE_MS;
					break;
				case "auth_err":
					wsRejected = true;
					failAuth();
					break;
				case "spin_started":
					if (
						d.entry_id != null &&
						d.user_name &&
						d.slot_name &&
						d.slot_rarity
					) {
						setSpinning({
							entry_id: d.entry_id,
							user_name: d.user_name,
							slot_name: d.slot_name,
							slot_rarity: d.slot_rarity,
						});
					}
					break;
				case "spin_completed":
					if (d.entry_id === currentEntryId) setCompleted();
					break;
				case "spin_error":
					if (d.entry_id === currentEntryId) setSpinError();
					break;
			}
		};

		ws.onclose = () => {
			wsQueued = false;
			setConn("disconnected", "отключено");
			if (keyOk && !wsRejected) {
				setTimeout(connectWs, wsRetryMs);
				wsRetryMs = Math.min(wsRetryMs * 2, WS_RETRY_MAX_MS);
			}
		};

		ws.onerror = () => ws?.close();
	}

	onMount(() => {
		if (!widgetAccessKey) {
			failAuth();
			return;
		}

		setIdle();
		apiFetch(`${WAPI_BASE}/queue`, {}, widgetAccessKey)
			.then((res) => {
				if (res.status === UNAUTHORIZED) {
					failAuth();
				} else if (!res.ok) {
					setKeyBadge("bad", "ошибка запроса");
					stateLabel = "Ошибка подключения виджета";
					idleText = "Ошибка подключения виджета.";
				} else {
					keyOk = true;
					connectWs();
				}
			})
			.catch(() => {
				setConn("disconnected", "нет связи");
				setKeyBadge("bad", "нет связи с сервером");
				stateLabel = "Нет связи";
				idleText = "Сервер недоступен.";
			});

		return () => {
			clearTimeout(idleTimer);
			ws?.close();
		};
	});
</script>

<svelte:head>
	<title>Виджет — Рулетка</title>
</svelte:head>

<div class="widget">
	<div class="widget__state">
		<span class="state-dot" aria-hidden="true"></span>
		{stateLabel}
	</div>

	{#if phase === "spinning"}
		<div class="spinner" aria-hidden="true"></div>
	{/if}

	{#if phase === "idle" || phase === "denied"}
		<div class="idle-text">{idleText}</div>
	{/if}

	{#if spin && phase !== "idle" && phase !== "denied"}
		<div class="spin-info">
			<div class="user-name">{spin.user_name}</div>
			<div class="slot-name">{spin.slot_name}</div>
			<div class="slot-rarity">{spin.slot_rarity}</div>
			<div class="entry-id">
				{phase === "error"
					? `#${spin.entry_id} — таймаут`
					: `#${spin.entry_id}`}
			</div>
		</div>
	{/if}
</div>

<div class="conn-badge">
	<span class={`conn-dot ${conn.state}`}></span>
	<span>{conn.label}</span>
	{#if badge.visible}
		<span class={`key-badge ${badge.cls}`}>{badge.label}</span>
	{/if}
</div>

<style>
	.widget {
		position: relative;
		background: color-mix(in oklch, var(--widget-bg) 86%, transparent);
		backdrop-filter: blur(14px);
		border-radius: 20px;
		padding: 44px 52px;
		text-align: center;
		min-width: 360px;
		box-shadow:
			0 18px 60px var(--widget-shadow),
			inset 0 1px 0 var(--widget-hairline),
			inset 0 0 0 1px var(--widget-hairline-soft);
		border: 1px solid var(--widget-border);
		overflow: hidden;
	}

	.widget::before {
		content: "";
		position: absolute;
		inset: 0;
		pointer-events: none;
		background-image: repeating-linear-gradient(
			to bottom,
			transparent 0 2px,
			var(--widget-scanline) 2px 3px
		);
		border-radius: inherit;
	}

	.widget__state {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		font-size: 13px;
		text-transform: uppercase;
		letter-spacing: 0.22em;
		color: var(--widget-state);
		margin-bottom: 18px;
		font-weight: 600;
	}

	.state-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: currentColor;
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.3;
		}
	}

	.idle-text {
		font-size: 26px;
		color: var(--widget-idle);
		font-weight: 300;
		letter-spacing: 0.01em;
	}

	.user-name {
		font-size: 20px;
		font-weight: 600;
		margin-bottom: 8px;
		color: var(--widget-ink);
	}

	.slot-name {
		font-size: 40px;
		font-weight: 800;
		margin-bottom: 6px;
		color: var(--widget-accent);
		font-family: "Archivo", sans-serif;
		letter-spacing: -0.01em;
	}

	.slot-rarity {
		font-size: 15px;
		font-weight: 500;
		opacity: 0.8;
		color: var(--widget-ink-muted);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	.entry-id {
		font-size: 12px;
		color: var(--widget-entry);
		margin-top: 14px;
		font-family: "IBM Plex Mono", monospace;
		letter-spacing: 0.04em;
	}

	.spinner {
		width: 48px;
		height: 48px;
		border: 3px solid var(--widget-track);
		border-top-color: var(--widget-spinner);
		border-right-color: var(--widget-spinner);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
		margin: 0 auto 18px;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.conn-badge {
		position: fixed;
		bottom: 12px;
		right: 12px;
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--widget-ink-faint);
		background: var(--widget-overlay);
		padding: 6px 12px;
		border-radius: 6px;
		font-family: "IBM Plex Mono", monospace;
		letter-spacing: 0.03em;
	}

	.conn-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}

	.conn-dot.connected {
		background: var(--widget-ok-dot);
	}

	.conn-dot.disconnected {
		background: var(--widget-bad-dot);
	}

	.key-badge {
		padding: 2px 10px;
		border-radius: 6px;
		font-size: 11px;
		font-weight: 600;
	}

	.key-badge.ok {
		background: var(--widget-ok-dim);
		color: var(--widget-ok);
	}

	.key-badge.missing {
		background: var(--widget-missing-dim);
		color: var(--widget-missing);
	}

	.key-badge.bad {
		background: var(--widget-bad-dim);
		color: var(--widget-bad);
	}
</style>
