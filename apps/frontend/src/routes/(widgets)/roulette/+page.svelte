<script lang="ts">
	import { onMount } from "svelte";
	import { WAPI_BASE, WS_URL, apiFetch } from "#lib/api";

	type ConnState = "connected" | "disconnected";

	const AUTO_MS = 10_000;
	const IDLE_AFTER_MS = 4_000;
	const WS_RETRY_BASE_MS = 1_000;
	const WS_RETRY_MAX_MS = 15_000;
	const UNAUTHORIZED = 401;

	const pak =
		typeof window !== "undefined"
			? (new URLSearchParams(window.location.search).get("pak") ?? "")
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
		setKeyBadge(pak ? "bad" : "missing", pak ? "ключ неверный" : "нет ключа");
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
					pak,
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
			if (pak) ws?.send(JSON.stringify({ type: "auth", token: pak }));
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
		if (!pak) {
			failAuth();
			return;
		}

		setIdle();
		apiFetch(`${WAPI_BASE}/queue`, {}, pak)
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
	:global(html) {
		background: transparent;
	}

	:global(body) {
		margin: 0;
		background: transparent;
		font-family: "Inter", system-ui, sans-serif;
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		color: #f2e9dc;
	}

	.widget {
		position: relative;
		background: color-mix(in oklch, oklch(0.12 0.01 47) 86%, transparent);
		backdrop-filter: blur(14px);
		border-radius: 20px;
		padding: 44px 52px;
		text-align: center;
		min-width: 360px;
		box-shadow:
			0 18px 60px rgba(0, 0, 0, 0.6),
			inset 0 1px 0 rgba(255, 255, 255, 0.08),
			inset 0 0 0 1px rgba(255, 255, 255, 0.06);
		border: 1px solid color-mix(in oklch, oklch(0.78 0.1 47) 22%, transparent);
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
			rgba(255, 255, 255, 0.025) 2px 3px
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
		color: color-mix(in oklch, oklch(0.78 0.1 47) 75%, white);
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
		color: color-mix(in oklch, oklch(0.78 0.1 47) 60%, white);
		font-weight: 300;
		letter-spacing: 0.01em;
	}

	.user-name {
		font-size: 20px;
		font-weight: 600;
		margin-bottom: 8px;
		color: #f6efe5;
	}

	.slot-name {
		font-size: 40px;
		font-weight: 800;
		margin-bottom: 6px;
		color: oklch(0.85 0.16 47);
		font-family: "Archivo", sans-serif;
		letter-spacing: -0.01em;
	}

	.slot-rarity {
		font-size: 15px;
		font-weight: 500;
		opacity: 0.8;
		color: #d9cfbf;
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	.entry-id {
		font-size: 12px;
		color: color-mix(in oklch, oklch(0.78 0.1 47) 55%, white);
		margin-top: 14px;
		font-family: "IBM Plex Mono", monospace;
		letter-spacing: 0.04em;
	}

	.spinner {
		width: 48px;
		height: 48px;
		border: 3px solid rgba(255, 255, 255, 0.08);
		border-top-color: oklch(0.78 0.16 47);
		border-right-color: oklch(0.78 0.16 47);
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
		color: rgba(217, 207, 191, 0.65);
		background: rgba(0, 0, 0, 0.5);
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
		background: oklch(0.78 0.12 165);
	}

	.conn-dot.disconnected {
		background: oklch(0.65 0.2 25);
	}

	.key-badge {
		padding: 2px 10px;
		border-radius: 6px;
		font-size: 11px;
		font-weight: 600;
	}

	.key-badge.ok {
		background: oklch(0.78 0.12 165 / 0.14);
		color: oklch(0.8 0.12 165);
	}

	.key-badge.missing {
		background: oklch(0.78 0.12 85 / 0.14);
		color: oklch(0.8 0.12 85);
	}

	.key-badge.bad {
		background: oklch(0.65 0.2 25 / 0.14);
		color: oklch(0.75 0.18 25);
	}
</style>
