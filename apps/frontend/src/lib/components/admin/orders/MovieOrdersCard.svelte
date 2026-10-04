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
	import { describeApiError } from "#lib/api-error-text";

	let orders = $state<MovieOrderResponse[]>([]);
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let formOpen = $state(false);
	let editId = $state<number | null>(null);
	let busy = $state(false);
	let removeId = $state<number | null>(null);

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

	function setError(err: unknown) {
		error = describeApiError(err);
	}

	async function load() {
		error = "";
		hint = "";
		const res = await api.listMovieOrders();
		if (res.isErr()) {
			setError(res.error);
			return;
		}
		orders = res.value;
		loaded = true;
	}

	function openNew() {
		editId = null;
		title = "";
		customerName = "";
		kind = MovieKind.Movie;
		source = OrderSource.Other;
		status = OrderStatus.Pending;
		comment = "";
		formOpen = true;
	}

	function openEdit(order: MovieOrderResponse) {
		editId = order.id;
		title = order.title ?? "";
		customerName = order.customer_name;
		kind = order.kind;
		source = order.source;
		status = order.status;
		comment = order.comment ?? "";
		formOpen = true;
	}

	function cancelForm() {
		formOpen = false;
		editId = null;
	}

	async function save() {
		if (!customerName.trim()) return;
		busy = true;
		error = "";
		const payload: UpsertMovieOrderRequest = {
			title: title.trim() || null,
			customer_name: customerName.trim(),
			kind,
			source,
			status,
			comment: comment.trim() || null,
		};
		const res =
			editId === null
				? await api.createMovieOrder(payload)
				: await api.updateMovieOrder(editId, payload);
		if (res.isErr()) {
			setError(res.error);
		} else {
			hint = editId === null ? "Заказ создан." : "Заказ обновлён.";
			formOpen = false;
			editId = null;
			await load();
		}
		busy = false;
	}

	async function remove(id: number) {
		if (!confirm("Удалить заказ фильма?")) return;
		removeId = id;
		error = "";
		const res = await api.deleteMovieOrder(id);
		if (res.isErr()) {
			setError(res.error);
		} else {
			hint = "Заказ удалён.";
			await load();
		}
		removeId = null;
	}

	onMount(() => {
		void load();
	});
</script>

<Card>
	<Section
		title="Заказы фильмов"
		hint="Список синхронизируется с Google-таблицей."
	>
		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		<Button variant="primary" onclick={openNew}>
			<IconPlus aria-hidden="true" />
			Создать заказ
		</Button>

		{#if formOpen}
			<form
				class="inline-form stacked"
				onsubmit={(e) => {
					e.preventDefault();
					void save();
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
					<Button variant="primary" type="submit" disabled={busy}>
						{busy ? "Сохранение..." : editId === null ? "Создать" : "Сохранить"}
					</Button>
					<Button size="sm" type="button" onclick={cancelForm}>Отмена</Button>
				</div>
			</form>
		{/if}

		{#if loaded}
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
						{#each orders as order (order.id)}
							<tr>
								<td class="mono">{order.id}</td>
								<td>{order.title ?? "—"}</td>
								<td>{order.customer_name}</td>
								<td>{kindLabels[order.kind] ?? order.kind}</td>
								<td>{sourceLabels[order.source] ?? order.source}</td>
								<td>
									<Badge tone={order.status}>
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
										onclick={() => remove(order.id)}
										disabled={removeId === order.id}
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
