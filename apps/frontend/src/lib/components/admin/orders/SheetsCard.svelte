<script lang="ts">
	import { api } from "#lib/api";
	import type { ImportReportResponse } from "@sapa-tv-ru/api-client";
	import { onMount } from "svelte";
	import IconDownload from "~icons/lucide/download";
	import { Alert, Button, Card, Input, Section } from "@sapa-tv-ru/ui-kit";
	import { describeApiError } from "#lib/api-error-text";

	let configured = $state(false);
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let url = $state("");
	let busy = $state(false);
	let report = $state<ImportReportResponse | null>(null);

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
</style>
