export const API_BASE = "http://localhost:3000";

export const WS_URL = "ws://localhost:3000/ws";

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
	return fetch(`${API_BASE}${path}`, { ...init, headers, credentials: "include" });
}
