<script lang="ts">
	import { api } from "#lib/api";
	import {
		MessageMatcher,
		RuleTrigger,
		type ActionResponse,
		type RewardResponse,
		type RuleConditions,
		type RuleResponse,
	} from "@sapa-tv-ru/api-client";
	import RuleForm from "./RuleForm.svelte";
	import IconPencil from "~icons/lucide/pencil";
	import IconPlus from "~icons/lucide/plus";
	import IconTrash2 from "~icons/lucide/trash-2";
	import { Alert, Button, Card, Section, TableWrap } from "@sapa-tv-ru/ui-kit";
	import { onMount } from "svelte";
	import { describeApiError } from "#lib/api-error-text";

	let rules = $state<RuleResponse[]>([]);
	let actions = $state<ActionResponse[]>([]);
	let rewards = $state<RewardResponse[]>([]);
	let rewardsError = $state("");
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");
	let removeId = $state<number | null>(null);

	let form: RuleForm;

	function setError(err: unknown) {
		error = describeApiError(err);
	}

	async function load() {
		error = "";
		hint = "";
		const res = await api.listRules();
		if (res.isErr()) {
			setError(res.error);
			return;
		}
		rules = res.value;
		loaded = true;
	}

	async function loadOptions() {
		const actionsRes = await api.listActions();
		if (actionsRes.isOk()) {
			actions = actionsRes.value;
		}

		const rewardsRes = await api.listRewards();
		if (rewardsRes.isErr()) {
			rewardsError = describeApiError(rewardsRes.error);
			return;
		}
		rewardsError = "";
		rewards = rewardsRes.value;
	}

	async function remove(id: number) {
		if (!confirm("Удалить правило?")) return;
		removeId = id;
		error = "";
		const res = await api.deleteRule(id);
		if (res.isErr()) {
			setError(res.error);
		} else {
			hint = "Правило удалено.";
			await load();
		}
		removeId = null;
	}

	function onsaved(message: string) {
		hint = message;
		void load();
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
		if (conditions.reward_ids.length === 0) {
			return "любая награда";
		}
		const titles = conditions.reward_ids.map(
			(id) => rewards.find((r) => r.id === id)?.title ?? id,
		);
		return `награды: ${titles.join(", ")}`;
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
		void load();
		void loadOptions();
	});
</script>

<Card>
	<Section
		title="Правила"
		hint="Событие → действие: при совпадении триггера и условий выполняется выбранное действие."
	>
		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		<Button variant="primary" onclick={() => form.openNew()}>
			<IconPlus aria-hidden="true" />
			Создать правило
		</Button>

		<RuleForm bind:this={form} {actions} {rewards} {rewardsError} {onsaved} />

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
										onclick={() => form?.openEdit(rule)}
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
