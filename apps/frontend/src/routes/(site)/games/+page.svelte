<script lang="ts">
	import { onMount } from "svelte";
	import { Alert } from "@sapa-tv-ru/ui-kit";
	import { api } from "#lib/api";
	import { describeApiError } from "#lib/api-error-text";
	import OrdersCatalog from "#lib/components/orders/OrdersCatalog.svelte";
	import type { GameOrderResponse } from "@sapa-tv-ru/api-client";

	let orders = $state<GameOrderResponse[]>([]);
	let loaded = $state(false);
	let error = $state("");

	async function load() {
		const res = await api.listGameOrders();
		if (res.isErr()) {
			error = describeApiError(res.error);
			return;
		}
		orders = res.value;
	}

	onMount(() => {
		void load().finally(() => (loaded = true));
	});
</script>

<svelte:head>
	<title>Sapa TV | Заказы игр</title>
</svelte:head>

<main class="catalog">
	<header class="catalog-head">
		<div class="label-line" aria-hidden="true">
			<span class="line"></span>
			<span class="label">заказы · игры</span>
			<span class="line"></span>
		</div>
		<h1 class="catalog-title">Заказы игр</h1>
	</header>

	{#if error}
		<Alert tone="error">{error}</Alert>
	{/if}

	{#if loaded && !error}
		<OrdersCatalog mode="games" {orders} />
	{/if}
</main>

<style>
	.catalog {
		max-width: none;
		width: 100%;
		margin: 0 auto;
		min-height: calc(100vh - var(--site-nav-h));
		padding: 3rem 1.5rem;
	}

	.catalog-head {
		margin-bottom: 1.6rem;
	}

	.label-line {
		display: flex;
		align-items: center;
		gap: 0.9rem;
		margin-bottom: 0.7rem;
	}

	.label {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		letter-spacing: 0.14em;
		text-transform: uppercase;
		color: var(--on-surface-variant);
	}

	.line {
		height: 1px;
		flex: 1;
		background: var(--outline-variant);
	}

	.catalog-title {
		margin: 0;
		font-family: var(--font-heading);
		font-size: clamp(1.8rem, 6vw, 2.6rem);
		font-weight: 900;
		letter-spacing: -0.03em;
		color: var(--on-background);
	}
</style>
