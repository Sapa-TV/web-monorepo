import { WS_URL, wapi, type QueueEntry } from "#lib/api";
import { describeApiError } from "#lib/api-error-text";
import { ApiErrorKind, normalizeApiError } from "#lib/internal/api-error";
import { SvelteDate } from "svelte/reactivity";

export type DockEvent = {
	time: string;
	text: string;
	cls: "start" | "complete" | "error";
};

type KeyState = "ok" | "missing" | "bad";
type ConnState = "connected" | "disconnected";

const LOG_LIMIT = 50;
const WS_RETRY_BASE_MS = 1_000;
const WS_RETRY_MAX_MS = 15_000;
const DEFAULT_POLL_INTERVAL_MS = 10_000;

export interface WsMessage {
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
}

interface SocketLike {
	send(_data: string): void;
	close(): void;
	onopen: () => void;
	onclose: () => void;
	onerror: () => void;
	onmessage: (_e: { data: string }) => void;
}

export interface DockStoreOptions {
	pollIntervalMs?: number;
	websocket?: new (_url: string) => SocketLike;
}

export class DockStore {
	entries = $state<QueueEntry[]>([]);
	nextUser = $state("");
	dequeueLabel = $state("▶ Dequeue");
	keyState = $state<KeyState | null>(null);
	connState = $state<ConnState>("disconnected");
	widgetOnline = $state(false);
	dequeueBusy = $state(false);
	enqueueBusy = $state(false);
	events = $state<DockEvent[]>([]);

	active = $derived(
		this.entries.filter(
			(e) =>
				e.status === "Pending" ||
				e.status === "Spinning" ||
				e.status === "Error",
		),
	);
	done = $derived(
		this.entries.filter(
			(e) => e.status === "Completed" || e.status === "Cancelled",
		),
	);

	private readonly accessKey: string;
	private readonly wapiAuth: { headers: Record<string, string> };
	private readonly pollIntervalMs: number;
	private readonly socketFactory: new (_url: string) => SocketLike;

	private socket: SocketLike | null = null;
	private socketRejected = false;
	private socketRetryMs = WS_RETRY_BASE_MS;
	private pollTimer: ReturnType<typeof setInterval> | undefined;
	private started = false;

	constructor(accessKey: string, options: DockStoreOptions = {}) {
		this.accessKey = accessKey;
		this.pollIntervalMs = options.pollIntervalMs ?? DEFAULT_POLL_INTERVAL_MS;
		this.socketFactory =
			options.websocket ??
			(globalThis.WebSocket as unknown as new (_url: string) => SocketLike);
		this.wapiAuth = { headers: { Authorization: `Bearer ${accessKey}` } };
	}

	start(): void {
		if (this.started) return;
		this.started = true;
		void this.loadAll();
		this.pollTimer = setInterval(
			() => void this.loadAll(),
			this.pollIntervalMs,
		);
		this.connectSocket();
	}

	stop(): void {
		clearInterval(this.pollTimer);
		if (this.socket) {
			this.socket.onclose = () => {};
			this.socket.close();
			this.socket = null;
		}
	}

	async loadAll(): Promise<void> {
		const [listRes, statsRes] = await Promise.all([
			wapi.list(undefined, this.wapiAuth),
			wapi.stats(this.wapiAuth),
		]);

		listRes.match(
			(data) => {
				this.setKeyState("ok");
				this.entries = data.entries;
				const hasSpinning = this.entries.some((e) => e.status === "Spinning");
				if (!hasSpinning) {
					const next = this.entries.find(
						(e) => e.status === "Error" || e.status === "Pending",
					);
					this.nextUser = next ? next.user_name || "" : "";
				}
			},
			(err) => {
				if (normalizeApiError(err).kind === ApiErrorKind.Unauthorized) {
					this.setKeyState(this.accessKey ? "bad" : "missing");
				}
			},
		);

		statsRes.match(
			(s) => {
				this.dequeueLabel = `▶ Dequeue (${s.pending + s.error})`;
			},
			() => {},
		);
	}

	async dequeueNext(): Promise<void> {
		this.dequeueBusy = true;
		const res = await wapi.dequeueNext(this.wapiAuth);
		res.match(
			(data) =>
				this.addEvent(
					`🎰 ${data.slot?.name} → #${data.entry.id} (${data.entry.user_name})`,
					"start",
				),
			(err) => this.addEvent(`❌ Dequeue: ${describeApiError(err)}`, "error"),
		);
		await this.loadAll();
		this.dequeueBusy = false;
	}

	async complete(id: number): Promise<void> {
		const res = await wapi.complete(id, this.wapiAuth);
		res.match(
			() => this.addEvent(`✔ #${id} завершён`, "complete"),
			(err) =>
				this.addEvent(`❌ Complete #${id}: ${describeApiError(err)}`, "error"),
		);
		await this.loadAll();
	}

	async cancel(id: number): Promise<void> {
		const res = await wapi.cancel(id, this.wapiAuth);
		res.match(
			() => this.addEvent(`✕ #${id} отменён`, "error"),
			(err) =>
				this.addEvent(`❌ Cancel #${id}: ${describeApiError(err)}`, "error"),
		);
		await this.loadAll();
	}

	async enqueue(name: string): Promise<boolean> {
		const trimmed = name.trim();
		if (!trimmed) return false;
		this.enqueueBusy = true;
		const res = await wapi.enqueueAnonymous({ name: trimmed }, this.wapiAuth);
		let added = false;
		res.match(
			() => {
				added = true;
				this.addEvent(`➕ ${trimmed} добавлен`, "complete");
			},
			(err) => this.addEvent(`❌ Ошибка ${describeApiError(err)}`, "error"),
		);
		await this.loadAll();
		this.enqueueBusy = false;
		return added;
	}

	private setKeyState(state: KeyState) {
		this.keyState = state;
	}

	private addEvent(text: string, cls: DockEvent["cls"]) {
		this.events = [
			{ time: new SvelteDate().toLocaleTimeString(), text, cls },
			...this.events,
		].slice(0, LOG_LIMIT);
	}

	private connectSocket() {
		if (this.socketRejected) return;
		this.socket = new this.socketFactory(WS_URL);
		this.socket.onopen = () => {
			this.connState = "connected";
			if (this.accessKey) {
				this.socket?.send(
					JSON.stringify({
						type: "auth",
						token: this.accessKey,
						role: "dock",
					}),
				);
			}
		};
		this.socket.onclose = () => {
			this.connState = "disconnected";
			this.widgetOnline = false;
			if (!this.socketRejected) {
				setTimeout(() => this.connectSocket(), this.socketRetryMs);
				this.socketRetryMs = Math.min(this.socketRetryMs * 2, WS_RETRY_MAX_MS);
			}
		};
		this.socket.onerror = () => this.socket?.close();
		this.socket.onmessage = (e) => this.handleMessage(e.data);
	}

	private handleMessage(raw: string) {
		const d = JSON.parse(raw) as WsMessage;
		switch (d.type) {
			case "auth_ok":
				this.socketRetryMs = WS_RETRY_BASE_MS;
				break;
			case "auth_err":
				this.socketRejected = true;
				this.setKeyState(this.accessKey ? "bad" : "missing");
				break;
			case "presence":
				this.widgetOnline = (d.widget_count ?? 0) > 0;
				break;
			case "spin_started":
				this.addEvent(
					`🎰 #${d.entry_id} — ${d.user_name}: ${d.slot_name}`,
					"start",
				);
				void this.loadAll();
				break;
			case "spin_completed":
				this.addEvent(`✔ #${d.entry_id} завершён`, "complete");
				void this.loadAll();
				break;
			case "spin_error":
				this.addEvent(`⚠ #${d.entry_id} таймаут`, "error");
				void this.loadAll();
				break;
		}
	}
}
