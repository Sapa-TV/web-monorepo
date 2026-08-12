import { createApi } from "@sapa-tv-ru/api-client";

const defaultOrigin =
	typeof location !== "undefined"
		? `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}`
		: "";

export const API_BASE = "";

const httpOrigin = typeof location !== "undefined" ? location.origin : "";

export const api = createApi(httpOrigin);

export const WS_URL = `${defaultOrigin}/ws`;

export interface QueueEntry {
	id: number;
	user_id: string;
	user_name: string;
	status: "Pending" | "Spinning" | "Completed" | "Cancelled" | "Error";
	result_slot_id: number | null;
	slot_name: string | null;
	created_at: string;
	updated_at: string;
}

export interface QueueStats {
	pending: number;
	spinning: number;
	completed: number;
	error: number;
	cancelled: number;
}

export function apiFetch(path: string, init: RequestInit = {}, pak = "") {
	const headers = new Headers(init.headers);
	if (pak) headers.set("Authorization", `Bearer ${pak}`);
	return fetch(`${API_BASE}${path}`, {
		...init,
		headers,
		credentials: "include",
	});
}
