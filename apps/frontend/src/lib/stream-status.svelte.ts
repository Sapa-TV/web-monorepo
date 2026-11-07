import { api } from "#lib/api";

export type StreamPhase = "unknown" | "online" | "offline";

export const STREAM_STATUS_POLL_MS = 10_000;

export function streamPhaseFrom(online: boolean): StreamPhase {
	return online ? "online" : "offline";
}

async function fetchOnline(): Promise<boolean | undefined> {
	const result = await api.getStreamStatus();
	return result.match(
		(body) => body.online,
		() => undefined,
	);
}

export interface StreamStatusStoreOptions {
	pollIntervalMs?: number;
	fetchOnline?: () => Promise<boolean | undefined>;
}

export class StreamStatusStore {
	phase = $state<StreamPhase>("unknown");

	private readonly pollIntervalMs: number;
	private readonly fetchOnline: () => Promise<boolean | undefined>;
	private timer: ReturnType<typeof setInterval> | undefined;

	constructor(options: StreamStatusStoreOptions = {}) {
		this.pollIntervalMs = options.pollIntervalMs ?? STREAM_STATUS_POLL_MS;
		this.fetchOnline = options.fetchOnline ?? fetchOnline;
	}

	start(): void {
		if (this.timer !== undefined) return;
		void this.refresh();
		this.timer = setInterval(() => void this.refresh(), this.pollIntervalMs);
	}

	stop(): void {
		clearInterval(this.timer);
		this.timer = undefined;
	}

	async refresh(): Promise<void> {
		const online = await this.fetchOnline().catch(() => undefined);
		if (online === undefined) return;
		this.phase = streamPhaseFrom(online);
	}
}
