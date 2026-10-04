<script lang="ts">
	import { api } from "#lib/api";
	import { VipKind, VipStatus } from "@sapa-tv-ru/api-client";
	import type {
		VipRecordResponse,
		VipRemindersResponse,
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

	let records = $state<VipRecordResponse[]>([]);
	let reminders = $state<VipRemindersResponse | null>(null);
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let formOpen = $state(false);
	let editId = $state<number | null>(null);
	let busy = $state(false);
	let removeId = $state<number | null>(null);

	let customerName = $state("");
	let kind = $state<VipKind>(VipKind.Vip);
	let rouletteDate = $state("");
	let endDate = $state("");
	let status = $state<VipStatus>(VipStatus.Active);
	let note = $state("");

	const kindLabels: Record<string, string> = {
		vip: "VIP",
		unvip: "UnVIP",
	};
	const statusLabels: Record<string, string> = {
		active: "активна",
		done: "завершена",
		cancelled: "отменена",
	};
	const statusTones: Record<string, "spinning" | "ok" | "cancelled"> = {
		active: "spinning",
		done: "ok",
		cancelled: "cancelled",
	};

	function setError(err: unknown) {
		error = describeApiError(err);
	}

	async function load() {
		error = "";
		hint = "";
		const [listRes, remindersRes] = await Promise.all([
			api.listVipRecords(),
			api.vipReminders(),
		]);
		if (listRes.isErr()) {
			setError(listRes.error);
			return;
		}
		if (remindersRes.isErr()) {
			setError(remindersRes.error);
			return;
		}
		records = listRes.value;
		reminders = remindersRes.value;
		loaded = true;
	}

	function openNew() {
		editId = null;
		customerName = "";
		kind = VipKind.Vip;
		rouletteDate = "";
		endDate = "";
		status = VipStatus.Active;
		note = "";
		formOpen = true;
	}

	function openEdit(record: VipRecordResponse) {
		editId = record.id;
		customerName = record.customer_name;
		kind = record.kind;
		rouletteDate = record.roulette_date;
		endDate = record.end_date;
		status = record.status;
		note = record.note ?? "";
		formOpen = true;
	}

	function cancelForm() {
		formOpen = false;
		editId = null;
	}

	async function save() {
		if (!customerName.trim() || !rouletteDate) return;
		busy = true;
		error = "";
		const res =
			editId === null
				? await api.createVipRecord({
						customer_name: customerName.trim(),
						kind,
						roulette_date: rouletteDate,
						end_date: endDate || null,
						note: note.trim() || null,
					})
				: await api.updateVipRecord(editId, {
						customer_name: customerName.trim(),
						kind,
						roulette_date: rouletteDate,
						end_date: endDate,
						status,
						note: note.trim() || null,
					});
		if (res.isErr()) {
			setError(res.error);
		} else {
			hint = editId === null ? "Запись создана." : "Запись обновлена.";
			formOpen = false;
			editId = null;
			await load();
		}
		busy = false;
	}

	async function remove(id: number) {
		if (!confirm("Удалить VIP-запись?")) return;
		removeId = id;
		error = "";
		const res = await api.deleteVipRecord(id);
		if (res.isErr()) {
			setError(res.error);
		} else {
			hint = "Запись удалена.";
			await load();
		}
		removeId = null;
	}

	onMount(() => {
		void load();
	});
</script>

<Card wide>
	<Section
		title="VIP / UnVIP"
		hint="VIP выдаётся на 14 дней, UnVIP (проигрыш постоянной випки) — на 7. Даты окончания подставляются автоматически, если оставить поле пустым."
	>
		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		{#if reminders && (reminders.expiring.length > 0 || reminders.awaiting_return.length > 0)}
			<div class="reminders">
				{#if reminders.expiring.length > 0}
					<div class="reminder-block">
						<span class="reminder-title">Истекает VIP:</span>
						{#each reminders.expiring as r (r.id)}
							<span class="reminder-item">
								{r.customer_name} · до {r.end_date}
							</span>
						{/each}
					</div>
				{/if}
				{#if reminders.awaiting_return.length > 0}
					<div class="reminder-block">
						<span class="reminder-title">Вернуть VIP (unvip закончился):</span>
						{#each reminders.awaiting_return as r (r.id)}
							<span class="reminder-item">
								{r.customer_name} · с {r.end_date}
							</span>
						{/each}
					</div>
				{/if}
			</div>
		{/if}

		<Button variant="primary" onclick={openNew}>
			<IconPlus aria-hidden="true" />
			Создать запись
		</Button>

		{#if formOpen}
			<form
				class="inline-form stacked"
				onsubmit={(e) => {
					e.preventDefault();
					void save();
				}}
			>
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

				<Field label="Дата рулетки">
					<Input type="date" bind:value={rouletteDate} required />
				</Field>

				<Field
					label={editId === null
						? "Дата окончания (пусто = +14/7 дней)"
						: "Дата окончания"}
				>
					<Input type="date" bind:value={endDate} required={editId !== null} />
				</Field>

				{#if editId !== null}
					<Field label="Статус">
						<Select bind:value={status}>
							{#each Object.entries(statusLabels) as [value, label] (value)}
								<option {value}>{label}</option>
							{/each}
						</Select>
					</Field>
				{/if}

				<Field label="Заметка">
					<Input type="text" placeholder="необязательно" bind:value={note} />
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
							<th>Заказчик</th>
							<th>Тип</th>
							<th>Рулетка</th>
							<th>Окончание</th>
							<th>Статус</th>
							<th class="actions-cell">Действия</th>
						</tr>
					</thead>
					<tbody>
						{#if records.length === 0}
							<tr>
								<td class="empty" colspan="7">Записей нет.</td>
							</tr>
						{/if}
						{#each records as record (record.id)}
							<tr>
								<td class="mono cell-nowrap">{record.id}</td>
								<td class="cell-nowrap">{record.customer_name}</td>
								<td class="cell-nowrap"
									><Badge tone={record.kind === "vip" ? "ok" : "bad"}>
										{kindLabels[record.kind] ?? record.kind}
									</Badge>
								</td>
								<td class="mono cell-nowrap">{record.roulette_date}</td>
								<td class="mono cell-nowrap">{record.end_date}</td>
								<td class="cell-nowrap"
									><Badge tone={statusTones[record.status] ?? "pending"}>
										{statusLabels[record.status] ?? record.status}
									</Badge>
								</td>
								<td class="actions-cell">
									<Button
										size="sm"
										icon
										onclick={() => openEdit(record)}
										aria-label="Редактировать запись"
									>
										<IconPencil aria-hidden="true" />
									</Button>
									<Button
										size="sm"
										variant="danger"
										icon
										onclick={() => remove(record.id)}
										disabled={removeId === record.id}
										aria-label="Удалить запись"
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
	.reminders {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px 14px;
		margin-bottom: 12px;
		border: 1px solid var(--outline-variant);
		border-radius: 10px;
		background: var(--surface-container-low);
	}

	.reminder-block {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 10px;
		font-size: 12px;
	}

	.reminder-title {
		font-weight: 600;
		color: var(--on-surface);
	}

	.reminder-item {
		font-family: var(--font-mono);
		font-size: 11px;
		color: var(--on-surface-variant);
	}

	.empty {
		text-align: center;
		color: var(--on-surface-variant);
		font-size: 13px;
	}
</style>
