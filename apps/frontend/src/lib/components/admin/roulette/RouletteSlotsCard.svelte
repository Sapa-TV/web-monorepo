<script lang="ts">
	import { api } from "#lib/api";
	import type {
		RarityResponse,
		RouletteSlotResponse,
		UpsertRouletteSlotRequest,
	} from "@sapa-tv-ru/api-client";
	import { onMount } from "svelte";
	import IconPencil from "~icons/lucide/pencil";
	import IconPlus from "~icons/lucide/plus";
	import IconTrash2 from "~icons/lucide/trash-2";
	import { Alert, Button, Card, Section, TableWrap } from "@sapa-tv-ru/ui-kit";
	import RouletteSlotForm from "#lib/components/admin/roulette/RouletteSlotForm.svelte";
	import { err, ok } from "neverthrow";
	import { CrudStore } from "#lib/internal/crud-store.svelte";

	let rarities = $state<RarityResponse[]>([]);

	let name = $state("");
	let rarityId = $state(0);
	let weight = $state("10");
	let action = $state("");

	const crud = new CrudStore<RouletteSlotResponse, UpsertRouletteSlotRequest>({
		list: async () => {
			const slotsRes = await api.listSlots();
			if (slotsRes.isErr()) return err(slotsRes.error);
			const raritiesRes = await api.listRarities();
			if (raritiesRes.isErr()) return err(raritiesRes.error);
			rarities = raritiesRes.value;
			return ok(slotsRes.value);
		},
		create: async (payload) => api.createSlot(payload),
		update: async (id, payload) => api.updateSlot(id, payload),
		remove: async (id) => api.deleteSlot(id),
		hints: {
			created: "Слот создан.",
			updated: "Слот обновлён.",
			removed: "Слот удалён.",
		},
		confirmRemove: "Удалить слот рулетки?",
	});

	const totalWeight = $derived(
		crud.items.reduce((sum, slot) => sum + slot.weight, 0),
	);
	const PERCENT = 100;

	function openNew() {
		name = "";
		rarityId = rarities[0]?.id ?? 0;
		weight = "10";
		action = "";
		crud.openForm(null);
	}

	function openEdit(slot: RouletteSlotResponse) {
		name = slot.name;
		rarityId = slot.rarity_id;
		weight = String(slot.weight);
		action = slot.action;
		crud.openForm(slot.id);
	}

	function buildPayload(): UpsertRouletteSlotRequest {
		return {
			name: name.trim(),
			rarity_id: rarityId,
			weight: Math.max(0, Number.parseInt(weight, 10) || 0),
			action: action.trim(),
		};
	}

	function rarityOf(id: number): RarityResponse | undefined {
		return rarities.find((r) => r.id === id);
	}

	function chance(slot: RouletteSlotResponse): string {
		if (totalWeight <= 0) return "—";
		return `${((slot.weight / totalWeight) * PERCENT).toFixed(1)}%`;
	}

	onMount(() => {
		void crud.load();
	});
</script>

<Card>
	<Section
		title="Слоты рулетки"
		hint="Секторы колеса. Шанс сектора — его вес относительно суммы весов."
	>
		{#if crud.error}
			<Alert tone="error">{crud.error}</Alert>
		{/if}
		{#if crud.hint}
			<Alert tone="success">{crud.hint}</Alert>
		{/if}

		<Button variant="primary" onclick={openNew}>
			<IconPlus aria-hidden="true" />
			Создать слот
		</Button>

		{#if crud.formOpen}
			<RouletteSlotForm
				bind:name
				bind:rarityId
				bind:weight
				bind:action
				{rarities}
				busy={crud.busy}
				submitLabel={crud.editId === null ? "Создать" : "Сохранить"}
				onsave={() => void crud.save(name.trim() !== "", buildPayload())}
				oncancel={() => crud.cancelForm()}
			/>
		{/if}

		{#if crud.loaded}
			<TableWrap>
				<table>
					<thead>
						<tr>
							<th>Имя</th>
							<th>Редкость</th>
							<th>Вес</th>
							<th>Шанс</th>
							<th>Действие</th>
							<th class="actions-cell">Действия</th>
						</tr>
					</thead>
					<tbody>
						{#each crud.items as slot (slot.id)}
							{@const rarity = rarityOf(slot.rarity_id)}
							<tr>
								<td>{slot.name}</td>
								<td>
									{#if rarity}
										<span class="rarity-cell">
											<!-- eslint-disable svelte/no-inline-styles : color comes from DB -->
											<span
												class="dot"
												style:background={rarity.color}
												aria-hidden="true"
											></span>
											<!-- eslint-enable svelte/no-inline-styles -->
											{rarity.display_name}
										</span>
									{:else}
										—
									{/if}
								</td>
								<td>{slot.weight}</td>
								<td>{chance(slot)}</td>
								<td class="mono">{slot.action || "—"}</td>
								<td class="actions-cell">
									<Button
										size="sm"
										icon
										onclick={() => openEdit(slot)}
										aria-label="Редактировать слот"
									>
										<IconPencil aria-hidden="true" />
									</Button>
									<Button
										size="sm"
										variant="danger"
										icon
										onclick={() => crud.remove(slot.id)}
										disabled={crud.removeId === slot.id}
										aria-label="Удалить слот"
									>
										<IconTrash2 aria-hidden="true" />
									</Button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</TableWrap>
		{:else}
			<p class="loading">Загрузка...</p>
		{/if}
	</Section>
</Card>

<style>
	.rarity-cell {
		display: inline-flex;
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
