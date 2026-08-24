<script lang="ts">
	import RouletteStage from "#lib/components/widgets/RouletteStage.svelte";
	import { RouletteStore } from "#lib/widgets/roulette-store.svelte";
	import { onDestroy } from "svelte";

	const widgetAccessKey =
		typeof window !== "undefined"
			? (new URLSearchParams(window.location.search).get("widget_access_key") ??
				"")
			: "";

	const roulette = new RouletteStore(widgetAccessKey);

	onDestroy(() => roulette.stop());
	roulette.start();
</script>

<svelte:head>
	<title>Виджет — Рулетка</title>
</svelte:head>

<RouletteStage
	phase={roulette.phase}
	stateLabel={roulette.stateLabel}
	idleText={roulette.idleText}
	spin={roulette.spin}
	conn={roulette.conn}
	badge={roulette.badge}
/>
