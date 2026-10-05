<script lang="ts">
	interface Props {
		name: string;
		image: string;
		glow: string;
		win: boolean;
	}

	let { name, image, glow, win }: Props = $props();

	function fallbackImage(e: Event) {
		const img = e.currentTarget as HTMLImageElement;
		if (!img.src.endsWith("/roulette/common.png")) {
			img.src = "/roulette/common.png";
		}
	}
</script>

<!-- eslint-disable-next-line svelte/no-inline-styles : glow color comes from DB -->
<div class="item" class:item--win={win} style:--glow={glow}>
	<img class="item__card" src={image} alt={name} onerror={fallbackImage} />
	<span class="item__name">{name}</span>
</div>

<style>
	.item {
		flex: 0 0 108px;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
	}

	.item__card {
		width: 96px;
		height: 96px;
		object-fit: contain;
		transition: filter 0.3s ease;
	}

	.item--win .item__card {
		filter: drop-shadow(0 0 10px var(--glow)) drop-shadow(0 0 22px var(--glow));
	}

	.item__name {
		max-width: 108px;
		font-size: 12px;
		font-weight: 600;
		text-align: center;
		color: var(--widget-ink);
		text-shadow: 0 1px 4px var(--widget-shadow);
		overflow: hidden;
		display: -webkit-box;
		line-clamp: 2;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
	}
</style>
