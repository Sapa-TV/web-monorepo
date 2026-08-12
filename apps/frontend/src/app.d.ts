// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
import "unplugin-icons/types/svelte";

declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	interface Window {
		mpegts?: {
			isSupported(): boolean;
			createPlayer(
				_config: { type: "flv"; isLive: boolean; url: string },
				_opts?: {
					enableStashBuffer?: boolean;
					liveBufferLatencyChasing?: boolean;
				},
			): {
				attachMediaElement(_element: HTMLVideoElement): void;
				load(): void;
				play(): Promise<void>;
				destroy(): void;
			};
		};
	}
}

export {};
