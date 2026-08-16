import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("#lib/api", () => ({
	api: {},
	apiFetch: vi.fn(),
}));

import { apiFetch } from "#lib/api";
import { completeCredsAuth } from "./creds";

const apiFetchMock = apiFetch as unknown as Mock;

const credsResult = {
	user_id: "1000",
	user_name: "bot",
};

describe("completeCredsAuth", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("exchanges code/state via the credential callback", async () => {
		apiFetchMock.mockResolvedValue(
			new Response(JSON.stringify(credsResult), { status: 200 }),
		);

		const result = await completeCredsAuth("abc", "cafe");

		expect(apiFetch).toHaveBeenCalledWith(
			"/api/admin/twitch/auth/callback?code=abc&state=cafe",
		);
		expect(result).toEqual(credsResult);
	});

	it("encodes code/state into the query", async () => {
		apiFetchMock.mockResolvedValue(
			new Response(JSON.stringify(credsResult), { status: 200 }),
		);

		await completeCredsAuth("a b", "c/d");

		expect(apiFetch).toHaveBeenCalledWith(
			"/api/admin/twitch/auth/callback?code=a%20b&state=c%2Fd",
		);
	});

	it("throws a user-facing message on 401 without a root session", async () => {
		apiFetchMock.mockResolvedValue(new Response("denied", { status: 401 }));
		await expect(completeCredsAuth("abc", "cafe")).rejects.toThrow(
			/сессия истекла/,
		);
	});

	it("throws a user-facing message on 403 without root rights", async () => {
		apiFetchMock.mockResolvedValue(new Response("denied", { status: 403 }));
		await expect(completeCredsAuth("abc", "cafe")).rejects.toThrow(/root/);
	});

	it("throws a user-facing message on 400 (flow never started / retry)", async () => {
		apiFetchMock.mockResolvedValue(new Response("denied", { status: 400 }));
		await expect(completeCredsAuth("abc", "cafe")).rejects.toThrow(/ещё раз/);
	});

	it("throws a generic message on unexpected status", async () => {
		apiFetchMock.mockResolvedValue(new Response("boom", { status: 500 }));
		await expect(completeCredsAuth("abc", "cafe")).rejects.toThrow(/HTTP 500/);
	});
});
