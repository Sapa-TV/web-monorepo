/**
 * Autoplay is a browser media API that fails via rejection; normalize here
 * so callers stay inside the Result world.
 */
export type AutoplayOutcome = "playing" | "blocked";

export async function tryAutoplay(player: {
	play: () => Promise<void>;
}): Promise<AutoplayOutcome> {
	try {
		await player.play();
		return "playing";
	} catch {
		return "blocked";
	}
}
