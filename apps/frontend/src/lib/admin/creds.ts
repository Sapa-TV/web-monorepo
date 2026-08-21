import { api } from "#lib/api";
import {
	HttpError,
	type TwitchAuthCallbackResponse,
} from "@sapa-tv-ru/api-client";

const UNAUTHORIZED = 401;
const FORBIDDEN = 403;
const BAD_REQUEST = 400;

export async function completeCredsAuth(
	code: string,
	state: string,
): Promise<TwitchAuthCallbackResponse> {
	const res = await api.twitchAuthCallback("", "", {
		query: { code, state },
	});
	if (res.isErr()) {
		const err = res.error;
		if (err instanceof HttpError) {
			if (err.status === UNAUTHORIZED || err.status === FORBIDDEN) {
				throw new Error(
					"Нет доступа: сессия истекла или у аккаунта нет прав root. Вернись на панель, перелогинься и попробуй снова.",
				);
			}
			if (err.status === BAD_REQUEST) {
				throw new Error(
					"Не удалось завершить авторизацию: попробуй на панели «Авторизовать» ещё раз.",
				);
			}
			throw new Error(`Twitch callback failed: HTTP ${err.status}`);
		}
		throw err;
	}
	return res.value;
}
