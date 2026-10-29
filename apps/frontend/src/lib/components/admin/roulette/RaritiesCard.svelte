<script lang="ts">
	import { api } from "#lib/api";
	import type {
		RarityResponse,
		UpsertRarityRequest,
	} from "@sapa-tv-ru/api-client";
	import { onMount } from "svelte";
	import IconChevronDown from "~icons/lucide/chevron-down";
	import IconPencil from "~icons/lucide/pencil";
	import IconPlus from "~icons/lucide/plus";
	import IconTrash2 from "~icons/lucide/trash-2";
	import { Alert, Button, Field, Input, TableWrap } from "@sapa-tv-ru/ui-kit";
	import { CrudStore } from "#lib/internal/crud-store.svelte";

	const DEFAULT_COLOR = "#9d9d9d";

	let name = $state("");
	let displayName = $state("");
	let image = $state("");
	let color = $state(DEFAULT_COLOR);

	const crud = new CrudStore<RarityResponse, UpsertRarityRequest>({
		list: async () => api.listRarities(),
		create: async (payload) => api.createRarity(payload),
		update: async (id, payload) => api.updateRarity(id, payload),
		remove: async (id) => api.deleteRarity(id),
		hints: {
			created: "Редкость создана.",
			updated: "Редкость обновлена.",
			removed: "Редкость удалена.",
		},
		confirmRemove:
			"Удалить редкость? Слоты с этой редкостью будут удалены вместе с ней.",
	});

	function openNew() {
		name = "";
		displayName = "";
		image = "";
		color = DEFAULT_COLOR;
		crud.openForm(null);
	}

	function openEdit(rarity: RarityResponse) {
		name = rarity.name;
		displayName = rarity.display_name;
		image = rarity.image;
		color = rarity.color;
		crud.openForm(rarity.id);
	}

	function buildPayload(): UpsertRarityRequest {
		return {
			name: name.trim(),
			display_name: displayName.trim(),
			image: image.trim(),
			color: color.trim(),
		};
	}

	onMount(() => {
		void crud.load();
	});
</script>

<details class="rarities">
	<summary>
		<span class="summary-title">Редкости</span>
		<span class="summary-count">{crud.items.length}</span>
		<span class="summary-chevron">
			<IconChevronDown aria-hidden="true" />
		</span>
	</summary>

	<div class="rarities-body">
		<p class="rarities-hint">
			Названия и цвета редкостей рулетки. Используются в слотах и на виджете.
		</p>

		{#if crud.error}
			<Alert tone="error">{crud.error}</Alert>
		{/if}
		{#if crud.hint}
			<Alert tone="success">{crud.hint}</Alert>
		{/if}

		<Button variant="primary" onclick={openNew}>
			<IconPlus aria-hidden="true" />
			Создать редкость
		</Button>

		{#if crud.formOpen}
			<form
				class="inline-form stacked"
				onsubmit={(e) => {
					e.preventDefault();
					void crud.save(
						name.trim() !== "" && displayName.trim() !== "",
						buildPayload(),
					);
				}}
			>
				<Field label="Отображаемое имя">
					<Input
						type="text"
						placeholder="напр. Легендарная"
						bind:value={displayName}
						required
					/>
				</Field>

				<Field label="Имя (slug)">
					<Input
						type="text"
						placeholder="напр. legendary"
						bind:value={name}
						required
					/>
				</Field>

				<Field label="Картинка">
					<Input
						type="text"
						placeholder="напр. legendary.png"
						bind:value={image}
					/>
				</Field>

				<Field label="Цвет">
					<span class="color-field">
						<!-- eslint-disable-next-line svelte/no-inline-styles : color comes from DB -->
						<span class="dot" style:background={color} aria-hidden="true"
						></span>
						<Input type="text" placeholder="#9d9d9d" bind:value={color} />
					</span>
				</Field>

				<div class="form-actions">
					<Button variant="primary" type="submit" disabled={crud.busy}>
						{crud.busy
							? "Сохранение..."
							: crud.editId === null
								? "Создать"
								: "Сохранить"}
					</Button>
					<Button size="sm" type="button" onclick={() => crud.cancelForm()}
						>Отмена</Button
					>
				</div>
			</form>
		{/if}

		{#if crud.loaded}
			<TableWrap>
				<table>
					<thead>
						<tr>
							<th>Название</th>
							<th>Slug</th>
							<th>Цвет</th>
							<th>Картинка</th>
							<th class="actions-cell">Действия</th>
						</tr>
					</thead>
					<tbody>
						{#each crud.items as rarity (rarity.id)}
							<tr>
								<td>{rarity.display_name}</td>
								<td class="mono">{rarity.name}</td>
								<td>
									<span class="color-cell">
										<!-- eslint-disable svelte/no-inline-styles : color comes from DB -->
										<span
											class="dot"
											style:background={rarity.color}
											aria-hidden="true"
										></span>
										<!-- eslint-enable svelte/no-inline-styles -->
										<span class="mono">{rarity.color}</span>
									</span>
								</td>
								<td class="mono">{rarity.image || "—"}</td>
								<td class="actions-cell">
									<Button
										size="sm"
										icon
										onclick={() => openEdit(rarity)}
										aria-label="Редактировать редкость"
									>
										<IconPencil aria-hidden="true" />
									</Button>
									<Button
										size="sm"
										variant="danger"
										icon
										onclick={() => crud.remove(rarity.id)}
										disabled={crud.removeId === rarity.id}
										aria-label="Удалить редкость"
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
	</div>
</details>

<style>
	.rarities-hint {
		margin: 0 0 12px;
		color: var(--on-surface-variant);
		font-size: 12px;
		line-height: 1.5;
	}

	.rarities {
		border: 1px solid var(--outline-variant);
		border-radius: 12px;
		background: var(--surface-container);
	}

	.rarities summary {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 14px 16px;
		cursor: pointer;
		user-select: none;
		list-style: none;
		font-weight: 600;
	}

	.rarities summary::-webkit-details-marker {
		display: none;
	}

	.summary-title {
		font-size: 15px;
		color: var(--on-surface);
	}

	.summary-count {
		font-size: 12px;
		padding: 1px 8px;
		border-radius: 999px;
		background: var(--surface-container-high);
		color: var(--on-surface-variant);
	}

	.summary-chevron {
		margin-left: auto;
		display: inline-flex;
		color: var(--on-surface-variant);
		transition: transform 0.15s ease;
	}

	.rarities[open] .summary-chevron {
		transform: rotate(180deg);
	}

	.rarities-body {
		display: flex;
		flex-direction: column;
		gap: 12px;
		align-items: flex-start;
		padding: 0 16px 16px;
	}

	.color-field {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.color-cell {
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
