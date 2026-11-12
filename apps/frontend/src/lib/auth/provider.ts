import { api } from "#lib/api";
import type {
	LinkedPlatformResponse,
	SessionResponse,
} from "@sapa-tv-ru/api-client";
import { type Result, errAsync, okAsync } from "neverthrow";

import {
	ApiError,
	ApiErrorKind,
	normalizeApiError,
} from "#lib/internal/api-error";

export const Provider = {
	Twitch: "twitch",
	VkVideoLive: "vk_video_live",
} as const;

export type Provider = (typeof Provider)[keyof typeof Provider];

export const PROVIDERS: Provider[] = [Provider.Twitch, Provider.VkVideoLive];

export function providerLabel(provider: Provider): string {
	return provider === Provider.VkVideoLive ? "VK Видео Live" : "Twitch";
}

export type FinishResult =
	| { kind: "session"; session: SessionResponse }
	| { kind: "merge"; ticket: string };

export async function startProvider(
	provider: Provider,
): Promise<Result<string, ApiError>> {
	const res =
		provider === Provider.VkVideoLive
			? await api.startVkVideoLiveLogin()
			: await api.startTwitchLogin();
	return res.map((r) => r.auth_url).mapErr(normalizeApiError);
}

async function getCurrentSession(): Promise<
	Result<SessionResponse | null, ApiError>
> {
	const res = await api.getMe();
	if (res.isOk()) return okAsync(res.value);
	const error = normalizeApiError(res.error);
	if (error.kind === ApiErrorKind.Unauthorized) return okAsync(null);
	return errAsync(error);
}

export async function finishProvider(
	provider: Provider,
	code: string,
	state: string,
): Promise<Result<FinishResult, ApiError>> {
	const cb =
		provider === Provider.VkVideoLive
			? await api.vkVideoLiveLoginCallback({ code, state })
			: await api.twitchLoginCallback({ code, state });
	if (cb.isErr()) return errAsync(normalizeApiError(cb.error));

	const current = await getCurrentSession();
	if (current.isErr()) return errAsync(current.error);
	if (current.value && !current.value.is_guest) {
		const linked = await api.linkSession({
			ticket: cb.value.ticket,
			merge: false,
		});
		if (linked.isOk())
			return okAsync({ kind: "session", session: linked.value });
		const error = normalizeApiError(linked.error);
		if (error.kind === ApiErrorKind.Conflict) {
			return okAsync({ kind: "merge", ticket: cb.value.ticket });
		}
		return errAsync(error);
	}

	const session = await api.createSession({ ticket: cb.value.ticket });
	return session
		.map((value) => ({ kind: "session" as const, session: value }))
		.mapErr(normalizeApiError);
}

export async function confirmMerge(
	ticket: string,
): Promise<Result<SessionResponse, ApiError>> {
	const res = await api.linkSession({ ticket, merge: true });
	return res.mapErr(normalizeApiError);
}

export async function getCurrentAccount(): Promise<
	Result<SessionResponse | null, ApiError>
> {
	return getCurrentSession();
}

export async function getLinkedPlatforms(): Promise<
	Result<LinkedPlatformResponse[], ApiError>
> {
	const res = await api.getLinks();
	return res.mapErr(normalizeApiError);
}
