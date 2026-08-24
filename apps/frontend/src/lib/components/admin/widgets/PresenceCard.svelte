<script lang="ts">
	import { api } from "#lib/api";
	import { Badge, Card, Section } from "@sapa-tv-ru/ui-kit";
	import { onMount } from "svelte";

	const POLL_INTERVAL_MS = 5000;

	let dockConnected = $state(false);
	let widgetCount = $state(0);
	let loaded = $state(false);

	async function loadPresence() {
		const res = await api.getPresence();
		if (res.isErr()) return;
		dockConnected = res.value.dock_connected;
		widgetCount = res.value.widget_count;
		loaded = true;
	}

	onMount(() => {
		void loadPresence();
		const poll = setInterval(() => void loadPresence(), POLL_INTERVAL_MS);
		return () => clearInterval(poll);
	});
</script>

<Card>
	<Section title="Клиенты">
		<p class="section-hint">
			Кто сейчас подключён по websocket (опрос каждые 5 сек).
		</p>

		{#if loaded}
			<ul class="presence-list" role="status">
				<li>
					<span class="presence-label">Док-панель</span>
					<Badge tone={dockConnected ? "ok" : "bad"}>
						{dockConnected ? "онлайн" : "офлайн"}
					</Badge>
				</li>
				<li>
					<span class="presence-label">Виджет</span>
					<Badge tone={widgetCount > 0 ? "ok" : "bad"}>
						{widgetCount > 0 ? "онлайн" : "офлайн"}
					</Badge>
				</li>
			</ul>
		{:else}
			<p class="section-hint">Загрузка...</p>
		{/if}
	</Section>
</Card>

<style>
	.presence-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.presence-list li {
		display: flex;
		align-items: center;
		gap: 10px;
	}

	.presence-label {
		min-width: 90px;
		font-weight: 600;
	}
</style>
