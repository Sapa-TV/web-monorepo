import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";
import { errAsync, okAsync } from "neverthrow";
import { HttpError } from "@sapa-tv-ru/api-client";

vi.mock("#lib/api", () => ({
	api: {
		startTwitchLogin: vi.fn(),
		getMe: vi.fn(),
		createSession: vi.fn(),
		logout: vi.fn(),
		listAdmins: vi.fn(),
		twitchLoginCallback: vi.fn(),
	},
}));

import { api } from "#lib/api";
import {
	completeLogin,
	getSession,
	guardAdmin,
	listAdmins,
	logout,
	startLogin,
} from "./session";
import { ApiErrorKind } from "#lib/internal/api-error";

const apiMock = api as unknown as Record<string, Mock>;

const session = {
	expires_at: "2026-08-15T12:00:00Z",
	is_root: false,
	twitch_user_id: "1000",
	twitch_user_name: "viewer",
};

describe("admin session helpers", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	describe("startLogin", () => {
		it("returns the twitch auth url", async () => {
			apiMock.startTwitchLogin.mockResolvedValue(
				okAsync({ auth_url: "https://id.twitch.tv/oauth2/authorize" }),
			);

			const result = await startLogin();

			expect(result.isOk()).toBe(true);
			expect(result._unsafeUnwrap()).toBe(
				"https://id.twitch.tv/oauth2/authorize",
			);
		});

		it("maps a 500 failure to Server kind", async () => {
			apiMock.startTwitchLogin.mockResolvedValue(
				errAsync(new HttpError(500, "Internal Server Error", null)),
			);

			const result = await startLogin();

			expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.Server);
		});

		it("maps 400 to BadRequest kind (missing twitch config)", async () => {
			apiMock.startTwitchLogin.mockResolvedValue(
				errAsync(new HttpError(400, "Bad Request", null)),
			);

			const result = await startLogin();

			expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.BadRequest);
		});
	});

	describe("completeLogin", () => {
		it("exchanges code/state for a ticket and creates the session", async () => {
			apiMock.twitchLoginCallback.mockResolvedValue(
				okAsync({
					ticket: "ticket-1",
					twitch_user_id: "1000",
					twitch_user_name: "viewer",
				}),
			);
			apiMock.createSession.mockResolvedValue(okAsync(session));

			const result = await completeLogin("abc", "cafe");

			expect(api.twitchLoginCallback).toHaveBeenCalledWith({
				code: "abc",
				state: "cafe",
			});
			expect(api.createSession).toHaveBeenCalledWith({ ticket: "ticket-1" });
			expect(result._unsafeUnwrap()).toEqual(session);
		});

		it("fails with BadRequest when the twitch callback errors", async () => {
			apiMock.twitchLoginCallback.mockResolvedValue(
				errAsync(new HttpError(400, "Bad Request", null)),
			);

			const result = await completeLogin("abc", "cafe");

			expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.BadRequest);
			expect(api.createSession).not.toHaveBeenCalled();
		});

		it("propagates the ticket exchange failure", async () => {
			apiMock.twitchLoginCallback.mockResolvedValue(
				okAsync({
					ticket: "ticket-1",
					twitch_user_id: "1000",
					twitch_user_name: "viewer",
				}),
			);
			apiMock.createSession.mockResolvedValue(
				errAsync(new HttpError(400, "Bad Request", null)),
			);

			const result = await completeLogin("abc", "cafe");

			expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.BadRequest);
		});
	});

	describe("getSession", () => {
		it("returns the session when logged in", async () => {
			apiMock.getMe.mockResolvedValue(okAsync(session));
			await expect(getSession()).resolves.toEqual(session);
		});

		it("returns null when not logged in", async () => {
			apiMock.getMe.mockResolvedValue(
				errAsync(new HttpError(401, "Unauthorized", null)),
			);
			await expect(getSession()).resolves.toBeNull();
		});
	});

	describe("guardAdmin", () => {
		it("returns not-logged-in without a session", async () => {
			apiMock.getMe.mockResolvedValue(
				errAsync(new HttpError(401, "Unauthorized", null)),
			);

			const result = await guardAdmin();

			expect(result._unsafeUnwrap()).toEqual({
				status: "not-logged-in",
				isRoot: false,
			});
		});

		it("returns not-admin when /api/admin is forbidden", async () => {
			apiMock.getMe.mockResolvedValue(okAsync(session));
			apiMock.listAdmins.mockResolvedValue(
				errAsync(new HttpError(403, "Forbidden", null)),
			);

			const result = await guardAdmin();

			expect(result._unsafeUnwrap()).toEqual({
				status: "not-admin",
				isRoot: false,
			});
		});

		it("returns admin + isRoot for the root user", async () => {
			apiMock.getMe.mockResolvedValue(okAsync({ ...session, is_root: true }));
			apiMock.listAdmins.mockResolvedValue(
				okAsync([
					{
						twitch_id: "1000",
						display_name: "root",
						is_root: true,
						created_at: "x",
					},
				]),
			);

			const result = await guardAdmin();

			expect(result._unsafeUnwrap()).toEqual({
				status: "admin",
				isRoot: true,
			});
		});

		it("errs with Server kind on unexpected http failures", async () => {
			apiMock.getMe.mockResolvedValue(okAsync(session));
			apiMock.listAdmins.mockResolvedValue(
				errAsync(new HttpError(503, "Service Unavailable", null)),
			);

			const result = await guardAdmin();

			expect(result.isErr()).toBe(true);
			expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.Server);
		});
	});

	describe("logout", () => {
		it("calls the logout endpoint", async () => {
			apiMock.logout.mockResolvedValue(okAsync(undefined));
			await logout();
			expect(api.logout).toHaveBeenCalledOnce();
		});
	});

	describe("listAdmins", () => {
		it("returns the admin list", async () => {
			apiMock.listAdmins.mockResolvedValue(
				okAsync([
					{
						twitch_id: "1000",
						display_name: "viewer",
						is_root: false,
						created_at: "x",
					},
				]),
			);

			const result = await listAdmins();

			const admins = result._unsafeUnwrap();
			expect(admins).toHaveLength(1);
			expect(admins[0].twitch_id).toBe("1000");
		});

		it("maps failures to Server kind", async () => {
			apiMock.listAdmins.mockResolvedValue(
				errAsync(new HttpError(503, "Service Unavailable", null)),
			);

			const result = await listAdmins();

			expect(result._unsafeUnwrapErr().kind).toBe(ApiErrorKind.Server);
		});
	});
});
