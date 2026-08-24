import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";
import { errAsync, okAsync } from "neverthrow";
import { HttpError } from "@sapa-tv-ru/api-client";

vi.mock("#lib/api", () => ({
	WS_URL: "ws://test/wapi/ws",
	wapi: {
		list: vi.fn(),
		stats: vi.fn(),
		dequeueNext: vi.fn(),
		complete: vi.fn(),
		cancel: vi.fn(),
		enqueueAnonymous: vi.fn(),
	},
}));

import { wapi } from "#lib/api";
import { DockStore, type WsMessage } from "./dock-store.svelte";

type MockedWapi = {
	list: ReturnType<typeof vi.fn>;
	stats: ReturnType<typeof vi.fn>;
	dequeueNext: ReturnType<typeof vi.fn>;
	complete: ReturnType<typeof vi.fn>;
	cancel: ReturnType<typeof vi.fn>;
	enqueueAnonymous: ReturnType<typeof vi.fn>;
};

const mocked = wapi as unknown as MockedWapi;

function entry(overrides: Partial<Record<string, unknown>> = {}) {
	return {
		id: 1,
		user_id: "42",
		user_name: "viewer",
		status: "Pending",
		slot_name: null,
		result_slot_id: null,
		created_at: "2026-01-01T00:00:00Z",
		updated_at: "2026-01-01T00:00:00Z",
		...overrides,
	};
}

class FakeWebSocket {
	static instances: FakeWebSocket[] = [];

	url: string;
	onopen: () => void = () => {};
	onclose: () => void = () => {};
	onerror: () => void = () => {};
	onmessage: (_e: { data: string }) => void = () => {};
	sent: string[] = [];
	closedCount = 0;

	constructor(url: string) {
		this.url = url;
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

	emit(msg: WsMessage) {
		this.onmessage({ data: JSON.stringify(msg) });
	}
}

function lastSocket(): FakeWebSocket {
	const socket = FakeWebSocket.instances.at(-1);
	expect(socket).toBeDefined();
	return socket as FakeWebSocket;
}

describe("DockStore", () => {
	beforeEach(() => {
		vi.useFakeTimers();
		FakeWebSocket.instances = [];
		vi.mocked(wapi.list).mockReset();
		vi.mocked(wapi.stats).mockReset();
		mocked.dequeueNext.mockReset();
		mocked.complete.mockReset();
		mocked.cancel.mockReset();
		mocked.enqueueAnonymous.mockReset();

		mocked.stats.mockReturnValue(
			okAsync({
				pending: 2,
				error: 1,
				spinning: 0,
				completed: 5,
				cancelled: 0,
			}),
		);
		mocked.list.mockReturnValue(okAsync({ entries: [] }));
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	function createStore() {
		return new DockStore("key-1", {
			pollIntervalMs: 10_000,
			websocket: FakeWebSocket,
		});
	}

	it("loadAll fills entries and derived lists", async () => {
		mocked.list.mockReturnValue(
			okAsync({
				entries: [
					entry({ id: 1, status: "Pending" }),
					entry({ id: 2, status: "Completed", slot_name: "Поболтать" }),
				],
			}),
		);

		const store = createStore();
		await store.loadAll();

		expect(store.entries.length).toBe(2);
		expect(store.active.map((e) => e.id)).toEqual([1]);
		expect(store.done.map((e) => e.id)).toEqual([2]);
		expect(store.keyState).toBe("ok");
		expect(store.nextUser).toBe("viewer");
	});

	it("loadAll keeps nextUser empty while someone is spinning", async () => {
		mocked.list.mockReturnValue(
			okAsync({
				entries: [
					entry({ id: 1, status: "Pending" }),
					entry({ id: 3, status: "Spinning", user_name: "spinner" }),
				],
			}),
		);

		const store = createStore();
		await store.loadAll();

		expect(store.active.length).toBe(2);
		expect(store.nextUser).toBe("");
	});

	it("loadAll unauthorized marks the key bad when key present", async () => {
		mocked.list.mockReturnValue(
			errAsync(new HttpError(401, "Unauthorized", null)),
		);

		const store = createStore();
		await store.loadAll();

		expect(store.keyState).toBe("bad");
	});

	it("dequeueNext logs a start event and reloads", async () => {
		mocked.list.mockReturnValue(okAsync({ entries: [] }));
		mocked.dequeueNext.mockReturnValue(
			okAsync({
				entry: entry({ id: 7, status: "Spinning" }),
				slot: { id: 1, name: "Джекпот" },
			}),
		);

		const store = createStore();
		await store.loadAll();
		await store.dequeueNext();

		expect(store.events[0]?.cls).toBe("start");
		expect(store.events[0]?.text).toContain("#7");
		expect(mocked.list).toHaveBeenCalledTimes(2);
	});

	it("enqueue rejects empty names without calling api", async () => {
		const store = createStore();

		const added = await store.enqueue("   ");

		expect(added).toBe(false);
		expect(mocked.enqueueAnonymous).not.toHaveBeenCalled();
	});

	it("enqueue trims, reports success and clears via returned flag", async () => {
		mocked.list.mockReturnValue(okAsync({ entries: [] }));
		mocked.enqueueAnonymous.mockReturnValue(okAsync({ id: 9 }));

		const store = createStore();
		const added = await store.enqueue("  viewer  ");

		expect(added).toBe(true);
		expect(mocked.enqueueAnonymous).toHaveBeenCalledWith(
			{ name: "viewer" },
			{ headers: { Authorization: "Bearer key-1" } },
		);
		expect(store.events[0]?.text).toContain("viewer добавлен");
	});

	it("enqueue failure keeps the flag false and logs an error event", async () => {
		mocked.list.mockReturnValue(okAsync({ entries: [] }));
		mocked.enqueueAnonymous.mockReturnValue(
			errAsync(new HttpError(409, "Conflict", null)),
		);

		const store = createStore();
		const added = await store.enqueue("dup");

		expect(added).toBe(false);
		expect(store.events[0]?.cls).toBe("error");
	});

	async function startStore(store: DockStore) {
		store.start();
		await vi.advanceTimersByTimeAsync(0);
	}

	it("socket handshake sends auth with dock role", async () => {
		const store = createStore();
		await startStore(store);

		const socket = lastSocket();
		socket.open();

		expect(store.connState).toBe("connected");
		expect(socket.sent).toHaveLength(1);
		expect(JSON.parse(socket.sent[0])).toEqual({
			type: "auth",
			token: "key-1",
			role: "dock",
		});
	});

	it("presence message toggles widget online flag", async () => {
		const store = createStore();
		await startStore(store);
		lastSocket().open();

		lastSocket().emit({ type: "presence", widget_count: 2 });
		expect(store.widgetOnline).toBe(true);

		lastSocket().emit({ type: "presence", widget_count: 0 });
		expect(store.widgetOnline).toBe(false);
	});

	it("spin messages log events and refresh entries", async () => {
		mocked.list.mockReturnValue(okAsync({ entries: [] }));

		const store = createStore();
		await startStore(store);
		mocked.list.mockClear();

		lastSocket().open();
		lastSocket().emit({
			type: "spin_started",
			entry_id: 4,
			user_name: "u",
			slot_name: "s",
		});
		await vi.advanceTimersByTimeAsync(0);

		expect(store.events[0]?.cls).toBe("start");
		expect(mocked.list).toHaveBeenCalledTimes(1);
	});

	it("reconnects with backoff and stops after auth_err", async () => {
		mocked.list.mockReturnValue(okAsync({ entries: [] }));

		const store = createStore();
		store.start();
		const first = lastSocket();
		first.open();
		first.close();

		await vi.advanceTimersByTimeAsync(999);
		expect(FakeWebSocket.instances.length).toBe(1);

		await vi.advanceTimersByTimeAsync(1);
		expect(FakeWebSocket.instances.length).toBe(2);

		const second = lastSocket();
		second.open();
		second.emit({ type: "auth_err" });
		expect(store.keyState).toBe("bad");

		second.close();
		await vi.advanceTimersByTimeAsync(60_000);
		expect(FakeWebSocket.instances.length).toBe(2);
	});

	it("stop closes the socket and prevents reconnects", async () => {
		mocked.list.mockReturnValue(okAsync({ entries: [] }));

		const store = createStore();
		await startStore(store);
		const socket = lastSocket();
		socket.open();

		store.stop();
		expect(socket.closedCount).toBe(1);

		await vi.advanceTimersByTimeAsync(60_000);
		expect(FakeWebSocket.instances.length).toBe(1);
	});
});
