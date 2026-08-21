<script lang="ts">
	import { api } from "#lib/api";
	import type {
		ActionKind,
		ActionResponse,
		UpsertActionRequest,
	} from "@sapa-tv-ru/api-client";
	import { onMount } from "svelte";
	import IconPencil from "~icons/lucide/pencil";
	import IconPlus from "~icons/lucide/plus";
	import IconTrash2 from "~icons/lucide/trash-2";
	import {
		Alert,
		Button,
		Card,
		Checkbox,
		Field,
		Input,
		Section,
		Select,
		TableWrap,
	} from "@sapa-tv-ru/ui-kit";

	type KindType = ActionKind["type"];

	let actions = $state<ActionResponse[]>([]);
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let formOpen = $state(false);
	let editId = $state<number | null>(null);
	let name = $state("");
	let kind = $state<KindType>("enqueue_roulette");
	let template = $state("");
	let enabled = $state(true);
	let busy = $state(false);
	let removeId = $state<number | null>(null);

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	async function load() {
		error = "";
		hint = "";
		const res = await api.listActions();
		if (res.isErr()) throw res.error;
		actions = res.value;
		loaded = true;
	}

	function openNew() {
		editId = null;
		name = "";
		kind = "enqueue_roulette";
		template = "";
		enabled = true;
		formOpen = true;
	}

	function openEdit(action: ActionResponse) {
		editId = action.id;
		name = action.name;
		kind = action.kind.type;
		template =
			action.kind.type === "chat_reply" ? action.kind.message_template : "";
		enabled = action.enabled;
		formOpen = true;
	}

	function cancelForm() {
		formOpen = false;
	}

	function buildKind(): ActionKind {
		if (kind === "chat_reply") {
			return { type: "chat_reply", message_template: template.trim() };
		}
		if (kind === "enqueue_roulette") {
			return { type: "enqueue_roulette" };
		}
		return { type: "no_action" };
	}

	async function save() {
		if (!name.trim()) return;
		busy = true;
		error = "";
		try {
			const payload: UpsertActionRequest = {
				name: name.trim(),
				kind: buildKind(),
				enabled,
			};
			const res =
				editId === null
					? await api.createAction(payload)
					: await api.updateAction(editId, payload);
			if (res.isErr()) throw res.error;
			hint = editId === null ? "Действие создано." : "Действие обновлено.";
			formOpen = false;
			await load();
		} catch (err) {
			setError(err);
		} finally {
			busy = false;
		}
	}

	async function remove(id: number) {
		if (!confirm("Удалить действие?")) return;
		removeId = id;
		error = "";
		try {
			const res = await api.deleteAction(id);
			if (res.isErr()) throw res.error;
			hint = "Действие удалено.";
			await load();
		} catch (err) {
			setError(err);
		} finally {
			removeId = null;
		}
	}

	function kindLabel(action: ActionResponse): string {
		if (action.kind.type === "no_action") return "Ничего";
		if (action.kind.type === "enqueue_roulette") return "Рулетка";
		return "Ответ в чат";
	}

	function templateValue(action: ActionResponse): string {
		return action.kind.type === "chat_reply"
			? action.kind.message_template
			: "—";
	}

	onMount(() => {
		load().catch(setError);
	});
</script>

<Card>
	<Section title="Действия">
		<p class="section-hint">
			Экшены, которые движок выполняет при срабатывании правила.
		</p>

		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		<Button variant="primary" onclick={openNew}>
			<IconPlus aria-hidden="true" />
			Создать действие
		</Button>

		{#if formOpen}
			<form
				class="inline-form stacked"
				onsubmit={(e) => {
					e.preventDefault();
					void save();
				}}
			>
				<Field label="Название">
					<Input
						type="text"
						placeholder="напр. Spin"
						bind:value={name}
						required
					/>
				</Field>

				<Field label="Тип">
					<Select bind:value={kind}>
						<option value="no_action">Ничего</option>
						<option value="enqueue_roulette">Рулетка</option>
						<option value="chat_reply">Ответ в чат</option>
					</Select>
				</Field>

				{#if kind === "chat_reply"}
					<Field label="Шаблон сообщения">
						<Input
							type="text"
							placeholder="напр. &#123;username&#125;, держи рулетку &#123;cost&#125;!"
							bind:value={template}
							required
						/>
					</Field>
				{/if}

				<Checkbox bind:checked={enabled}>Включено</Checkbox>

				<div class="form-actions">
					<Button variant="primary" type="submit" disabled={busy}>
						{busy ? "Сохранение..." : editId === null ? "Создать" : "Сохранить"}
					</Button>
					<Button size="sm" onclick={cancelForm}>Отмена</Button>
				</div>
			</form>
		{/if}

		{#if loaded}
			<TableWrap>
				<table>
					<thead>
						<tr>
							<th>Имя</th>
							<th>Тип</th>
							<th>Параметры</th>
							<th>Вкл</th>
							<th class="actions-cell">Действия</th>
						</tr>
					</thead>
					<tbody>
						{#each actions as action (action.id)}
							<tr>
								<td>{action.name}</td>
								<td>{kindLabel(action)}</td>
								<td class="mono">{templateValue(action)}</td>
								<td>{action.enabled ? "да" : "нет"}</td>
								<td class="actions-cell">
									<Button
										size="sm"
										icon
										onclick={() => openEdit(action)}
										aria-label="Редактировать действие"
									>
										<IconPencil aria-hidden="true" />
									</Button>
									<Button
										size="sm"
										variant="danger"
										icon
										onclick={() => remove(action.id)}
										disabled={removeId === action.id}
										aria-label="Удалить действие"
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
