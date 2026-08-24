<script lang="ts">
	import { Badge, Button, Section, TableWrap } from "@sapa-tv-ru/ui-kit";
	import IconCheck from "~icons/lucide/check";
	import IconX from "~icons/lucide/x";
	import type { QueueEntry } from "#lib/api";

	type BadgeTone =
		| "ok"
		| "missing"
		| "bad"
		| "pending"
		| "spinning"
		| "completed"
		| "error"
		| "cancelled"
		| "root"
		| "connected"
		| "disconnected";

	interface Props {
		active: QueueEntry[];
		done: QueueEntry[];
		oncomplete: (_id: number) => void;
		oncancel: (_id: number) => void;
	}

	let { active, done, oncomplete, oncancel }: Props = $props();
</script>

<Section title="Активные">
	<TableWrap>
		<table>
			<thead>
				<tr>
					<th>Имя</th>
					<th>Статус</th>
					<th>Слот</th>
					<th class="th-actions"></th>
				</tr>
			</thead>
			<tbody>
				{#if active.length === 0}
					<tr>
						<td colspan="4" class="empty-row">Нет записей</td>
					</tr>
				{:else}
					{#each active as e (e.id)}
						<tr>
							<td>{e.user_name || e.user_id}</td>
							<td
								><Badge tone={e.status.toLowerCase() as BadgeTone}
									>{e.status}</Badge
								></td
							>
							<td
								>{e.status === "Spinning"
									? e.slot_name || e.result_slot_id || "—"
									: "—"}</td
							>
							<td class="actions-cell">
								{#if e.status === "Pending" || e.status === "Error"}
									<Button
										variant="cancel"
										size="sm"
										icon
										type="button"
										onclick={() => oncancel(e.id)}
										aria-label="Отменить"
									>
										<IconX aria-hidden="true" />
									</Button>
								{:else if e.status === "Spinning"}
									<Button
										variant="complete"
										size="sm"
										icon
										type="button"
										onclick={() => oncomplete(e.id)}
										aria-label="Завершить"
									>
										<IconCheck aria-hidden="true" />
									</Button>
								{/if}
							</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</TableWrap>
</Section>

<Section title="Завершённые / Отменённые">
	<TableWrap>
		<table>
			<thead>
				<tr>
					<th>Имя</th>
					<th>Результат</th>
					<th>Статус</th>
				</tr>
			</thead>
			<tbody>
				{#if done.length === 0}
					<tr>
						<td colspan="3" class="empty-row">Нет записей</td>
					</tr>
				{:else}
					{#each [...done].reverse() as e (e.id)}
						<tr>
							<td>{e.user_name || e.user_id}</td>
							<td
								>{e.status === "Completed" ? e.slot_name || "✔" : "отменён"}</td
							>
							<td>
								<Badge tone={e.status.toLowerCase() as BadgeTone}
									>{e.status}</Badge
								></td
							>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</TableWrap>
</Section>

<style>
	.empty-row {
		color: var(--on-surface-variant);
		font-style: italic;
		padding: 14px;
		text-align: center;
	}

	.th-actions {
		width: 1%;
	}
</style>
