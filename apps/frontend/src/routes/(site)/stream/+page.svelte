<script lang="ts">
	import { onMount } from "svelte";
	import IconMessageCircle from "~icons/lucide/message-circle";

	let video: HTMLVideoElement | undefined = $state();
	let status = $state("");

	const MAX_RETRIES = 50;
	const RETRY_INTERVAL_MS = 200;

	onMount(() => {
		let player: { destroy: () => void } | undefined;
		let retries = 0;

		function tryStart() {
			if (!video) return;
			if (typeof window.mpegts !== "undefined" && window.mpegts.isSupported()) {
				const instance = window.mpegts.createPlayer(
					{
						type: "flv",
						isLive: true,
						url: "https://sapa-tv.ru/live/stream1.flv",
					},
					{
						enableStashBuffer: false,
						liveBufferLatencyChasing: true,
					},
				);

				instance.attachMediaElement(video);
				instance.load();
				instance.play().catch((err) => console.warn("Autoplay blocked:", err));
				player = instance;
			} else if (retries < MAX_RETRIES) {
				retries += 1;
				setTimeout(tryStart, RETRY_INTERVAL_MS);
			} else {
				status = "mpegts.js не поддерживается";
				console.error("mpegts.js не загружен или не поддерживается браузером");
			}
		}

		tryStart();

		return () => player?.destroy();
	});
</script>

<svelte:head>
	<title>Онлайн Трансляция</title>
	<script
		src="https://cdn.jsdelivr.net/npm/mpegts.js@1.7.3/dist/mpegts.min.js"
	></script>
</svelte:head>

<div class="page">
	<main class="tuner">
		<div class="video-shell">
			<video bind:this={video} controls autoplay muted playsinline></video>
			{#if status}
				<span class="status">{status}</span>
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

		.chat-sidebar {
			width: 100%;
			height: clamp(360px, 40vh, 480px);
			border-left: none;
			border-top: 1px solid var(--outline-variant);
		}
	}
</style>
