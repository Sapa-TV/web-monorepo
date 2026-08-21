import { createApi, createWapi } from "@sapa-tv-ru/api-client";

const defaultOrigin =
	typeof location !== "undefined"
		? `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}`
		: "";

export const API_BASE = "";

export const WAPI_BASE = "/wapi";

const httpOrigin = typeof location !== "undefined" ? location.origin : "";

export const api = createApi(httpOrigin);

export const wapi = createWapi(httpOrigin);

export const WS_URL = `${defaultOrigin}${WAPI_BASE}/ws`;

export type { QueueStats } from "@sapa-tv-ru/api-client";
export type { QueueEntryResponse as QueueEntry } from "@sapa-tv-ru/api-client";

export function apiFetch(
	path: string,
	init: RequestInit = {},
	widgetAccessKey = "",
) {
	const headers = new Headers(init.headers);
	if (widgetAccessKey)
		headers.set("Authorization", `Bearer ${widgetAccessKey}`);
	return fetch(`${API_BASE}${path}`, {
		...init,
		headers,
		credentials: "include",
	});
}
