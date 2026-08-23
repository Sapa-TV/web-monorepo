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
import { ApiErrorKind } from "#lib/internal/api-error";

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
		expect(result.isOk()).toBe(true);
		expect(result._unsafeUnwrap()).toEqual(credsResult);
	});

	it("passes code/state into the query as-is", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(okAsync(credsResult));

		await completeCredsAuth("a b", "c/d");

		expect(api.twitchAuthCallback).toHaveBeenCalledWith({
			code: "a b",
			state: "c/d",
		});
	});

	it("maps 401 to Unauthorized kind", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(
			errAsync(new HttpError(401, "Unauthorized", null)),
		);

		const result = await completeCredsAuth("abc", "cafe");

		expect(result.isErr()).toBe(true);
		expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.Unauthorized);
	});

	it("maps 403 to Forbidden kind", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(
			errAsync(new HttpError(403, "Forbidden", null)),
		);

		const result = await completeCredsAuth("abc", "cafe");

		expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.Forbidden);
	});

	it("maps 400 to BadRequest kind", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(
			errAsync(new HttpError(400, "Bad Request", null)),
		);

		const result = await completeCredsAuth("abc", "cafe");

		expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.BadRequest);
	});

	it("maps 500 to Server kind while keeping the status", async () => {
		apiMock.twitchAuthCallback.mockResolvedValue(
			errAsync(new HttpError(500, "Internal Server Error", null)),
		);

		const result = await completeCredsAuth("abc", "cafe");

		const err = result._unsafeUnwrapErr();
		expect(err.kind).toBe(ApiErrorKind.Server);
		expect(err.status).toBe(500);
	});
});
