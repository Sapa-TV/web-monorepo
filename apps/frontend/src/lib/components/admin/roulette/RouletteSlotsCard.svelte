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

	let slots = $state<RouletteSlotResponse[]>([]);
	let rarities = $state<RarityResponse[]>([]);
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let formOpen = $state(false);
	let editId = $state<number | null>(null);
	let busy = $state(false);
	let removeId = $state<number | null>(null);

	let name = $state("");
	let rarityId = $state(0);
	let weight = $state("10");
	let action = $state("");

	const totalWeight = $derived(
		slots.reduce((sum, slot) => sum + slot.weight, 0),
	);
	const PERCENT = 100;

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	async function load() {
		error = "";
		hint = "";
		const [slotsRes, raritiesRes] = await Promise.all([
			api.listSlots(),
			api.listRarities(),
		]);
		if (slotsRes.isErr()) throw slotsRes.error;
		if (raritiesRes.isErr()) throw raritiesRes.error;
		slots = slotsRes.value;
		rarities = raritiesRes.value;
		loaded = true;
	}

	function openNew() {
		editId = null;
		name = "";
		rarityId = rarities[0]?.id ?? 0;
		weight = "10";
		action = "";
		formOpen = true;
	}

	function openEdit(slot: RouletteSlotResponse) {
		editId = slot.id;
		name = slot.name;
		rarityId = slot.rarity_id;
		weight = String(slot.weight);
		action = slot.action;
		formOpen = true;
	}

	function cancelForm() {
		formOpen = false;
		editId = null;
	}

	async function save() {
		if (!name.trim()) return;
		busy = true;
		error = "";
		try {
			const payload: UpsertRouletteSlotRequest = {
				name: name.trim(),
				rarity_id: rarityId,
				weight: Math.max(0, Number.parseInt(weight, 10) || 0),
				action: action.trim(),
			};
			const res =
				editId === null
					? await api.createSlot(payload)
					: await api.updateSlot(editId, payload);
			if (res.isErr()) throw res.error;
			hint = editId === null ? "Слот создан." : "Слот обновлён.";
			formOpen = false;
			editId = null;
			await load();
		} catch (err) {
			setError(err);
		} finally {
			busy = false;
		}
	}

	async function remove(id: number) {
		if (!confirm("Удалить слот рулетки?")) return;
		removeId = id;
		error = "";
		try {
			const res = await api.deleteSlot(id);
			if (res.isErr()) throw res.error;
			hint = "Слот удалён.";
			await load();
		} catch (err) {
			setError(err);
		} finally {
			removeId = null;
		}
	}

	function rarityOf(id: number): RarityResponse | undefined {
		return rarities.find((r) => r.id === id);
	}

	function chance(slot: RouletteSlotResponse): string {
		if (totalWeight <= 0) return "—";
		return `${((slot.weight / totalWeight) * PERCENT).toFixed(1)}%`;
	}

	onMount(() => {
		load().catch(setError);
	});
</script>

<Card>
	<Section title="Слоты рулетки">
		<p class="section-hint">
			Секторы колеса. Шанс сектора — его вес относительно суммы весов.
		</p>

		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		<Button variant="primary" onclick={openNew}>
			<IconPlus aria-hidden="true" />
			Создать слот
		</Button>

		{#if formOpen}
			<RouletteSlotForm
				bind:name
				bind:rarityId
				bind:weight
				bind:action
				{rarities}
				{busy}
				submitLabel={editId === null ? "Создать" : "Сохранить"}
				onsave={() => void save()}
				oncancel={cancelForm}
			/>
		{/if}

		{#if loaded}
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
						{#each slots as slot (slot.id)}
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
										onclick={() => remove(slot.id)}
										disabled={removeId === slot.id}
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
