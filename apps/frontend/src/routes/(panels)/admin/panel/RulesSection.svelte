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

<Card>
	<Section title="Правила">
		<p class="section-hint">
			Событие → действие: при совпадении триггера и условий выполняется
			выбранное действие.
		</p>

		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		<Button variant="primary" onclick={openNew}>
			<IconPlus aria-hidden="true" />
			Создать правило
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

				<Field label="Триггер">
					<Select bind:value={trigger}>
						<option value={RuleTrigger.ChatMessage}>Сообщение в чате</option>
						<option value={RuleTrigger.RewardRedemption}
							>Исполнение награды</option
						>
					</Select>
				</Field>

				{#if trigger === RuleTrigger.ChatMessage}
					<div class="field-row">
						<Field label="Условие">
							<Select bind:value={matcher}>
								<option value={MessageMatcher.Contains}>содержит</option>
								<option value={MessageMatcher.StartsWith}>начинается с</option>
								<option value={MessageMatcher.Equals}>равно</option>
								<option value={MessageMatcher.EndsWith}>заканчивается на</option
								>
							</Select>
						</Field>
						<Field label="Шаблон">
							<Input
								type="text"
								placeholder="напр. !spin"
								bind:value={pattern}
								required={!pattern.trim()}
							/>
						</Field>
					</div>
				{:else}
					<Field label="Награда">
						<Select bind:value={rewardId}>
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
						</Select>
					</Field>
				{/if}

				<Field label="Действие">
					<Select bind:value={actionId}>
						{#if actions.length === 0}
							<option value={null} disabled>Действий нет</option>
						{:else}
							<option value={null} disabled>Выбери действие...</option>
							{#each actions as action (action.id)}
								<option value={action.id}>{action.name}</option>
							{/each}
						{/if}
					</Select>
				</Field>

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
									<Button
										size="sm"
										icon
										onclick={() => openEdit(rule)}
										aria-label="Редактировать правило"
									>
										<IconPencil aria-hidden="true" />
									</Button>
									<Button
										size="sm"
										variant="danger"
										icon
										onclick={() => remove(rule.id)}
										disabled={removeId === rule.id}
										aria-label="Удалить правило"
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
	.field-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.field-row :global(.field) {
		flex: 1;
		min-width: 160px;
	}
</style>
