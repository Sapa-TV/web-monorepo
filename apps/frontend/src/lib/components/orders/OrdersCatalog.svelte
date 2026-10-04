<script lang="ts" generics="Row extends GameOrderResponse | MovieOrderResponse">
	import { Badge, Input, Select, TableWrap } from "@sapa-tv-ru/ui-kit";
	import type {
		GameOrderResponse,
		MovieOrderResponse,
	} from "@sapa-tv-ru/api-client";

	type Mode = "games" | "movies";

	interface Props {
		mode: Mode;
		orders: Row[];
	}

	let { mode, orders }: Props = $props();

	let q = $state("");
	let status = $state("");
	let kind = $state("");
	let source = $state("");

	const statusLabels = $derived<Record<string, string>>(
		mode === "games"
			? { pending: "ожидает", completed: "пройдена", cancelled: "отмена" }
			: { pending: "ожидает", completed: "просмотрен", cancelled: "отмена" },
	);

	const kindLabels = $derived<Record<string, string>>(
		mode === "games"
			? { stream: "стрим", playthrough: "прохождение" }
			: { movie: "фильм", series: "сериал", anime: "аниме", youtube: "ютуб" },
	);

	const sourceLabels: Record<string, string> = {
		donate: "донат",
		points: "баллы",
		roulette: "рулетка",
		other: "—",
	};

	const kindOptions = $derived(
		[...new Set(orders.map((o) => o.kind as string))].sort(),
	);

	const filtered = $derived(
		orders.filter((o) => {
			if (status && o.status !== status) return false;
			if (kind && o.kind !== kind) return false;
			if (source && o.source !== source) return false;
			if (q) {
				const needle = q.toLowerCase();
				const title = o.title?.toLowerCase() ?? "";
				if (
					!title.includes(needle) &&
					!o.customer_name.toLowerCase().includes(needle)
				)
					return false;
			}
			return true;
		}),
	);

	function statusLabel(value: string): string {
		return statusLabels[value] ?? value;
	}

	function kindLabel(value: string): string {
		return kindLabels[value] ?? value;
	}
</script>

<div class="filters">
	<Input bind:value={q} placeholder="Поиск по названию или заказчику" />
	<Select bind:value={status} title="Статус">
		<option value="">все статусы</option>
		{#each Object.entries(statusLabels) as [value, label] (value)}
			<option {value}>{label}</option>
		{/each}
	</Select>
	<Select bind:value={kind} title="Тип">
		<option value="">все типы</option>
		{#each kindOptions as value (value)}
			<option {value}>{kindLabel(value)}</option>
		{/each}
	</Select>
	<Select bind:value={source} title="Источник">
		<option value="">все источники</option>
		{#each Object.entries(sourceLabels) as [value, label] (value)}
			<option {value}>{label}</option>
		{/each}
	</Select>
</div>

<p class="count mono">{filtered.length} из {orders.length}</p>

<TableWrap>
	<table>
		<thead>
			<tr>
				<th>№</th>
				<th>{mode === "games" ? "Игра" : "Название"}</th>
				<th>Заказчик</th>
				<th>Тип</th>
				<th>Источник</th>
				<th>Статус</th>
				{#if mode === "games"}
					<th>Дата</th>
				{/if}
				<th>Комментарий</th>
			</tr>
		</thead>
		<tbody>
			{#each filtered as order (order.id)}
				<tr>
					<td class="mono cell-nowrap">{order.id}</td>
					<td class="title cell-grow">{order.title ?? "—"}</td>
					<td class="cell-nowrap">{order.customer_name}</td>
					<td class="cell-nowrap">{kindLabel(order.kind)}</td>
					<td class="cell-nowrap"
						>{sourceLabels[order.source] ?? order.source}</td
					>
					<td class="cell-nowrap">
						<Badge tone={order.status}>{statusLabel(order.status)}</Badge>
					</td>
					{#if mode === "games"}
						<td class="mono cell-nowrap">
							{"completed_at" in order ? (order.completed_at ?? "") : ""}
						</td>
					{/if}
					<td class="comment">{order.comment ?? ""}</td>
				</tr>
			{/each}
		</tbody>
	</table>
</TableWrap>

<style>
	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 0.6rem;
		margin-bottom: 0.8rem;
	}

	.filters :global(.field-input) {
		flex: 1 1 10rem;
	}

	.count {
		margin: 0 0 0.8rem;
		font-size: 0.7rem;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		color: var(--on-surface-variant);
	}

	.title {
		width: 30%;
	}

	.comment {
		min-width: 16rem;
		font-size: 0.82rem;
		color: var(--on-surface-variant);
	}
</style>
