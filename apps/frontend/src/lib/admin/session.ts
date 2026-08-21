import { api } from "#lib/api";
import {
	HttpError,
	type AdminResponse,
	type SessionResponse,
} from "@sapa-tv-ru/api-client";

const UNAUTHORIZED = 401;
const FORBIDDEN = 403;
const BAD_REQUEST = 400;

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

export async function startLogin(): Promise<string> {
	const res = await api.startTwitchLogin();
	if (res.isErr()) {
		const err = res.error;
		if (err instanceof HttpError && err.status === BAD_REQUEST) {
			throw new Error("Ошибка сервера: отсутствует twitch config");
		}
		throw err;
	}
	return res.value.auth_url;
}

export async function completeLogin(
	code: string,
	state: string,
): Promise<SessionResponse> {
	const cbRes = await api.twitchLoginCallback("", "", {
		query: { code, state },
	});
	if (cbRes.isErr()) {
		const err = cbRes.error;
		if (err instanceof HttpError) {
			throw new Error(`Twitch callback failed: HTTP ${err.status}`);
		}
		throw err;
	}
	const sessionRes = await api.createSession({ ticket: cbRes.value.ticket });
	if (sessionRes.isErr()) throw sessionRes.error;
	return sessionRes.value;
}

export async function getSession(): Promise<SessionResponse | null> {
	const res = await api.getMe();
	if (res.isErr()) return null;
	return res.value;
}

export async function logout(): Promise<void> {
	await api.logout();
}

export async function guardAdmin(): Promise<AdminGuard> {
	const session = await getSession();
	if (!session) return { status: GuardStatus.NotLoggedIn, isRoot: false };
	const res = await api.listAdmins();
	if (res.isErr()) {
		const err = res.error;
		if (err instanceof HttpError) {
			if (err.status === UNAUTHORIZED) {
				return { status: GuardStatus.NotLoggedIn, isRoot: false };
			}
			if (err.status === FORBIDDEN) {
				return { status: GuardStatus.NotAdmin, isRoot: false };
			}
		}
		throw err;
	}
	return { status: GuardStatus.Admin, isRoot: session.is_root };
}

export async function listAdmins(): Promise<AdminResponse[]> {
	const res = await api.listAdmins();
	if (res.isErr()) throw res.error;
	return res.value;
}
