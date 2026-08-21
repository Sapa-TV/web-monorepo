import { createApi, createWapi } from "@sapa-tv-ru/api-client";

const defaultOrigin =
	typeof location !== "undefined"
		? `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}`
		: "";

export const WAPI_BASE = "/wapi";

const httpOrigin = typeof location !== "undefined" ? location.origin : "";

export const api = createApi(httpOrigin);

export const wapi = createWapi(httpOrigin);

export const WS_URL = `${defaultOrigin}${WAPI_BASE}/ws`;

export type { QueueStats } from "@sapa-tv-ru/api-client";
export type { QueueEntryResponse as QueueEntry } from "@sapa-tv-ru/api-client";
