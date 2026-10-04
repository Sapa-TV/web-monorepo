<script lang="ts">
	import GameOrdersCard from "#lib/components/admin/orders/GameOrdersCard.svelte";
	import MovieOrdersCard from "#lib/components/admin/orders/MovieOrdersCard.svelte";
	import SheetsCard from "#lib/components/admin/orders/SheetsCard.svelte";
	import VipCard from "#lib/components/admin/orders/VipCard.svelte";

	type Tab = "games" | "movies" | "vip";

	let tab = $state<Tab>("games");

	const tabs: { id: Tab; label: string }[] = [
		{ id: "games", label: "Игры" },
		{ id: "movies", label: "Фильмы" },
		{ id: "vip", label: "VIP" },
	];
</script>

<svelte:head>
	<title>Sapa TV | Заказы</title>
</svelte:head>

<div class="tabs" role="tablist">
	{#each tabs as t (t.id)}
		<button
			type="button"
			role="tab"
			aria-selected={tab === t.id}
			class="tab"
			class:tab--active={tab === t.id}
			onclick={() => (tab = t.id)}
		>
			{t.label}
		</button>
	{/each}
</div>

{#if tab === "games"}
	<GameOrdersCard />
{:else if tab === "movies"}
	<MovieOrdersCard />
{:else}
	<VipCard />
{/if}

<SheetsCard />

<style>
	.tabs {
		display: flex;
		gap: 8px;
		margin-bottom: 16px;
	}

	.tab {
		appearance: none;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container-low);
		color: var(--on-surface);
		border-radius: 999px;
		padding: 6px 16px;
		font: inherit;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition:
			border-color 0.15s,
			background 0.15s;
	}

	.tab:hover {
		border-color: var(--primary);
	}

	.tab--active {
		background: var(--primary);
		border-color: transparent;
		color: var(--on-primary);
	}
</style>
