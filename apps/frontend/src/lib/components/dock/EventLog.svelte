<script lang="ts">
	import { Section } from "@sapa-tv-ru/ui-kit";

	interface Props {
		events: {
			time: string;
			text: string;
			cls: "start" | "complete" | "error";
		}[];
	}

	let { events }: Props = $props();
</script>

<Section title="События">
	<div class="event-log" role="log">
		{#if events.length === 0}
			<div class="empty-row">Ожидание событий...</div>
		{:else}
			{#each events as e (e)}
				<div class={`ev ev-${e.cls}`}>
					<span class="ev-time">[{e.time}]</span>
					{e.text}
				</div>
			{/each}
		{/if}
	</div>
</Section>

<style>
	.event-log {
		background: var(--surface-container);
		border: 1px solid var(--outline-variant);
		border-radius: 12px;
		padding: 14px;
		max-height: 200px;
		overflow-y: auto;
		font-family: var(--font-mono);
		font-size: 12px;
	}

	.event-log .ev {
		padding: 4px 0;
		border-bottom: 1px solid var(--outline-variant);
	}

	.event-log .ev:last-child {
		border-bottom: none;
	}

	.ev-time {
		color: var(--on-surface-variant);
		margin-right: 8px;
	}

	.ev-start {
		color: var(--primary);
	}

	.ev-complete {
		color: var(--secondary);
	}

	.ev-error {
		color: var(--error);
	}

	.empty-row {
		color: var(--on-surface-variant);
		font-style: italic;
		padding: 14px;
		text-align: center;
	}
</style>
