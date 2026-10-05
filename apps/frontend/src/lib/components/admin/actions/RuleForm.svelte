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
	import {
		Alert,
		Button,
		Checkbox,
		Field,
		Input,
		Select,
	} from "@sapa-tv-ru/ui-kit";
	import { describeApiError } from "#lib/api-error-text";

	interface Props {
		actions: ActionResponse[];
		rewards: RewardResponse[];
		rewardsError: string;
		onsaved: (_message: string) => void;
	}

	let { actions, rewards, rewardsError, onsaved }: Props = $props();

	let formOpen = $state(false);
	let editId = $state<number | null>(null);
	let name = $state("");
	let enabled = $state(true);
	let trigger = $state<RuleTrigger>(RuleTrigger.ChatMessage);
	let matcher = $state<MessageMatcher>(MessageMatcher.Contains);
	let pattern = $state("");
	let rewardIds = $state<string[]>([]);
	let rewardPlatform = $state<number | null>(null);
	let actionId = $state<number | null>(null);
	let busy = $state(false);
	let formError = $state("");

	const PLATFORM_LABELS: Record<number, string> = {
		1: "Twitch",
		3: "VK Video Live",
	};

	let filteredRewards = $derived(
		rewardPlatform === null
			? rewards
			: rewards.filter((r) => r.platform === rewardPlatform),
	);

	function platformLabel(id: number): string {
		return PLATFORM_LABELS[id] ?? `Платформа ${id}`;
	}

	function toggleReward(id: string, checked: boolean) {
		rewardIds = checked
			? [...rewardIds, id]
			: rewardIds.filter((r) => r !== id);
	}

	export function openNew() {
		editId = null;
		name = "";
		enabled = true;
		trigger = RuleTrigger.ChatMessage;
		matcher = MessageMatcher.Contains;
		pattern = "";
		rewardIds = [];
		rewardPlatform = null;
		actionId = null;
		formError = "";
		formOpen = true;
	}

	export function openEdit(rule: RuleResponse) {
		editId = rule.id;
		name = rule.name;
		enabled = rule.enabled;
		trigger = rule.trigger;
		actionId = rule.action_id;
		if (rule.conditions.trigger === "chat_message") {
			matcher = rule.conditions.matcher;
			pattern = rule.conditions.pattern ?? "";
			rewardIds = [];
			rewardPlatform = null;
		} else {
			matcher = MessageMatcher.Contains;
			pattern = "";
			rewardIds = rule.conditions.reward_ids;
			rewardPlatform = rule.conditions.platform ?? null;
		}
		formError = "";
		formOpen = true;
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
			reward_ids: rewardIds,
			platform: rewardPlatform,
		};
	}

	async function save() {
		if (!name.trim()) return;
		if (actionId === null) {
			formError = "Выбери действие для правила.";
			return;
		}
		busy = true;
		formError = "";
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
		if (res.isErr()) {
			formError = describeApiError(res.error);
		} else {
			const message =
				editId === null ? "Правило создано." : "Правило обновлено.";
			formOpen = false;
			onsaved(message);
		}
		busy = false;
	}
</script>

{#if formOpen}
	<form
		class="inline-form stacked"
		onsubmit={(e) => {
			e.preventDefault();
			void save();
		}}
	>
		{#if formError}
			<Alert tone="error">{formError}</Alert>
		{/if}

		<Field label="Название">
			<Input type="text" placeholder="напр. Spin" bind:value={name} required />
		</Field>

		<Field label="Триггер">
			<Select bind:value={trigger}>
				<option value={RuleTrigger.ChatMessage}>Сообщение в чате</option>
				<option value={RuleTrigger.RewardRedemption}>Исполнение награды</option>
			</Select>
		</Field>

		{#if trigger === RuleTrigger.ChatMessage}
			<div class="field-row">
				<Field label="Условие">
					<Select bind:value={matcher}>
						<option value={MessageMatcher.Contains}>содержит</option>
						<option value={MessageMatcher.StartsWith}>начинается с</option>
						<option value={MessageMatcher.Equals}>равно</option>
						<option value={MessageMatcher.EndsWith}>заканчивается на</option>
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
			<Field label="Платформа">
				<Select bind:value={rewardPlatform}>
					<option value={null}>Любая платформа</option>
					{#each Object.entries(PLATFORM_LABELS) as [id, label] (id)}
						<option value={Number(id)}>{label}</option>
					{/each}
				</Select>
			</Field>

			<Field label="Награды (пусто — любая награда)">
				{#if rewardsError}
					<Alert tone="error">Награды недоступны: {rewardsError}</Alert>
				{:else if filteredRewards.length === 0}
					<p class="empty-rewards">Наград не найдено</p>
				{:else}
					<div class="rewards-list">
						{#each filteredRewards as reward (reward.id)}
							<Checkbox
								checked={rewardIds.includes(reward.id)}
								onchange={(e) =>
									toggleReward(reward.id, e.currentTarget.checked)}
							>
								{reward.title} ({reward.cost}) · {platformLabel(
									reward.platform,
								)}{reward.used_in_rules ? " • в правилах" : ""}
							</Checkbox>
						{/each}
					</div>
				{/if}
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

	.rewards-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
		max-height: 240px;
		overflow-y: auto;
		padding: 4px 0;
	}

	.empty-rewards {
		margin: 0;
		font-size: 13px;
		color: var(--on-surface-variant);
	}
</style>
