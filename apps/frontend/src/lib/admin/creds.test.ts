import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";
import { errAsync, okAsync } from "neverthrow";
import { HttpError } from "@sapa-tv-ru/api-client";

vi.mock("#lib/api", () => ({
	api: {
		twitchAuthCallback: vi.fn(),
	},
}));

import { api } from "#lib/api";
import { completeCredsAuth } from "./creds";

const apiMock = api as unknown as Record<string, Mock>;

const credsResult = {
	user_id: "1000",
	user_name: "bot",
};

describe("completeCredsAuth", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("exchanges code/state via the credential callback", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(okAsync(credsResult));

		const result = await completeCredsAuth("abc", "cafe");

		expect(api.twitchAuthCallback).toHaveBeenCalledWith({
			code: "abc",
			state: "cafe",
		});
		expect(result).toEqual(credsResult);
	});

	it("passes code/state into the query as-is", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(okAsync(credsResult));

		await completeCredsAuth("a b", "c/d");

		expect(api.twitchAuthCallback).toHaveBeenCalledWith({
			code: "a b",
			state: "c/d",
		});
	});

	it("throws a user-facing message on 401 without a root session", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(
			errAsync(new HttpError(401, "Unauthorized", null)),
		);
		await expect(completeCredsAuth("abc", "cafe")).rejects.toThrow(
			/сессия истекла/,
		);
	});

	it("throws a user-facing message on 403 without root rights", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(
			errAsync(new HttpError(403, "Forbidden", null)),
		);
		await expect(completeCredsAuth("abc", "cafe")).rejects.toThrow(/root/);
	});

	it("throws a user-facing message on 400 (flow never started / retry)", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(
			errAsync(new HttpError(400, "Bad Request", null)),
		);
		await expect(completeCredsAuth("abc", "cafe")).rejects.toThrow(/ещё раз/);
	});

	it("throws a generic message on unexpected status", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(
			errAsync(new HttpError(500, "Internal Server Error", null)),
		);
		await expect(completeCredsAuth("abc", "cafe")).rejects.toThrow(/HTTP 500/);
	});
});
