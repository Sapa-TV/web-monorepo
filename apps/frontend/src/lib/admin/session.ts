import { api } from "#lib/api";
import type { AdminResponse, SessionResponse } from "@sapa-tv-ru/api-client";
import { type Result, err, errAsync, okAsync } from "neverthrow";

import {
	ApiError,
	ApiErrorKind,
	normalizeApiError,
} from "#lib/internal/api-error";

export const GuardStatus = {
	Admin: "admin",
	NotLoggedIn: "not-logged-in",
	NotAdmin: "not-admin",
} as const;

export type GuardStatus = (typeof GuardStatus)[keyof typeof GuardStatus];

export interface AdminGuard {
	status: GuardStatus;
	isRoot: boolean;
}

export async function startLogin(): Promise<Result<string, ApiError>> {
	const res = await api.startTwitchLogin();
	return res.map((r) => r.auth_url).mapErr(normalizeApiError);
}

export async function completeLogin(
	code: string,
	state: string,
): Promise<Result<SessionResponse, ApiError>> {
	const cb = await api.twitchLoginCallback({ code, state });
	const normalized = cb.mapErr(normalizeApiError);
	if (normalized.isErr()) {
		return err(normalized.error);
	}
	const session = await api.createSession({
		ticket: normalized.value.ticket,
	});
	return session.mapErr(normalizeApiError);
}

export async function getSession(): Promise<SessionResponse | null> {
	const res = await api.getMe();
	if (res.isErr()) return null;
	return res.value;
}

export async function logout(): Promise<void> {
	await api.logout();
}

/**
 * Classifies the admin session. Returns Err only for unexpected failures
 * (network problems, 5xx); 401/403 are expressed as guard statuses.
 */
export async function guardAdmin(): Promise<Result<AdminGuard, ApiError>> {
	const session = await getSession();
	if (!session)
		return okAsync({ status: GuardStatus.NotLoggedIn, isRoot: false });

	const res = await api.listAdmins();
	if (res.isOk()) {
		return okAsync({ status: GuardStatus.Admin, isRoot: session.is_root });
	}

	const err = normalizeApiError(res.error);
	switch (err.kind) {
		case ApiErrorKind.Unauthorized:
			return okAsync({ status: GuardStatus.NotLoggedIn, isRoot: false });
		case ApiErrorKind.Forbidden:
			return okAsync({ status: GuardStatus.NotAdmin, isRoot: false });
		default:
			return errAsync(err);
	}
}

export async function listAdmins(): Promise<Result<AdminResponse[], ApiError>> {
	const res = await api.listAdmins();
	return res.mapErr(normalizeApiError);
}
