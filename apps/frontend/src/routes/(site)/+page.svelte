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

		<div class="shelf" role="list" aria-label="Выбери площадку">
			<a
				class="tape"
				role="listitem"
				style="--brand: oklch(0.62 0.25 293); --brand-ink: oklch(0.97 0.02 293)"
				href="https://www.twitch.tv/sapushka_"
				target="_blank"
				rel="noopener"
			>
				<span class="tape__ink">
					<svg
						viewBox="0 0 24 24"
						fill="currentColor"
						aria-hidden="true"
					>
						<path
							d="M11.571 4.714h1.715v5.143H11.57zm4.715 0H18v5.143h-1.714zM6 0 1.714 4.286v15.428h5.143V24l4.286-4.286h3.428L22.286 12V0zm14.571 11.143-3.428 3.428h-3.429l-3 3v-3H6.857V1.714h13.714z"
						/>
					</svg>
					Смотреть на Twitch
				</span>
			</a>
			<a
				class="tape"
				role="listitem"
				style="--brand: oklch(0.62 0.19 252); --brand-ink: oklch(0.97 0.02 252)"
				href="https://live.vkvideo.ru/sapushka_"
				target="_blank"
				rel="noopener"
			>
				<span class="tape__index">
					<svg
						viewBox="0 0 24 24"
						fill="currentColor"
						aria-hidden="true"
					>
						<path
							d="M15.684 2H8.316C2.504 2 2 2.504 2 8.316v7.368C2 21.496 2.504 22 8.316 22h7.368C21.496 22 22 21.496 22 15.684V8.316C22 2.504 21.496 2 15.684 2zm3.268 12.292h-1.682c-.636 0-.832-.508-1.976-1.668-1.004-.988-1.444-1.124-1.692-1.124-.348 0-.448.1-.448.58v1.524c0 .416-.132.664-1.216.664-1.792 0-3.784-1.084-5.172-3.108C4.664 8.664 4.216 7.1 4.216 6.684c0-.248.1-.48.58-.48h1.682c.44 0 .604.184.772.632.848 2.46 2.26 4.616 2.844 4.616.216 0 .316-.1.316-.648V9.08c-.064-1.044-.612-1.132-.612-1.5 0-.184.152-.364.4-.364h2.648c.368 0 .496.2.496.616v2.512c0 .368.164.496.268.496.216 0 .396-.1.792-.496 1.248-1.396 2.14-3.552 2.14-3.552.12-.248.312-.48.748-.48h1.682c.52 0 .624.264.512.616-.216.984-2.312 3.892-2.312 3.892-.184.296-.248.428 0 .76.184.248.78.76 1.184 1.216.5.66 1.06 1.32 1.244 1.944.164.468-.084.712-.556.712z"
						/>
					</svg>
					VK Видео Live
				</span>
			</a>
			<a
				class="tape"
				role="listitem"
				style="--brand: oklch(0.6 0.2 28); --brand-ink: oklch(0.98 0.01 28)"
				href="https://www.youtube.com/@sapa_sapushka"
				target="_blank"
				rel="noopener"
			>
				<span class="tape__index">
					<svg
						viewBox="0 0 24 24"
						fill="currentColor"
						aria-hidden="true"
					>
						<path
							d="M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z"
						/>
					</svg>
					YouTube
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

	.stage__kicker {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		font-family: "IBM Plex Mono", monospace;
		font-size: 0.72rem;
		font-weight: 500;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--on-surface-variant);
		border: 1px solid var(--outline-variant);
		background: var(--surface-container-low);
		padding: 0.35rem 0.85rem;
		border-radius: 6px;
	}

	.ticker-dot {
		width: 0.45rem;
		height: 0.45rem;
		border-radius: 6px;
		background: var(--primary);
		animation: pulse 2.2s ease-in-out infinite;
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
		padding: 0.85rem 1.1rem;
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

	.tape__ink svg,
	.tape__index svg {
		width: 1.15rem;
		height: 1.15rem;
		flex-shrink: 0;
	}

	.tape__label {
		font-family: "IBM Plex Mono", monospace;
		font-size: 0.66rem;
		font-weight: 500;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		opacity: 0.78;
		white-space: nowrap;
	}

	.footnote {
		margin: 0.75rem 0 0;
		font-family: "IBM Plex Mono", monospace;
		font-size: 0.72rem;
		letter-spacing: 0.08em;
		color: var(--on-surface-variant);
	}

	@media (min-width: 640px) {
		.tape {
			padding: 1rem 1.35rem;
		}
	}
</style>