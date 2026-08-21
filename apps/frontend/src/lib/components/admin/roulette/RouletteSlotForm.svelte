<script lang="ts">
	import type { RarityResponse } from "@sapa-tv-ru/api-client";
	import { Button, Field, Input, Select } from "@sapa-tv-ru/ui-kit";

	interface Props {
		rarities: RarityResponse[];
		name: string;
		rarityId: number;
		weight: string;
		action: string;
		busy?: boolean;
		submitLabel: string;
		onsave: () => void;
		oncancel: () => void;
	}

	let {
		rarities,
		name = $bindable(""),
		rarityId = $bindable(0),
		weight = $bindable("10"),
		action = $bindable(""),
		busy = false,
		submitLabel,
		onsave,
		oncancel,
	}: Props = $props();

	const selectedColor = $derived(
		rarities.find((r) => r.id === rarityId)?.color ?? "transparent",
	);

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!name.trim() || rarities.length === 0) return;
		onsave();
	}
</script>

<form class="inline-form stacked" onsubmit={submit}>
	<Field label="Название">
		<Input
			type="text"
			placeholder="напр. Подписка"
			bind:value={name}
			required
		/>
	</Field>

	<Field label="Редкость">
		<span class="rarity-select">
			<!-- eslint-disable-next-line svelte/no-inline-styles : color comes from DB -->
			<span class="dot" style:background={selectedColor} aria-hidden="true"
			></span>
			<Select bind:value={rarityId} disabled={rarities.length === 0}>
				{#each rarities as rarity (rarity.id)}
					<option value={rarity.id}>{rarity.display_name}</option>
				{/each}
			</Select>
		</span>
	</Field>

	<Field label="Вес">
		<Input type="number" min={0} step={1} bind:value={weight} required />
	</Field>

	<Field label="Действие">
		<Input type="text" placeholder="необязательно" bind:value={action} />
	</Field>

	<div class="form-actions">
		<Button
			variant="primary"
			type="submit"
			disabled={busy || rarities.length === 0}
		>
			{busy ? "Сохранение..." : submitLabel}
		</Button>
		<Button size="sm" type="button" onclick={oncancel}>Отмена</Button>
	</div>
</form>

<style>
	.rarity-select {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
		border: 1px solid var(--outline-variant);
	}
</style>
