<script lang="ts">
	import { api } from "#lib/api";
	import type { ImportReportResponse } from "@sapa-tv-ru/api-client";
	import { onMount } from "svelte";
	import IconDownload from "~icons/lucide/download";
	import IconRefreshCw from "~icons/lucide/refresh-cw";
	import { Alert, Button, Card, Input, Section } from "@sapa-tv-ru/ui-kit";
	import { describeApiError } from "#lib/api-error-text";

	let configured = $state(false);
	let lastSyncedAt = $state<string | null>(null);
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let url = $state("");
	let busy = $state(false);
	let syncBusy = $state(false);
	let report = $state<ImportReportResponse | null>(null);

	const lastSyncText = $derived(
		lastSyncedAt
			? new Date(lastSyncedAt).toLocaleString("ru-RU")
			: "ещё не было",
	);

	function setError(err: unknown) {
		error = describeApiError(err);
	}

	async function load() {
		const res = await api.sheetsStatus();
		if (res.isErr()) {
			setError(res.error);
			return;
		}
		configured = res.value.configured;
		lastSyncedAt = res.value.last_synced_at ?? null;
		if (res.value.spreadsheet_id) {
			url = `https://docs.google.com/spreadsheets/d/${res.value.spreadsheet_id}/edit`;
		}
		loaded = true;
	}

	async function runImport() {
		if (!url.trim()) return;
		busy = true;
		error = "";
		report = null;
		const res = await api.importOrders({ spreadsheet_url: url.trim() });
		if (res.isErr()) {
			setError(res.error);
		} else {
			report = res.value;
			hint = "Импорт завершён. Обнови вкладки, чтобы увидеть новые записи.";
		}
		busy = false;
	}

	async function runSync() {
		syncBusy = true;
		error = "";
		const res = await api.syncOrders();
		if (res.isErr()) {
			setError(res.error);
		} else {
			lastSyncedAt = res.value.synced_at;
			hint = `Синхронизировано: игры ${res.value.games}, фильмы ${res.value.movies}, VIP ${res.value.vip}.`;
		}
		syncBusy = false;
	}

	onMount(() => {
		void load();
	});
</script>

<Card wide>
	<Section
		title="Google Sheets"
		hint="Импорт из таблицы в базу сайта. Дубликаты (по заказчику и названию) пропускаются. Ссылка сохраняется и используется для дальнейшей синхронизации."
	>
		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		{#if loaded && !configured}
			<Alert tone="error">
				Интеграция не настроена: задай GOOGLE_SERVICE_ACCOUNT_KEY_PATH на
				сервере.
			</Alert>
		{/if}

		<form
			class="inline-form"
			onsubmit={(e) => {
				e.preventDefault();
				void runImport();
			}}
		>
			<Input
				type="text"
				placeholder="https://docs.google.com/spreadsheets/d/…"
				bind:value={url}
				required
			/>
			<Button variant="primary" type="submit" disabled={busy || !configured}>
				<IconDownload aria-hidden="true" />
				{busy ? "Импорт..." : "Импортировать"}
			</Button>
		</form>

		{#if loaded && configured}
			<div class="sync-row">
				<Button
					size="sm"
					type="button"
					onclick={() => void runSync()}
					disabled={syncBusy}
				>
					<IconRefreshCw aria-hidden="true" />
					{syncBusy ? "Синхронизация..." : "Синхронизировать сейчас"}
				</Button>
				<span class="sync-status">Последний синк: {lastSyncText}</span>
			</div>
		{/if}

		{#if report}
			<div class="report">
				<span class="report-row">
					Игры: +{report.games.imported}, пропущено {report.games.skipped}
				</span>
				<span class="report-row">
					Фильмы: +{report.movies.imported}, пропущено {report.movies.skipped}
				</span>
				<span class="report-row">
					VIP: +{report.vip.imported}, пропущено {report.vip.skipped}
				</span>
			</div>
		{/if}
	</Section>
</Card>

<style>
	.inline-form {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.inline-form :global(.field-input) {
		flex: 1 1 auto;
	}

	.report {
		display: flex;
		flex-direction: column;
		gap: 4px;
		margin-top: 10px;
		font-family: var(--font-mono);
		font-size: 12px;
		color: var(--on-surface-variant);
	}

	.sync-row {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 12px;
	}

	.sync-status {
		font-family: var(--font-mono);
		font-size: 11px;
		color: var(--on-surface-variant);
	}
</style>
