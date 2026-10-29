import { WS_URL, wapi, type QueueEntry } from "#lib/api";
import { describeApiError } from "#lib/api-error-text";
import { ApiErrorKind, normalizeApiError } from "#lib/internal/api-error";
import {
	connectWs,
	type SocketLike,
	type WsClient,
	type WsServerMessage,
} from "@sapa-tv-ru/ws-client";
import { SvelteDate } from "svelte/reactivity";

export type DockEvent = {
	time: string;
	text: string;
	cls: "start" | "complete" | "error";
};

type KeyState = "ok" | "missing" | "bad";
type ConnState = "connected" | "disconnected";

const LOG_LIMIT = 50;
const DEFAULT_POLL_INTERVAL_MS = 10_000;

const PLATFORM_NAMES: Record<number, string> = {
	1: "Twitch",
	2: "YouTube",
	3: "VKVL",
};

function platformSuffix(platform: number | null): string {
	if (platform === null) return "";
	return ` (${PLATFORM_NAMES[platform] ?? platform})`;
}

export interface DockStoreOptions {
	pollIntervalMs?: number;
	websocket?: new (_url: string) => SocketLike;
}

export class DockStore {
	entries = $state<QueueEntry[]>([]);
	nextUser = $state("");
	nextPlatform = $state<number | null>(null);
	dequeueLabel = $state("Крутить");
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
	private readonly socketFactory?: new (_url: string) => SocketLike;

	private ws: WsClient | null = null;
	private pollTimer: ReturnType<typeof setInterval> | undefined;
	private started = false;

	constructor(accessKey: string, options: DockStoreOptions = {}) {
		this.accessKey = accessKey;
		this.pollIntervalMs = options.pollIntervalMs ?? DEFAULT_POLL_INTERVAL_MS;
		this.socketFactory = options.websocket;
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
		this.ws?.close();
		this.ws = null;
	}

	async loadAll(): Promise<void> {
		const listRes = await wapi.list(undefined, this.wapiAuth);

		listRes.match(
			(data) => {
				this.setKeyState("ok");
				this.entries = data.entries;
				const hasSpinning = this.entries.some((e) => e.status === "Spinning");
				if (hasSpinning) {
					this.dequeueLabel = "Крутится...";
					return;
				}
				const next = this.entries.find(
					(e) => e.status === "Error" || e.status === "Pending",
				);
				this.nextUser = next ? next.user_name || "" : "";
				this.nextPlatform = next?.platform ?? null;
				this.dequeueLabel = next
					? `Крутить для: ${this.nextUser}${platformSuffix(this.nextPlatform)}`
					: "Крутить: очередь пуста";
			},
			(err) => {
				if (normalizeApiError(err).kind === ApiErrorKind.Unauthorized) {
					this.setKeyState(this.accessKey ? "bad" : "missing");
				}
			},
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
		this.ws = connectWs({
			url: WS_URL,
			token: this.accessKey,
			role: "dock",
			onMessage: (m) => this.handleMessage(m),
			onState: (state) => {
				this.connState = state;
				if (state === "disconnected") this.widgetOnline = false;
			},
			onAuthError: () => this.setKeyState(this.accessKey ? "bad" : "missing"),
			socketFactory: this.socketFactory,
		});
	}

	private handleMessage(d: WsServerMessage) {
		switch (d.type) {
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
