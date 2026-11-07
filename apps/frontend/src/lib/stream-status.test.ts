import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { errAsync, okAsync } from "neverthrow";
import { NetworkError } from "@sapa-tv-ru/api-client";

vi.mock("#lib/api", () => ({
	api: {
		getStreamStatus: vi.fn(),
	},
}));

import { api } from "#lib/api";
import { StreamStatusStore, streamPhaseFrom } from "./stream-status.svelte";

const mocked = api as unknown as {
	getStreamStatus: ReturnType<typeof vi.fn>;
};

describe("streamPhaseFrom", () => {
	it("maps the online flag to a phase", () => {
		expect(streamPhaseFrom(true)).toBe("online");
		expect(streamPhaseFrom(false)).toBe("offline");
	});
});

describe("StreamStatusStore", () => {
	beforeEach(() => {
		vi.useFakeTimers();
		mocked.getStreamStatus.mockReset();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("starts unknown before the first response", () => {
		const store = new StreamStatusStore();
		expect(store.phase).toBe("unknown");
	});

	it("starts online when the first poll reports a live stream", async () => {
		mocked.getStreamStatus.mockReturnValue(okAsync({ online: true }));

		const store = new StreamStatusStore();
		store.start();
		await vi.advanceTimersByTimeAsync(0);

		expect(store.phase).toBe("online");
		store.stop();
	});

	it("keeps the last known phase when polling fails", async () => {
		mocked.getStreamStatus
			.mockReturnValueOnce(okAsync({ online: true }))
			.mockReturnValueOnce(errAsync(new NetworkError(new Error("down"))));

		const store = new StreamStatusStore({ pollIntervalMs: 5_000 });
		store.start();
		await vi.advanceTimersByTimeAsync(0);
		expect(store.phase).toBe("online");

		await vi.advanceTimersByTimeAsync(5_000);
		expect(store.phase).toBe("online");
		store.stop();
	});

	it("flips to offline on a later poll", async () => {
		mocked.getStreamStatus
			.mockReturnValueOnce(okAsync({ online: true }))
			.mockReturnValueOnce(okAsync({ online: false }));

		const store = new StreamStatusStore({ pollIntervalMs: 5_000 });
		store.start();
		await vi.advanceTimersByTimeAsync(0);
		expect(store.phase).toBe("online");

		await vi.advanceTimersByTimeAsync(5_000);
		expect(store.phase).toBe("offline");
		store.stop();
	});

	it("stop prevents further polling", async () => {
		mocked.getStreamStatus.mockReturnValue(okAsync({ online: true }));

		const store = new StreamStatusStore({ pollIntervalMs: 5_000 });
		store.start();
		await vi.advanceTimersByTimeAsync(0);
		store.stop();

		await vi.advanceTimersByTimeAsync(60_000);
		expect(mocked.getStreamStatus).toHaveBeenCalledTimes(1);
	});
});
