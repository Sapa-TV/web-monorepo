import { api } from "#lib/api";
import type { Result } from "neverthrow";

import { ApiError, normalizeApiError } from "#lib/internal/api-error";

export const CredsPlatform = {
	Twitch: "twitch",
	VkVideoLive: "vk_video_live",
} as const;

export type CredsPlatform = (typeof CredsPlatform)[keyof typeof CredsPlatform];

export interface ConnectedUser {
	user_id: string;
	user_name: string | null;
}

/**
 * Exchanges the OAuth code/state for bot credentials.
 * Human-readable messages live in the calling route, keyed by `error.kind`.
 */
export async function completeCredsAuth(
	platform: CredsPlatform,
	code: string,
	state: string,
): Promise<Result<ConnectedUser, ApiError>> {
	if (platform === CredsPlatform.VkVideoLive) {
		const res = await api.vkVideoLiveAuthCallback({ code, state });
		return res
			.map((r) => ({ user_id: r.user_id, user_name: r.user_name }))
			.mapErr(normalizeApiError);
	}
	const res = await api.twitchAuthCallback({ code, state });
	return res
		.map((r) => ({ user_id: r.user_id, user_name: r.user_name ?? null }))
		.mapErr(normalizeApiError);
}
