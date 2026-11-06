import { ResultAsync, errAsync, okAsync } from "neverthrow";

/**
 * Autoplay is a browser media API that fails via rejection; normalize here
 * so callers stay inside the Result world.
 */
export type AutoplayOutcome = "playing" | "blocked";

export async function tryAutoplay(player: {
	play: () => Promise<void> | void;
}): Promise<AutoplayOutcome> {
	try {
		await player.play();
		return "playing";
	} catch {
		return "blocked";
	}
}

export type StreamPlayer = {
	attachMediaElement(_element: HTMLVideoElement): void;
	load(): void;
	play(): Promise<void> | void;
	destroy(): void;
};

export type StreamPlayerFailure = "unsupported" | "load_failed";

export function loadStreamPlayer(
	element: HTMLVideoElement,
	url: string,
): ResultAsync<StreamPlayer, StreamPlayerFailure> {
	return ResultAsync.fromPromise(
		import("mpegts.js"),
		() => "load_failed" as const,
	).andThen(({ default: mpegts }) => {
		if (!mpegts.isSupported()) {
			return errAsync<StreamPlayer, StreamPlayerFailure>("unsupported");
		}
		const player = mpegts.createPlayer(
			{ type: "flv", isLive: true, url },
			{ enableStashBuffer: false, liveBufferLatencyChasing: true },
		);
		player.attachMediaElement(element);
		player.load();
		return okAsync<StreamPlayer, StreamPlayerFailure>(player);
	});
}
