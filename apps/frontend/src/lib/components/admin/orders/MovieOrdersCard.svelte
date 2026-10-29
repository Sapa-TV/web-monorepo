<script lang="ts">
	import { api } from "#lib/api";
	import { MovieKind, OrderSource, OrderStatus } from "@sapa-tv-ru/api-client";
	import type {
		MovieOrderResponse,
		UpsertMovieOrderRequest,
	} from "@sapa-tv-ru/api-client";
	import { onMount } from "svelte";
	import IconPencil from "~icons/lucide/pencil";
	import IconPlus from "~icons/lucide/plus";
	import IconTrash2 from "~icons/lucide/trash-2";
	import {
		Alert,
		Badge,
		Button,
		Card,
		Field,
		Input,
		Section,
		Select,
		TableWrap,
	} from "@sapa-tv-ru/ui-kit";
	import { CrudStore } from "#lib/internal/crud-store.svelte";

	let title = $state("");
	let customerName = $state("");
	let kind = $state<MovieKind>(MovieKind.Movie);
	let source = $state<OrderSource>(OrderSource.Other);
	let status = $state<OrderStatus>(OrderStatus.Pending);
	let comment = $state("");

	const kindLabels: Record<string, string> = {
		movie: "фильм",
		series: "сериал",
		anime: "аниме",
		youtube: "ютуб",
	};
	const sourceLabels: Record<string, string> = {
		donate: "донат",
		points: "баллы",
		roulette: "рулетка",
		other: "—",
	};
	const statusLabels: Record<string, string> = {
		pending: "ожидает",
		completed: "просмотрен",
		cancelled: "отмена",
	};

	const crud = new CrudStore<MovieOrderResponse, UpsertMovieOrderRequest>({
		list: async () => api.listMovieOrders(),
		create: async (payload) => api.createMovieOrder(payload),
		update: async (id, payload) => api.updateMovieOrder(id, payload),
		remove: async (id) => api.deleteMovieOrder(id),
		hints: {
			created: "Заказ создан.",
			updated: "Заказ обновлён.",
			removed: "Заказ удалён.",
		},
		confirmRemove: "Удалить заказ фильма?",
	});

	function openNew() {
		title = "";
		customerName = "";
		kind = MovieKind.Movie;
		source = OrderSource.Other;
		status = OrderStatus.Pending;
		comment = "";
		crud.openForm(null);
	}

	function openEdit(order: MovieOrderResponse) {
		title = order.title ?? "";
		customerName = order.customer_name;
		kind = order.kind;
		source = order.source;
		status = order.status;
		comment = order.comment ?? "";
		crud.openForm(order.id);
	}

	function buildPayload(): UpsertMovieOrderRequest {
		return {
			title: title.trim() || null,
			customer_name: customerName.trim(),
			kind,
			source,
			status,
			comment: comment.trim() || null,
		};
	}

	onMount(() => {
		void crud.load();
	});
</script>

<Card wide>
	<Section
		title="Заказы фильмов"
		hint="Список синхронизируется с Google-таблицей."
	>
		{#if crud.error}
			<Alert tone="error">{crud.error}</Alert>
		{/if}
		{#if crud.hint}
			<Alert tone="success">{crud.hint}</Alert>
		{/if}

		<Button variant="primary" onclick={openNew}>
			<IconPlus aria-hidden="true" />
			Создать заказ
		</Button>

		{#if crud.formOpen}
			<form
				class="inline-form stacked"
				onsubmit={(e) => {
					e.preventDefault();
					void crud.save(customerName.trim() !== "", buildPayload());
				}}
			>
				<Field label="Название (можно заполнить позже)">
					<Input
						type="text"
						placeholder="напр. Большой куш"
						bind:value={title}
					/>
				</Field>

				<Field label="Заказчик">
					<Input
						type="text"
						placeholder="ник"
						bind:value={customerName}
						required
					/>
				</Field>

				<Field label="Тип">
					<Select bind:value={kind}>
						{#each Object.entries(kindLabels) as [value, label] (value)}
							<option {value}>{label}</option>
						{/each}
					</Select>
				</Field>

				<Field label="Источник">
					<Select bind:value={source}>
						{#each Object.entries(sourceLabels) as [value, label] (value)}
							<option {value}>{label}</option>
						{/each}
					</Select>
				</Field>

				<Field label="Статус">
					<Select bind:value={status}>
						{#each Object.entries(statusLabels) as [value, label] (value)}
							<option {value}>{label}</option>
						{/each}
					</Select>
				</Field>

				<Field label="Комментарий">
					<Input type="text" placeholder="необязательно" bind:value={comment} />
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
							<th>№</th>
							<th>Название</th>
							<th>Заказчик</th>
							<th>Тип</th>
							<th>Источник</th>
							<th>Статус</th>
							<th class="actions-cell">Действия</th>
						</tr>
					</thead>
					<tbody>
						{#if crud.items.length === 0}
							<tr>
								<td class="empty" colspan="7"
									>Пока пусто — создай заказ или импортируй из таблицы.</td
								>
							</tr>
						{/if}
						{#each crud.items as order (order.id)}
							<tr>
								<td class="mono cell-nowrap">{order.id}</td>
								<td>{order.title ?? "—"}</td>
								<td class="cell-nowrap">{order.customer_name}</td>
								<td class="cell-nowrap"
									>{kindLabels[order.kind] ?? order.kind}</td
								>
								<td class="cell-nowrap"
									>{sourceLabels[order.source] ?? order.source}</td
								>
								<td class="cell-nowrap"
									><Badge tone={order.status}>
										{statusLabels[order.status] ?? order.status}
									</Badge>
								</td>
								<td class="actions-cell">
									<Button
										size="sm"
										icon
										onclick={() => openEdit(order)}
										aria-label="Редактировать заказ"
									>
										<IconPencil aria-hidden="true" />
									</Button>
									<Button
										size="sm"
										variant="danger"
										icon
										onclick={() => crud.remove(order.id)}
										disabled={crud.removeId === order.id}
										aria-label="Удалить заказ"
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
	.empty {
		text-align: center;
		color: var(--on-surface-variant);
		font-size: 13px;
	}
</style>
