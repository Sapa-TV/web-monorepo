import { api } from "#lib/api";
import type { TwitchAuthCallbackResponse } from "@sapa-tv-ru/api-client";
import type { Result } from "neverthrow";

import { ApiError, normalizeApiError } from "#lib/internal/api-error";

/**
 * Exchanges the OAuth code/state for bot credentials.
 * Human-readable messages live in the calling route, keyed by `error.kind`.
 */
export async function completeCredsAuth(
	code: string,
	state: string,
): Promise<Result<TwitchAuthCallbackResponse, ApiError>> {
	const res = await api.twitchAuthCallback({ code, state });
	return res.mapErr(normalizeApiError);
}
