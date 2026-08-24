<script lang="ts">
	import { onMount } from "svelte";
	import { WS_URL, wapi } from "#lib/api";
	import RouletteStage from "#lib/components/widgets/RouletteStage.svelte";
	import {
		HttpError,
		NetworkError,
		TimeoutError,
	} from "@sapa-tv-ru/api-client";

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

	const wapiAuth = {
		headers: { Authorization: `Bearer ${widgetAccessKey}` },
	};

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
				void wapi.complete(currentEntryId, wapiAuth);
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
				ws?.send(
					JSON.stringify({
						type: "auth",
						token: widgetAccessKey,
						role: "widget",
					}),
				);
		};

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
		wapi.list(undefined, wapiAuth).then((res) =>
			res.match(
				() => {
					keyOk = true;
					connectWs();
				},
				(err) => {
					if (err instanceof HttpError && err.status === UNAUTHORIZED) {
						failAuth();
					} else if (
						err instanceof NetworkError ||
						err instanceof TimeoutError
					) {
						setConn("disconnected", "нет связи");
						setKeyBadge("bad", "нет связи с сервером");
						stateLabel = "Нет связи";
						idleText = "Сервер недоступен.";
					} else {
						setKeyBadge("bad", "ошибка запроса");
						stateLabel = "Ошибка подключения виджета";
						idleText = "Ошибка подключения виджета.";
					}
				},
			),
		);

		return () => {
			clearTimeout(idleTimer);
			ws?.close();
		};
	});
</script>

<svelte:head>
	<title>Виджет — Рулетка</title>
</svelte:head>

<RouletteStage {phase} {stateLabel} {idleText} {spin} {conn} {badge} />
