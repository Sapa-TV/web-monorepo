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

<section class="card">
	<div class="section-title">Действия</div>
	<p class="section-hint">
		Экшены, которые движок выполняет при срабатывании правила.
	</p>

	{#if error}
		<p class="alert alert--error" role="alert">{error}</p>
	{/if}
	{#if hint}
		<p class="alert alert--ok">{hint}</p>
	{/if}

	<button class="btn btn--primary" type="button" onclick={openNew}>
		<IconPlus aria-hidden="true" />
		Создать действие
	</button>

	{#if formOpen}
		<form
			class="inline-form stacked"
			onsubmit={(e) => {
				e.preventDefault();
				void save();
			}}
		>
			<label class="field">
				<span>Название</span>
				<input
					type="text"
					placeholder="напр. Spin"
					bind:value={name}
					required
				/>
			</label>

			<label class="field">
				<span>Тип</span>
				<select bind:value={kind}>
					<option value="no_action">Ничего</option>
					<option value="enqueue_roulette">Рулетка</option>
					<option value="chat_reply">Ответ в чат</option>
				</select>
			</label>

			{#if kind === "chat_reply"}
				<label class="field">
					<span>Шаблон сообщения</span>
					<input
						type="text"
						placeholder="напр. &#123;username&#125;, держи рулетку &#123;cost&#125;!"
						bind:value={template}
						required
					/>
				</label>
			{/if}

			<label class="check">
				<input type="checkbox" bind:checked={enabled} />
				Включено
			</label>

			<div class="form-actions">
				<button class="btn btn--primary" type="submit" disabled={busy}>
					{busy ? "Сохранение..." : editId === null ? "Создать" : "Сохранить"}
				</button>
				<button class="btn btn--sm" type="button" onclick={cancelForm}>
					Отмена
				</button>
			</div>
		</form>
	{/if}

	{#if loaded}
		<div class="table-wrap">
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
								<button
									class="btn btn--sm btn--icon"
									type="button"
									onclick={() => openEdit(action)}
									aria-label="Редактировать действие"
								>
									<IconPencil aria-hidden="true" />
								</button>
								<button
									class="btn btn--sm btn--danger btn--icon"
									type="button"
									onclick={() => remove(action.id)}
									disabled={removeId === action.id}
									aria-label="Удалить действие"
								>
									<IconTrash2 aria-hidden="true" />
								</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{:else}
		<p class="loading">Загрузка...</p>
	{/if}
</section>

<style>
	.card {
		background: var(--surface-container);
		border: 1px solid var(--outline-variant);
		border-radius: 12px;
		padding: 18px;
		margin-bottom: 20px;
		max-width: 720px;
	}

	.section-title {
		font-size: 12px;
		font-weight: 600;
		color: var(--on-surface-variant);
		margin-bottom: 10px;
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.section-hint {
		margin: 0 0 12px;
		color: var(--on-surface-variant);
		font-size: 12px;
		line-height: 1.5;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 8px 14px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container);
		font-size: 13px;
		font-weight: 600;
		font-family: inherit;
		color: var(--on-surface);
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s,
			filter 0.15s;
	}

	.btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.btn--primary {
		background: var(--primary);
		border-color: transparent;
		color: var(--on-primary);
	}

	.btn--primary:hover:not(:disabled) {
		background: var(--primary-dim);
	}

	.btn--sm {
		padding: 6px 12px;
		border-color: var(--outline-variant);
		color: var(--on-surface);
	}

	.btn--sm:hover:not(:disabled) {
		border-color: var(--outline);
		background: var(--surface-container-high);
	}

	.btn--danger:hover:not(:disabled) {
		border-color: var(--error);
		color: var(--error);
	}

	.btn--icon {
		padding: 6px 8px;
	}

	.alert {
		max-width: 720px;
		margin: 0 0 12px;
		padding: 10px 14px;
		border-radius: 10px;
		font-size: 12px;
		line-height: 1.4;
	}

	.alert--error {
		background: color-mix(in oklch, var(--error) 12%, transparent);
		color: var(--error);
	}

	.alert--ok {
		background: color-mix(in oklch, var(--secondary) 14%, transparent);
		color: var(--secondary);
	}

	.inline-form {
		display: flex;
		flex-direction: column;
		gap: 12px;
		align-items: stretch;
		margin: 14px 0;
		padding: 14px;
		border: 1px solid var(--outline-variant);
		border-radius: 12px;
		background: var(--surface);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-size: 12px;
		color: var(--on-surface-variant);
	}

	.field input,
	.field select {
		padding: 8px 12px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container);
		color: var(--on-surface);
		font-size: 13px;
		font-family: inherit;
		outline: none;
	}

	.field input:focus,
	.field select:focus {
		border-color: var(--primary);
	}

	.check {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
		color: var(--on-surface);
	}

	.form-actions {
		display: flex;
		gap: 8px;
	}

	.table-wrap {
		margin-top: 14px;
		overflow: hidden;
		border-radius: 12px;
		border: 1px solid var(--outline-variant);
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th {
		text-align: left;
		padding: 10px 14px;
		font-size: 11px;
		color: var(--on-surface-variant);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		border-bottom: 1px solid var(--outline-variant);
		background: var(--surface-container-high);
	}

	td {
		padding: 10px 14px;
		border-bottom: 1px solid var(--outline-variant);
	}

	tr:last-child td {
		border-bottom: none;
	}

	.mono {
		font-family: "IBM Plex Mono", ui-monospace, monospace;
		font-size: 12px;
	}

	.actions-cell {
		width: 1%;
		text-align: right;
		white-space: nowrap;
	}

	.actions-cell .btn {
		margin-left: 4px;
	}

	.loading {
		margin-top: 12px;
		color: var(--on-surface-variant);
		font-size: 13px;
	}
</style>
