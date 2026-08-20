<script lang="ts">
	import { api } from "#lib/api";
	import {
		MessageMatcher,
		RuleTrigger,
		type ActionResponse,
		type RewardResponse,
		type RuleConditions,
		type RuleResponse,
		type UpsertRuleRequest,
	} from "@sapa-tv-ru/api-client";
	import { onMount } from "svelte";
	import IconPencil from "~icons/lucide/pencil";
	import IconPlus from "~icons/lucide/plus";
	import IconTrash2 from "~icons/lucide/trash-2";

	let rules = $state<RuleResponse[]>([]);
	let actions = $state<ActionResponse[]>([]);
	let rewards = $state<RewardResponse[]>([]);
	let rewardsError = $state("");
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let formOpen = $state(false);
	let editId = $state<number | null>(null);
	let name = $state("");
	let enabled = $state(true);
	let trigger = $state<RuleTrigger>(RuleTrigger.ChatMessage);
	let matcher = $state<MessageMatcher>(MessageMatcher.Contains);
	let pattern = $state("");
	let rewardId = $state("");
	let actionId = $state<number | null>(null);
	let busy = $state(false);
	let removeId = $state<number | null>(null);

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	async function load() {
		error = "";
		hint = "";
		const res = await api.listRules();
		if (res.isErr()) throw res.error;
		rules = res.value;
		loaded = true;
	}

	async function loadOptions() {
		try {
			const res = await api.listActions();
			if (res.isErr()) throw res.error;
			actions = res.value;
		} catch {
			// actions list stays as-is if it fails to refresh
		}
		try {
			const res = await api.listRewards();
			if (res.isErr()) throw res.error;
			rewardsError = "";
			rewards = res.value;
		} catch (err) {
			rewardsError = err instanceof Error ? err.message : String(err);
		}
	}

	function openNew() {
		editId = null;
		name = "";
		enabled = true;
		trigger = RuleTrigger.ChatMessage;
		matcher = MessageMatcher.Contains;
		pattern = "";
		rewardId = "";
		actionId = null;
		formOpen = true;
		void loadOptions();
	}

	function openEdit(rule: RuleResponse) {
		editId = rule.id;
		name = rule.name;
		enabled = rule.enabled;
		trigger = rule.trigger;
		actionId = rule.action_id;
		if (rule.conditions.trigger === "chat_message") {
			matcher = rule.conditions.matcher;
			pattern = rule.conditions.pattern ?? "";
			rewardId = "";
		} else {
			matcher = MessageMatcher.Contains;
			pattern = "";
			rewardId = rule.conditions.reward_id ?? "";
		}
		formOpen = true;
		void loadOptions();
	}

	function cancelForm() {
		formOpen = false;
	}

	function buildConditions(): RuleConditions {
		if (trigger === RuleTrigger.ChatMessage) {
			return {
				trigger: "chat_message",
				matcher,
				pattern: pattern.trim() ? pattern.trim() : null,
			};
		}
		return {
			trigger: "reward_redemption",
			reward_id: rewardId || null,
		};
	}

	async function save() {
		if (!name.trim()) return;
		if (actionId === null) {
			error = "Выбери действие для правила.";
			return;
		}
		busy = true;
		error = "";
		try {
			const payload: UpsertRuleRequest = {
				name: name.trim(),
				enabled,
				trigger,
				conditions: buildConditions(),
				action_id: actionId,
			};
			const res =
				editId === null
					? await api.createRule(payload)
					: await api.updateRule(editId, payload);
			if (res.isErr()) throw res.error;
			hint = editId === null ? "Правило создано." : "Правило обновлено.";
			formOpen = false;
			await load();
		} catch (err) {
			setError(err);
		} finally {
			busy = false;
		}
	}

	async function remove(id: number) {
		if (!confirm("Удалить правило?")) return;
		removeId = id;
		error = "";
		try {
			const res = await api.deleteRule(id);
			if (res.isErr()) throw res.error;
			hint = "Правило удалено.";
			await load();
		} catch (err) {
			setError(err);
		} finally {
			removeId = null;
		}
	}

	function actionName(id: number): string {
		return actions.find((a) => a.id === id)?.name ?? `#${id}`;
	}

	function conditionsLabel(conditions: RuleConditions): string {
		if (conditions.trigger === "chat_message") {
			return `${matcherLabel(conditions.matcher)}${
				conditions.pattern ? ` «${conditions.pattern}»` : ""
			}`;
		}
		return conditions.reward_id
			? `награда ${conditions.reward_id}`
			: "любая награда";
	}

	function matcherLabel(value: MessageMatcher): string {
		switch (value) {
			case MessageMatcher.Contains:
				return "содержит";
			case MessageMatcher.StartsWith:
				return "начинается с";
			case MessageMatcher.Equals:
				return "равно";
			case MessageMatcher.EndsWith:
				return "заканчивается на";
		}
	}

	onMount(() => {
		load().catch(setError);
		void loadOptions();
	});
</script>

<section class="card">
	<div class="section-title">Правила</div>
	<p class="section-hint">
		Событие → действие: при совпадении триггера и условий выполняется выбранное
		действие.
	</p>

	{#if error}
		<p class="alert alert--error" role="alert">{error}</p>
	{/if}
	{#if hint}
		<p class="alert alert--ok">{hint}</p>
	{/if}

	<button class="btn btn--primary" type="button" onclick={openNew}>
		<IconPlus aria-hidden="true" />
		Создать правило
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
				<span>Триггер</span>
				<select bind:value={trigger}>
					<option value={RuleTrigger.ChatMessage}>Сообщение в чате</option>
					<option value={RuleTrigger.RewardRedemption}
						>Исполнение награды</option
					>
				</select>
			</label>

			{#if trigger === RuleTrigger.ChatMessage}
				<div class="field-row">
					<label class="field">
						<span>Условие</span>
						<select bind:value={matcher}>
							<option value={MessageMatcher.Contains}>содержит</option>
							<option value={MessageMatcher.StartsWith}>начинается с</option>
							<option value={MessageMatcher.Equals}>равно</option>
							<option value={MessageMatcher.EndsWith}>заканчивается на</option>
						</select>
					</label>
					<label class="field">
						<span>Шаблон</span>
						<input
							type="text"
							placeholder="напр. !spin"
							bind:value={pattern}
							required={!pattern.trim()}
						/>
					</label>
				</div>
			{:else}
				<label class="field">
					<span>Награда</span>
					<select bind:value={rewardId}>
						{#if rewardsError}
							<option value="">Награды недоступны: {rewardsError}</option>
						{:else if rewards.length === 0}
							<option value="">Наград не найдено</option>
						{:else}
							<option value="">Любая награда</option>
							{#each rewards as reward (reward.id)}
								<option value={reward.id}>
									{reward.title} ({reward.cost}){reward.used_in_rules
										? " • в правилах"
										: ""}
								</option>
							{/each}
						{/if}
					</select>
				</label>
			{/if}

			<label class="field">
				<span>Действие</span>
				<select bind:value={actionId}>
					{#if actions.length === 0}
						<option value={null} disabled>Действий нет</option>
					{:else}
						<option value={null} disabled>Выбери действие...</option>
						{#each actions as action (action.id)}
							<option value={action.id}>{action.name}</option>
						{/each}
					{/if}
				</select>
			</label>

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
						<th>Триггер</th>
						<th>Условия</th>
						<th>Действие</th>
						<th>Вкл</th>
						<th class="actions-cell">Действия</th>
					</tr>
				</thead>
				<tbody>
					{#each rules as rule (rule.id)}
						<tr>
							<td>{rule.name}</td>
							<td
								>{rule.trigger === RuleTrigger.ChatMessage
									? "Чат"
									: "Награда"}</td
							>
							<td class="mono">{conditionsLabel(rule.conditions)}</td>
							<td>{actionName(rule.action_id)}</td>
							<td>{rule.enabled ? "да" : "нет"}</td>
							<td class="actions-cell">
								<button
									class="btn btn--sm btn--icon"
									type="button"
									onclick={() => openEdit(rule)}
									aria-label="Редактировать правило"
								>
									<IconPencil aria-hidden="true" />
								</button>
								<button
									class="btn btn--sm btn--danger btn--icon"
									type="button"
									onclick={() => remove(rule.id)}
									disabled={removeId === rule.id}
									aria-label="Удалить правило"
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

	.field-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.field-row .field {
		flex: 1;
		min-width: 160px;
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
	}

	.actions-cell .btn {
		margin-left: 4px;
	}

	.loading {
		margin-top: 12px;
	}
</style>
