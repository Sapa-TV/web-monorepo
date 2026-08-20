<script lang="ts">
	import { asset, resolve } from "$app/paths";
	import { Button } from "@sapa-tv-ru/ui-kit";
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
					<img class="crt__cast" src={asset("avatar.png")} alt="" />
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
			<div class="tape tape--stream">
				<Button variant="brand" href={resolve("stream")}>
					<IconPlay aria-hidden="true" />
					Смотреть стрим
				</Button>
			</div>
			<div class="tape tape--twitch">
				<Button
					variant="brand"
					href="https://www.twitch.tv/sapushka_"
					target="_blank"
					rel="noopener"
				>
					<IconTwitch aria-hidden="true" />
					Смотреть на Twitch
				</Button>
			</div>
			<div class="tape tape--vk">
				<Button
					variant="brand"
					href="https://live.vkvideo.ru/sapushka_"
					target="_blank"
					rel="noopener"
				>
					<IconVk aria-hidden="true" />
					Смотреть на VK Видео Live
				</Button>
			</div>
			<div class="tape tape--youtube">
				<Button
					variant="brand"
					href="https://www.youtube.com/@sapa_sapushka"
					target="_blank"
					rel="noopener"
				>
					<IconYoutube aria-hidden="true" />
					Смотреть на YouTube
				</Button>
			</div>
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
		flex-direction: column;
	}

	.tape :global(.btn) {
		width: 100%;
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
</style>
