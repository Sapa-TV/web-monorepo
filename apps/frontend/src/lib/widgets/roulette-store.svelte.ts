import { WS_URL, wapi } from "#lib/api";
import {
	HttpError,
	NetworkError,
	TimeoutError,
	type RarityResponse,
	type RouletteSlotResponse,
} from "@sapa-tv-ru/api-client";
import {
	connectWs,
	type SocketLike,
	type WsClient,
	type WsServerMessage,
} from "@sapa-tv-ru/ws-client";

export type RoulettePhase =
	"idle" | "spinning" | "completed" | "error" | "denied";

type ConnState = "connected" | "disconnected";

export interface SpinInfo {
	entry_id: number;
	user_name: string;
	slot_id: number | null;
	slot_name: string;
	slot_rarity: string;
}

export interface TrackItem {
	key: number;
	slot_id: number;
	name: string;
	rarity_id: number;
}

export const TRACK_LENGTH = 56;
const TRACK_TAIL_ITEMS = 8;
export const TRACK_WIN_INDEX = TRACK_LENGTH - TRACK_TAIL_ITEMS;

export function buildTrack(
	slots: RouletteSlotResponse[],
	winningSlotId: number,
	rng: () => number = Math.random,
): TrackItem[] {
	if (slots.length === 0) return [];
	const totalWeight = slots.reduce((sum, s) => sum + s.weight, 0);
	const pick = (): RouletteSlotResponse => {
		if (totalWeight <= 0) return slots[Math.floor(rng() * slots.length)];
		let roll = rng() * totalWeight;
		for (const slot of slots) {
			roll -= slot.weight;
			if (roll < 0) return slot;
		}
		return slots[slots.length - 1];
	};
	const winner =
		slots.find((s) => s.id === winningSlotId) ?? slots[slots.length - 1];
	return Array.from({ length: TRACK_LENGTH }, (_, i) => {
		const slot = i === TRACK_WIN_INDEX ? winner : pick();
		return {
			key: i,
			slot_id: slot.id,
			name: slot.name,
			rarity_id: slot.rarity_id,
		};
	});
}

const AUTO_COMPLETE_MS = 10_000;
const IDLE_AFTER_MS = 7_000;
const HTTP_UNAUTHORIZED = 401;

export interface RouletteStoreOptions {
	websocket?: new (_url: string) => SocketLike;
	autoCompleteMs?: number;
	idleAfterMs?: number;
}

export class RouletteStore {
	phase = $state<RoulettePhase>("idle");
	stateLabel = $state("Ожидание");
	idleText = $state("Ожидание следующего спина...");
	conn = $state<{ state: ConnState; label: string }>({
		state: "disconnected",
		label: "",
	});
	badge = $state<{
		cls: "ok" | "bad" | "missing";
		label: string;
		visible: boolean;
	}>({ cls: "missing", label: "", visible: false });
	spin = $state<SpinInfo | null>(null);
	slots = $state<RouletteSlotResponse[]>([]);
	rarities = $state<RarityResponse[]>([]);
	track = $state<TrackItem[]>([]);

	private readonly accessKey: string;
	private readonly wapiAuth: { headers: Record<string, string> };
	private readonly autoCompleteMs: number;
	private readonly idleAfterMs: number;
	private readonly socketFactory?: new (_url: string) => SocketLike;

	private ws: WsClient | null = null;
	private keyOk = false;
	private currentEntryId: number | null = null;
	private idleTimer: ReturnType<typeof setTimeout> | undefined;

	constructor(accessKey: string, options: RouletteStoreOptions = {}) {
		this.accessKey = accessKey;
		this.autoCompleteMs = options.autoCompleteMs ?? AUTO_COMPLETE_MS;
		this.idleAfterMs = options.idleAfterMs ?? IDLE_AFTER_MS;
		this.socketFactory = options.websocket;
		this.wapiAuth = { headers: { Authorization: `Bearer ${accessKey}` } };
	}

	start(): void {
		if (!this.accessKey) {
			this.failAuth();
			return;
		}
		void this.verifyKeyAndConnect();
	}

	stop(): void {
		clearTimeout(this.idleTimer);
		this.ws?.close();
		this.ws = null;
	}

	handleMessage(d: WsServerMessage): void {
		switch (d.type) {
			case "spin_started":
				if (d.entry_id != null && d.user_name && d.slot_name && d.slot_rarity) {
					this.setSpinning({
						entry_id: d.entry_id,
						user_name: d.user_name,
						slot_id: d.slot_id ?? null,
						slot_name: d.slot_name,
						slot_rarity: d.slot_rarity,
					});
				}
				break;
			case "spin_completed":
				if (d.entry_id === this.currentEntryId) this.setCompleted();
				break;
			case "spin_error":
				if (d.entry_id === this.currentEntryId) this.setSpinError();
				break;
		}
	}

	setIdle(): void {
		clearTimeout(this.idleTimer);
		this.stateLabel = "Ожидание";
		this.idleText = "Ожидание следующего спина...";
		this.phase = "idle";
		this.spin = null;
		this.track = [];
	}

	failAuth(): void {
		this.ws?.close();
		this.ws = null;
		clearTimeout(this.idleTimer);
		this.setConn("disconnected", "не авторизован");
		this.setKeyBadge(
			this.accessKey ? "bad" : "missing",
			this.accessKey ? "ключ неверный" : "нет ключа",
		);
		this.phase = "denied";
		this.stateLabel = "Доступ запрещён";
		this.idleText = "Ошибка подключения виджета.";
	}

	private setConn(state: ConnState, label: string) {
		this.conn = { state, label };
	}

	private setKeyBadge(cls: "ok" | "bad" | "missing", label = "") {
		this.badge = { cls, label, visible: true };
	}

	private setSpinning(data: SpinInfo) {
		clearTimeout(this.idleTimer);
		this.currentEntryId = data.entry_id;
		this.stateLabel = "Крутится!";
		this.phase = "spinning";
		this.spin = data;
		const winning =
			data.slot_id != null
				? this.slots.find((s) => s.id === data.slot_id)
				: this.slots.find((s) => s.name === data.slot_name);
		this.track = winning ? buildTrack(this.slots, winning.id) : [];
		this.idleTimer = setTimeout(() => {
			if (this.currentEntryId === data.entry_id) {
				void wapi.complete(data.entry_id, this.wapiAuth);
				this.setCompleted();
			}
		}, this.autoCompleteMs);
	}

	private setCompleted() {
		clearTimeout(this.idleTimer);
		this.stateLabel = "✔ Завершён";
		this.phase = "completed";
		this.idleTimer = setTimeout(() => this.setIdle(), this.idleAfterMs);
	}

	private setSpinError() {
		clearTimeout(this.idleTimer);
		this.stateLabel = "⚠ Ошибка";
		this.phase = "error";
		this.idleTimer = setTimeout(() => this.setIdle(), this.idleAfterMs);
	}

	private async verifyKeyAndConnect(): Promise<void> {
		const res = await wapi.list(undefined, this.wapiAuth);
		res.match(
			() => {
				this.keyOk = true;
				void this.loadSlots();
				this.connectSocket();
			},
			(err) => {
				if (err instanceof HttpError && err.status === HTTP_UNAUTHORIZED) {
					this.failAuth();
					return;
				}
				if (err instanceof NetworkError || err instanceof TimeoutError) {
					this.setConn("disconnected", "нет связи");
					this.setKeyBadge("bad", "нет связи с сервером");
					this.stateLabel = "Нет связи";
					this.idleText = "Сервер недоступен.";
					return;
				}
				this.setKeyBadge("bad", "ошибка запроса");
				this.stateLabel = "Ошибка подключения виджета";
				this.idleText = "Ошибка подключения виджета.";
			},
		);
	}

	private async loadSlots(): Promise<void> {
		const [slotsRes, raritiesRes] = await Promise.all([
			wapi.listSlots(this.wapiAuth),
			wapi.listRarities(this.wapiAuth),
		]);
		slotsRes.match(
			(slots) => {
				this.slots = slots;
			},
			() => {},
		);
		raritiesRes.match(
			(rarities) => {
				this.rarities = rarities;
			},
			() => {},
		);
	}

	private connectSocket() {
		if (!this.keyOk || this.ws) return;
		this.ws = connectWs({
			url: WS_URL,
			token: this.accessKey,
			role: "widget",
			onMessage: (m) => this.handleMessage(m),
			onState: (state) =>
				this.setConn(state, state === "connected" ? "подключено" : "отключено"),
			onAuthError: () => this.failAuth(),
			socketFactory: this.socketFactory,
		});
	}
}
