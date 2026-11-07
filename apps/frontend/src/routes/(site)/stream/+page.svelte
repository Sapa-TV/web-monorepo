<script lang="ts">
	import { onMount } from "svelte";
	import IconMessageCircle from "~icons/lucide/message-circle";
	import IconVideoOff from "~icons/lucide/video-off";
	import {
		loadStreamPlayer,
		tryAutoplay,
		type StreamPlayer,
	} from "#lib/internal/media";
	import { StreamStatusStore } from "#lib/stream-status.svelte";

	const STREAM_URL = "https://sapa-tv.ru/live/stream1.flv";
	const FAILURE_TEXT = {
		unsupported: "Браузер не поддерживает воспроизведение",
		load_failed: "Не удалось загрузить плеер",
	};

	const stream = new StreamStatusStore();

	let video: HTMLVideoElement | undefined = $state();
	let status = $state("");

	onMount(() => {
		stream.start();
		return () => stream.stop();
	});

	$effect(() => {
		if (stream.phase !== "online" || !video) return;

		let player: StreamPlayer | undefined;
		let destroyed = false;
		status = "";

		void loadStreamPlayer(video, STREAM_URL).then((result) => {
			if (result.isErr()) {
				status = FAILURE_TEXT[result.error];
				return;
			}
			if (destroyed) {
				result.value.destroy();
				return;
			}
			player = result.value;
			void tryAutoplay(player).then((outcome) => {
				if (outcome === "blocked") {
					console.warn("Autoplay blocked: требуется клик по видео.");
				}
			});
		});

		return () => {
			destroyed = true;
			player?.destroy();
		};
	});
</script>

<svelte:head>
	<title>Онлайн Трансляция</title>
</svelte:head>

<div class="page">
	<main class="tuner">
		<div class="video-shell">
			{#if stream.phase === "online"}
				<video bind:this={video} controls autoplay muted playsinline></video>
				{#if status}
					<span class="status">{status}</span>
				{/if}
			{:else if stream.phase === "offline"}
				<div class="placeholder" role="status">
					<IconVideoOff aria-hidden="true" />
					<p class="placeholder-title">Сейчас офлайн</p>
					<p class="placeholder-text">
						Следите за анонсами — скоро вернёмся в эфир.
					</p>
				</div>
			{:else}
				<div class="placeholder" role="status">
					<span class="pulse" aria-hidden="true"></span>
					<p class="placeholder-text">Проверяем эфир…</p>
				</div>
			{/if}
		</div>
	</main>

	<aside class="chat-sidebar">
		<button
			class="chat-vk-btn"
			type="button"
			onclick={() =>
				window.open(
					"https://live.vkvideo.ru/sapushka_/stream/default/only-chat",
					"_blank",
				)}
		>
			<IconMessageCircle aria-hidden="true" />
			VK чат
		</button>
		<div class="chat-panel">
			<iframe
				class="active"
				title="Твитч чат"
				src="https://www.twitch.tv/embed/sapushka_/chat?parent=sapa-tv.ru&amp;darkpopout"
				allowfullscreen
				sandbox="allow-modals allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox allow-forms"
			></iframe>
		</div>
	</aside>
</div>

<style>
	.page {
		min-height: 0;
		display: flex;
		height: calc(100vh - var(--site-nav-h));
		overflow: hidden;
	}

	.tuner {
		width: 100%;
		max-width: 100%;
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.status {
		position: absolute;
		top: 0.6rem;
		left: 0.6rem;
		z-index: 2;
		font-size: 0.8rem;
		color: var(--on-surface-variant);
		background: color-mix(in oklch, var(--surface) 70%, transparent);
		padding: 0.15rem 0.5rem;
		border-radius: 6px;
	}

	.video-shell {
		flex: 1;
		min-height: 0;
		position: relative;
		background-color: var(--video-bg);
		overflow: hidden;
		border: 1px solid var(--outline-variant);
	}

	.placeholder {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 1.5rem;
		text-align: center;
		color: var(--on-surface-variant);
	}

	.placeholder :global(svg) {
		width: 2.5rem;
		height: 2.5rem;
		opacity: 0.7;
	}

	.placeholder-title {
		margin: 0;
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--on-surface);
	}

	.placeholder-text {
		margin: 0;
		font-size: 0.9rem;
		max-width: 24rem;
	}

	.pulse {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
		background: var(--primary);
		animation: pulse 1.4s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 0.25;
		}
		50% {
			opacity: 1;
		}
	}

	video {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		outline: none;
	}

	.chat-sidebar {
		width: 360px;
		flex-shrink: 0;
		border-left: 1px solid var(--outline-variant);
		background-color: var(--surface);
		position: relative;
		z-index: 999;
		isolation: isolate;
		padding: 0.6rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}

	.chat-vk-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.55rem 0.75rem;
		border: 1px solid var(--outline-variant);
		background-color: var(--surface-container);
		color: var(--on-surface);
		font: inherit;
		font-size: 0.85rem;
		font-weight: 600;
		cursor: pointer;
		flex-shrink: 0;
		transition:
			background-color 0.15s,
			border-color 0.15s;
	}

	.chat-vk-btn:hover {
		background-color: var(--secondary-container);
		border-color: var(--outline);
	}

	.chat-panel {
		flex: 1;
		min-height: 0;
	}

	.chat-sidebar iframe {
		width: 100%;
		height: 100%;
		border: none;
		display: block;
		overflow: hidden;
	}

	@media (max-width: 768px) {
		.page {
			height: auto;
			overflow: visible;
			flex-direction: column;
		}

		.tuner {
			height: auto;
		}

		.video-shell {
			flex: none;
			width: 100%;
			aspect-ratio: 16 / 9;
		}

		.chat-sidebar {
			width: 100%;
			height: clamp(360px, 45vh, 520px);
			border-left: none;
			border-top: 1px solid var(--outline-variant);
		}
	}
</style>
