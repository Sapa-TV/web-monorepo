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
			await expect(startLogin()).resolves.toBe(
				"https://id.twitch.tv/oauth2/authorize",
			);
		});

		it("throws when the start call fails", async () => {
			apiMock.startTwitchLogin.mockResolvedValue(
				errAsync(new HttpError(500, "Internal Server Error", null)),
			);
			await expect(startLogin()).rejects.toThrow();
		});

		it("throws a user-facing error on 400 (missing twitch config)", async () => {
			apiMock.startTwitchLogin.mockResolvedValue(
				errAsync(new HttpError(400, "Bad Request", null)),
			);
			await expect(startLogin()).rejects.toThrow();
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
			expect(result).toEqual(session);
		});

		it("throws when the twitch callback returns an error", async () => {
			apiMock.twitchLoginCallback.mockResolvedValue(
				errAsync(new HttpError(400, "Bad Request", null)),
			);

			await expect(completeLogin("abc", "cafe")).rejects.toThrow(/HTTP 400/);
			expect(api.createSession).not.toHaveBeenCalled();
		});

		it("throws when the ticket exchange fails", async () => {
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

			await expect(completeLogin("abc", "cafe")).rejects.toThrow();
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
			await expect(guardAdmin()).resolves.toEqual({
				status: "not-logged-in",
				isRoot: false,
			});
		});

		it("returns not-admin when /api/admin is forbidden", async () => {
			apiMock.getMe.mockResolvedValue(okAsync(session));
			apiMock.listAdmins.mockResolvedValue(
				errAsync(new HttpError(403, "Forbidden", null)),
			);
			await expect(guardAdmin()).resolves.toEqual({
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
			await expect(guardAdmin()).resolves.toEqual({
				status: "admin",
				isRoot: true,
			});
		});

		it("rethrows non-http errors", async () => {
			apiMock.getMe.mockResolvedValue(okAsync(session));
			apiMock.listAdmins.mockResolvedValue(errAsync(new Error("boom")));
			await expect(guardAdmin()).rejects.toThrow("boom");
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
			const admins = await listAdmins();
			expect(admins).toHaveLength(1);
			expect(admins[0].twitch_id).toBe("1000");
		});

		it("throws when the request fails", async () => {
			apiMock.listAdmins.mockResolvedValue(
				errAsync(new HttpError(503, "Service Unavailable", null)),
			);
			await expect(listAdmins()).rejects.toThrow();
		});
	});
});
