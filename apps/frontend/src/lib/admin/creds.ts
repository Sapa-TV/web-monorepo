import { apiFetch } from "#lib/api";
import type { TwitchAuthCallbackResponse } from "@sapa-tv-ru/api-client";

const UNAUTHORIZED = 401;
const FORBIDDEN = 403;
const BAD_REQUEST = 400;

export async function completeCredsAuth(
	code: string,
	state: string,
): Promise<TwitchAuthCallbackResponse> {
	const res = await apiFetch(
		`/api/admin/twitch/auth/callback?code=${encodeURIComponent(code)}&state=${encodeURIComponent(state)}`,
	);
	if (!res.ok) {
		if (res.status === UNAUTHORIZED || res.status === FORBIDDEN) {
			throw new Error(
				"Нет доступа: сессия истекла или у аккаунта нет прав root. Вернись на панель, перелогинься и попробуй снова.",
			);
		}
		if (res.status === BAD_REQUEST) {
			throw new Error(
				"Не удалось завершить авторизацию: попробуй на панели «Авторизовать» ещё раз.",
			);
		}
		throw new Error(`Twitch callback failed: HTTP ${res.status}`);
	}
	return (await res.json()) as TwitchAuthCallbackResponse;
}
