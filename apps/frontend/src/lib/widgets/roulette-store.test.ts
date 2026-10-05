import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { errAsync, okAsync } from "neverthrow";
import { HttpError, NetworkError, TimeoutError } from "@sapa-tv-ru/api-client";

vi.mock("#lib/api", () => ({
	WS_URL: "ws://test/wapi/ws",
	wapi: {
		list: vi.fn(),
		complete: vi.fn(),
		listSlots: vi.fn(),
		listRarities: vi.fn(),
	},
}));

import { wapi } from "#lib/api";
import {
	buildTrack,
	RouletteStore,
	TRACK_LENGTH,
	TRACK_WIN_INDEX,
	type WsMessageLike,
} from "./roulette-store.svelte";

type MockedWapi = {
	list: ReturnType<typeof vi.fn>;
	complete: ReturnType<typeof vi.fn>;
	listSlots: ReturnType<typeof vi.fn>;
	listRarities: ReturnType<typeof vi.fn>;
};

const mocked = wapi as unknown as MockedWapi;

class FakeWebSocket {
	static instances: FakeWebSocket[] = [];

	onopen: () => void = () => {};
	onclose: () => void = () => {};
	onerror: () => void = () => {};
	onmessage: (_e: { data: string }) => void = () => {};
	sent: string[] = [];
	closedCount = 0;

	constructor(_url: string) {
		FakeWebSocket.instances.push(this);
	}

	send(data: string) {
		this.sent.push(data);
	}

	close() {
		this.closedCount += 1;
		this.onclose();
	}

	open() {
		this.onopen();
	}

	emit(msg: WsMessageLike) {
		this.onmessage({ data: JSON.stringify(msg) });
	}
}

function lastSocket(): FakeWebSocket {
	const socket = FakeWebSocket.instances.at(-1);
	expect(socket).toBeDefined();
	return socket as FakeWebSocket;
}

function spinMessage(overrides: Partial<WsMessageLike> = {}) {
	return {
		type: "spin_started",
		entry_id: 7,
		user_name: "viewer",
		slot_id: 2,
		slot_name: "Джекпот",
		slot_rarity: "legendary",
		...overrides,
	} as WsMessageLike;
}

describe("RouletteStore", () => {
	beforeEach(() => {
		vi.useFakeTimers();
		FakeWebSocket.instances = [];
		mocked.list.mockReset();
		mocked.complete.mockReset();
		mocked.listSlots.mockReset();
		mocked.listRarities.mockReset();
		mocked.list.mockReturnValue(okAsync({ entries: [] }));
		mocked.listSlots.mockReturnValue(okAsync([]));
		mocked.listRarities.mockReturnValue(okAsync([]));
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	function createStore() {
		return new RouletteStore("key-1", {
			websocket: FakeWebSocket,
			autoCompleteMs: 5_000,
			idleAfterMs: 4_000,
		});
	}

	async function startConnected() {
		const store = createStore();
		store.start();
		await vi.advanceTimersByTimeAsync(0);
		lastSocket().open();
		return store;
	}

	it("start without key denies access immediately", () => {
		const store = new RouletteStore("", { websocket: FakeWebSocket });
		store.start();

		expect(store.phase).toBe("denied");
		expect(store.badge.cls).toBe("missing");
		expect(mocked.list).not.toHaveBeenCalled();
	});

	it("successful key check connects and sends widget auth", async () => {
		const store = createStore();
		store.start();
		await vi.advanceTimersByTimeAsync(0);

		lastSocket().open();

		expect(store.conn.state).toBe("connected");
		expect(JSON.parse(lastSocket().sent[0])).toEqual({
			type: "auth",
			token: "key-1",
			role: "widget",
		});
	});

	it("401 on key check fails auth without opening a socket", async () => {
		mocked.list.mockReturnValue(
			errAsync(new HttpError(401, "Unauthorized", null)),
		);

		const store = createStore();
		store.start();
		await vi.advanceTimersByTimeAsync(0);

		expect(store.phase).toBe("denied");
		expect(store.badge.cls).toBe("bad");
		expect(FakeWebSocket.instances.length).toBe(0);
	});

	it("network problems keep idle phase with offline texts", async () => {
		mocked.list.mockReturnValue(errAsync(new NetworkError(new Error("down"))));

		const store = createStore();
		store.start();
		await vi.advanceTimersByTimeAsync(0);

		expect(store.phase).toBe("idle");
		expect(store.conn.label).toBe("нет связи");
		expect(store.idleText).toBe("Сервер недоступен.");
	});

	it("timeout problems keep idle phase with offline texts", async () => {
		mocked.list.mockReturnValue(errAsync(new TimeoutError(1234)));

		const store = createStore();
		store.start();
		await vi.advanceTimersByTimeAsync(0);

		expect(store.phase).toBe("idle");
		expect(store.stateLabel).toBe("Нет связи");
	});

	it("other errors show generic failure texts", async () => {
		mocked.list.mockReturnValue(
			errAsync(new HttpError(500, "Server Error", null)),
		);

		const store = createStore();
		store.start();
		await vi.advanceTimersByTimeAsync(0);

		expect(store.phase).toBe("idle");
		expect(store.stateLabel).toBe("Ошибка подключения виджета");
	});

	it("spin_started switches to spinning phase", async () => {
		const store = await startConnected();
		lastSocket().emit(spinMessage());

		expect(store.phase).toBe("spinning");
		expect(store.spin?.entry_id).toBe(7);
		expect(store.spin?.slot_name).toBe("Джекпот");
		expect(store.stateLabel).toBe("Крутится!");
	});

	it("incomplete spin_started is ignored", async () => {
		const store = await startConnected();
		lastSocket().emit(spinMessage({ slot_rarity: undefined }));

		expect(store.phase).toBe("idle");
		expect(store.spin).toBeNull();
	});

	it("auto-completes after autoCompleteMs and returns to idle", async () => {
		const store = await startConnected();
		mocked.complete.mockReturnValue(okAsync({}));
		lastSocket().emit(spinMessage());

		await vi.advanceTimersByTimeAsync(4_999);
		expect(store.phase).toBe("spinning");
		expect(mocked.complete).not.toHaveBeenCalled();

		await vi.advanceTimersByTimeAsync(1);
		expect(mocked.complete).toHaveBeenCalledWith(7, {
			headers: { Authorization: "Bearer key-1" },
		});
		expect(store.phase).toBe("completed");

		await vi.advanceTimersByTimeAsync(4_000);
		expect(store.phase).toBe("idle");
		expect(store.spin).toBeNull();
	});

	it("spin_completed only affects the current entry", async () => {
		const store = await startConnected();
		lastSocket().emit(spinMessage());
		lastSocket().emit({ type: "spin_completed", entry_id: 999 });
		expect(store.phase).toBe("spinning");

		lastSocket().emit({ type: "spin_completed", entry_id: 7 });
		expect(store.phase).toBe("completed");
	});

	it("spin_error only affects the current entry", async () => {
		const store = await startConnected();
		lastSocket().emit(spinMessage({ entry_id: 8 }));
		lastSocket().emit({ type: "spin_error", entry_id: 7 });
		expect(store.phase).toBe("spinning");

		lastSocket().emit({ type: "spin_error", entry_id: 8 });
		expect(store.phase).toBe("error");
		expect(store.stateLabel).toBe("⚠ Ошибка");
	});

	it("stop prevents the auto-complete timer", async () => {
		const store = await startConnected();
		lastSocket().emit(spinMessage());

		store.stop();
		await vi.advanceTimersByTimeAsync(60_000);

		expect(store.phase).toBe("spinning");
		expect(mocked.complete).not.toHaveBeenCalled();
	});

	it("auth_err denies and close does not reconnect", async () => {
		const store = await startConnected();
		const socket = lastSocket();

		socket.emit({ type: "auth_err" });
		expect(store.phase).toBe("denied");

		socket.close();
		await vi.advanceTimersByTimeAsync(60_000);

		expect(FakeWebSocket.instances.length).toBe(1);
	});

	it("spin_started builds a track ending on the winning slot", async () => {
		mocked.listSlots.mockReturnValue(
			okAsync([
				{ id: 1, name: "Обычное", rarity_id: 1, weight: 9, action: "" },
				{ id: 2, name: "Джекпот", rarity_id: 2, weight: 1, action: "" },
			]),
		);

		const store = await startConnected();
		await vi.advanceTimersByTimeAsync(0);
		lastSocket().emit(spinMessage());

		expect(store.track.length).toBe(TRACK_LENGTH);
		expect(store.track[TRACK_WIN_INDEX].slot_id).toBe(2);
	});

	it("spin without matching slot leaves the track empty", async () => {
		const store = await startConnected();
		await vi.advanceTimersByTimeAsync(0);
		lastSocket().emit(spinMessage());

		expect(store.phase).toBe("spinning");
		expect(store.track).toEqual([]);
	});
});

describe("buildTrack", () => {
	const slots = [
		{ id: 1, name: "A", rarity_id: 1, weight: 1, action: "" },
		{ id: 2, name: "B", rarity_id: 2, weight: 3, action: "" },
	];

	it("returns empty track for empty slots", () => {
		expect(buildTrack([], 1)).toEqual([]);
	});

	it("places the winning slot at TRACK_WIN_INDEX", () => {
		const track = buildTrack(slots, 1, () => 0.99);
		expect(track.length).toBe(TRACK_LENGTH);
		expect(track[TRACK_WIN_INDEX].name).toBe("A");
	});

	it("picks slots weighted by weight", () => {
		const track = buildTrack(slots, 1, () => 0.5);
		const filler = track.filter((_, i) => i !== TRACK_WIN_INDEX);
		expect(filler.every((item) => item.slot_id === 2)).toBe(true);
	});
});
