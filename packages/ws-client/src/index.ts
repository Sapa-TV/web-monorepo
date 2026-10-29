export type WsRole = "dock" | "widget";

export type WsClientMessage =
	| { type: "auth"; token: string; role: WsRole }
	| { type: "complete"; entry_id: number };

export type WsServerMessage =
	| { type: "auth_ok" }
	| { type: "auth_err" }
	| { type: "complete_ok"; entry_id: number }
	| { type: "complete_err"; entry_id: number; error: string }
	| { type: "presence"; dock?: boolean; widget_count?: number }
	| {
			type: "spin_started";
			entry_id?: number;
			slot_id?: number;
			slot_name?: string;
			slot_rarity?: string;
			user_name?: string;
	  }
	| { type: "spin_completed"; entry_id?: number }
	| { type: "spin_error"; entry_id?: number };

export interface SocketLike {
	send(data: string): void;
	close(): void;
	onopen: () => void;
	onclose: () => void;
	onerror: () => void;
	onmessage: (e: { data: string }) => void;
}

export type WsState = "connected" | "disconnected";

export interface WsClientOptions {
	url: string;
	token: string;
	role: WsRole;
	onMessage: (message: WsServerMessage) => void;
	onState?: (state: WsState) => void;
	onAuthError?: () => void;
	socketFactory?: new (url: string) => SocketLike;
	retryBaseMs?: number;
	retryMaxMs?: number;
}

export interface WsClient {
	send(message: WsClientMessage): void;
	close(): void;
}

const DEFAULT_RETRY_BASE_MS = 1_000;
const DEFAULT_RETRY_MAX_MS = 15_000;

export function connectWs(options: WsClientOptions): WsClient {
	const factory =
		options.socketFactory ??
		(globalThis.WebSocket as unknown as new (url: string) => SocketLike);
	const retryBaseMs = options.retryBaseMs ?? DEFAULT_RETRY_BASE_MS;
	const retryMaxMs = options.retryMaxMs ?? DEFAULT_RETRY_MAX_MS;
	let retryMs = retryBaseMs;
	let socket: SocketLike | null = null;
	let rejected = false;
	let closed = false;

	const connect = (): void => {
		if (closed || rejected) return;
		const ws = new factory(options.url);
		socket = ws;
		ws.onopen = () => {
			options.onState?.("connected");
			ws.send(
				JSON.stringify({
					type: "auth",
					token: options.token,
					role: options.role,
				}),
			);
		};
		ws.onmessage = (e) => {
			let parsed: WsServerMessage;
			try {
				parsed = JSON.parse(e.data) as WsServerMessage;
			} catch {
				console.warn("ws-client: malformed message dropped");
				return;
			}
			if (parsed.type === "auth_ok") retryMs = retryBaseMs;
			if (parsed.type === "auth_err") {
				rejected = true;
				options.onMessage(parsed);
				options.onAuthError?.();
				return;
			}
			options.onMessage(parsed);
		};
		ws.onclose = () => {
			socket = null;
			options.onState?.("disconnected");
			if (!closed && !rejected) {
				setTimeout(connect, retryMs);
				retryMs = Math.min(retryMs * 2, retryMaxMs);
			}
		};
		ws.onerror = () => ws.close();
	};

	connect();

	return {
		send: (message) => {
			socket?.send(JSON.stringify(message));
		},
		close: () => {
			closed = true;
			if (socket) {
				const target = socket;
				target.onclose = () => {};
				target.close();
				socket = null;
			}
		},
	};
}
