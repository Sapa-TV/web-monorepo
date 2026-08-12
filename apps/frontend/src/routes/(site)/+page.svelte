<script lang="ts">
	import IconPlay from "~icons/lucide/play";
	import IconTwitch from "~icons/lucide/twitch";
	import IconYoutube from "~icons/lucide/youtube";
	import IconVk from "~icons/simple-icons/vk";
</script>

<svelte:head>
	<title>Sapa TV | Скоро открытие</title>
</svelte:head>

<main class="landing">
	<section class="stage" aria-label="Sapa TV — скоро открытие">
		<div class="stack">
			<div class="crt" aria-hidden="true">
				<div class="crt__mask">
					<img class="crt__cast" src="/avatar.png" alt="" />
					<span class="crt__scan"></span>
				</div>
				<div class="crt__foot">
					<span class="crt__badge">SAPA</span>
					<span class="crt__idx">A-00</span>
				</div>
			</div>
		</div>

		<h1 class="wordmark">
			<span>sapa</span><span class="wordmark-sep">·</span><span>tv</span>
		</h1>

		<p class="tagline">Скоро открытие.</p>

		<div class="shelf" aria-label="Выбери площадку">
			<a class="tape tape--stream" href="/stream">
				<span class="tape__ink">
					<IconPlay aria-hidden="true" />
					Смотреть стрим
				</span>
			</a>
			<a
				class="tape tape--twitch"
				href="https://www.twitch.tv/sapushka_"
				target="_blank"
				rel="noopener"
			>
				<span class="tape__ink">
					<IconTwitch aria-hidden="true" />
					Смотреть на Twitch
				</span>
			</a>
			<a
				class="tape tape--vk"
				href="https://live.vkvideo.ru/sapushka_"
				target="_blank"
				rel="noopener"
			>
				<span class="tape__index">
					<IconVk aria-hidden="true" />
					Смотреть на VK Видео Live
				</span>
			</a>
			<a
				class="tape tape--youtube"
				href="https://www.youtube.com/@sapa_sapushka"
				target="_blank"
				rel="noopener"
			>
				<span class="tape__index">
					<IconYoutube aria-hidden="true" />
					Смотреть на YouTube
				</span>
			</a>
		</div>
	</section>
</main>

<style>
	.landing {
		min-height: calc(100vh - var(--site-nav-h));
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem 1.5rem 2.5rem;
	}

	.stage {
		width: 100%;
		max-width: 42rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1.5rem;
		text-align: center;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.35;
		}
	}

	.stack {
		display: flex;
		justify-content: center;
	}

	.crt {
		width: min(24rem, 90vw);
		border-radius: 1.25rem;
		background: var(--surface-container-high);
		padding: 0.6rem 0.6rem 0.4rem;
		box-shadow:
			0 1px 0 var(--surface-bright) inset,
			0 18px 40px -18px color-mix(in oklch, var(--on-surface) 45%, transparent);
		border: 1px solid var(--outline-variant);
	}

	.crt__mask {
		position: relative;
		border-radius: 0.85rem;
		overflow: hidden;
		background: var(--surface-container-highest);
		line-height: 0;
	}

	.crt__cast {
		display: block;
		width: 100%;
		aspect-ratio: 1 / 1;
		object-fit: cover;
	}

	.crt__scan {
		position: absolute;
		inset: 0;
		pointer-events: none;
		background-image:
			repeating-linear-gradient(
				to bottom,
				transparent 0 2px,
				rgba(0 0 0 / 0.08) 2px 3px
			),
			radial-gradient(
				circle at 50% 42%,
				transparent 0 52%,
				color-mix(in oklch, var(--on-surface) 14%, transparent) 100%
			);
		opacity: 0.7;
	}

	.crt__foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem 0.7rem 0.15rem;
		font-family: "IBM Plex Mono", monospace;
		font-size: 0.62rem;
		font-weight: 600;
		letter-spacing: 0.12em;
		color: var(--on-surface-variant);
	}

	.crt__idx {
		color: var(--primary);
	}

	.wordmark {
		margin: 0;
		font-family: "Archivo", sans-serif;
		font-size: clamp(2.6rem, 15vw, 6rem);
		font-weight: 900;
		letter-spacing: -0.04em;
		line-height: 1;
		color: var(--on-background);
		display: flex;
		align-items: baseline;
		flex-wrap: wrap;
		justify-content: center;
	}

	.wordmark-sep {
		color: var(--primary);
		margin: 0 0.15em;
	}

	.tagline {
		margin: 0;
		color: var(--on-surface-variant);
		font-size: 1.05rem;
		font-weight: 500;
		line-height: 1.55;
		max-width: 28rem;
	}

	.shelf {
		width: 100%;
		max-width: 30rem;
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
		margin-top: 0.5rem;
	}

	.tape {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		text-decoration: none;
		background: var(--brand);
		color: var(--brand-ink);
		border-radius: 0.9rem;
		padding: 0.85rem 2rem;
		box-shadow:
			0 8px 20px -8px color-mix(in oklch, var(--brand) 60%, transparent),
			inset 0 0 0 1px color-mix(in oklch, var(--brand-ink) 18%, transparent);
		transition:
			transform 0.16s ease-out,
			filter 0.16s ease-out;
	}

	.tape:hover {
		transform: translateY(-2px);
		filter: brightness(1.06);
	}

	.tape:active {
		transform: translateY(0) scale(0.985);
	}

	.tape__ink,
	.tape__index {
		display: inline-flex;
		align-items: center;
		gap: 0.6rem;
		font-weight: 700;
		font-size: 0.95rem;
		line-height: 1;
	}

	.tape--stream {
		--brand: var(--primary-dim);
		--brand-ink: var(--stream-ink);
	}

	.tape--twitch {
		--brand: var(--twitch-brand);
		--brand-ink: var(--twitch-ink);
	}

	.tape--vk {
		--brand: var(--vk-brand);
		--brand-ink: var(--vk-ink);
	}

	.tape--youtube {
		--brand: var(--youtube-brand);
		--brand-ink: var(--youtube-ink);
	}

	@media (min-width: 640px) {
		.tape {
			padding: 1rem 2rem;
		}
	}
</style>
