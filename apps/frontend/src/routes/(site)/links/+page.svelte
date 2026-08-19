<script lang="ts">
	import { resolve } from "$app/paths";
</script>

<svelte:head>
	<title>Sapa TV | Каталог</title>
</svelte:head>

<main class="catalog">
	<header class="catalog-head">
		<div class="label-line" aria-hidden="true">
			<span class="line"></span>
			<span class="label">каталог · индексация</span>
			<span class="line"></span>
		</div>
		<h1 class="catalog-title">Ссылки на страницы</h1>
	</header>

	<ul class="list">
		<li class="item">
			<a class="card" href={resolve("")}>
				<span class="card__idx" aria-hidden="true">01</span>
				<span class="card__body">
					<span class="name">Главная</span>
					<span class="url">/</span>
				</span>
			</a>
		</li>
		<li class="item">
			<a class="card" href={resolve("stream")}>
				<span class="card__idx" aria-hidden="true">02</span>
				<span class="card__body">
					<span class="name">Трансляция</span>
					<span class="url">/stream</span>
				</span>
				<span class="card__chip" aria-hidden="true">live-эфир</span>
			</a>
		</li>
		<li class="item">
			<a class="card" href={resolve("admin/panel")}>
				<span class="card__idx" aria-hidden="true">03</span>
				<span class="card__body">
					<span class="name">Админ-панель</span>
					<span class="url">/admin/panel</span>
				</span>
				<span class="card__chip" aria-hidden="true">ключи виджетов</span>
			</a>
		</li>
	</ul>
</main>

<style>
	.catalog {
		max-width: 44rem;
		width: 100%;
		margin: 0 auto;
		min-height: calc(100vh - var(--site-nav-h));
		padding: 3rem 1.5rem;
	}

	.catalog-head {
		margin-bottom: 2.5rem;
	}

	.label-line {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
	}

	.line {
		height: 1px;
		flex: 1;
		background: var(--outline-variant);
	}

	.label {
		font-family: "IBM Plex Mono", monospace;
		font-size: 0.72rem;
		font-weight: 500;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--on-surface-variant);
	}

	.catalog-title {
		margin: 0 0 0.6rem;
		font-size: clamp(1.8rem, 6vw, 2.6rem);
		font-weight: 900;
		letter-spacing: -0.03em;
		color: var(--on-background);
	}

	.list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.card {
		display: flex;
		align-items: center;
		gap: 1.1rem;
		text-decoration: none;
		background: var(--surface-container-lowest);
		border: 1px solid var(--outline-variant);
		border-radius: 1rem;
		padding: 0.9rem 1.1rem;
		transition:
			border-color 0.15s,
			transform 0.15s ease-out,
			box-shadow 0.15s;
	}

	.card:hover {
		border-color: var(--primary);
		transform: translateY(-1px);
		box-shadow: 0 10px 24px -12px
			color-mix(in oklch, var(--primary) 40%, transparent);
	}

	.card__idx {
		font-family: "IBM Plex Mono", monospace;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--on-surface-variant);
		border: 1px solid var(--outline-variant);
		border-radius: 0.5rem;
		padding: 0.35rem 0.45rem;
		min-width: 2.3rem;
		text-align: center;
	}

	.card:hover .card__idx {
		border-color: var(--primary);
		color: var(--primary);
	}

	.card__body {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		min-width: 0;
	}

	.name {
		color: var(--on-surface);
		font-weight: 700;
		font-size: 1rem;
	}

	.url {
		color: var(--on-surface-variant);
		font-family: "IBM Plex Mono", monospace;
		font-size: 0.78rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.card__chip {
		margin-left: auto;
		font-family: "IBM Plex Mono", monospace;
		font-size: 0.66rem;
		font-weight: 500;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--primary);
		border: 1px solid color-mix(in oklch, var(--primary) 40%, transparent);
		border-radius: 6px;
		padding: 0.2rem 0.6rem;
		white-space: nowrap;
	}

	@media (max-width: 420px) {
		.card__chip {
			display: none;
		}
	}
</style>
