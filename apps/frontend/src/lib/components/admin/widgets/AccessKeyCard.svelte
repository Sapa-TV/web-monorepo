<script lang="ts">
	import { api } from "#lib/api";
	import {
		Alert,
		Button,
		Card,
		Code,
		ConfirmDialog,
		Section,
	} from "@sapa-tv-ru/ui-kit";
	import { onDestroy } from "svelte";
	import IconCheck from "~icons/lucide/check";
	import IconCopy from "~icons/lucide/copy";
	import IconRefreshCw from "~icons/lucide/refresh-cw";

	interface Props {
		accessKey: string;
		onrotated: () => void;
	}

	let { accessKey, onrotated }: Props = $props();

	let copied = $state(false);
	let wakBusy = $state(false);
	let error = $state("");
	let hint = $state("");
	let rotateOpen = $state(false);

	let copyTimer: ReturnType<typeof setTimeout> | null = null;
	const COPY_FEEDBACK_MS = 1500;

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	async function copyWak() {
		try {
			await navigator.clipboard.writeText(accessKey);
			copied = true;
			if (copyTimer) clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = false), COPY_FEEDBACK_MS);
		} catch {
			// clipboard unavailable
		}
	}

	async function rotateWak() {
		wakBusy = true;
		error = "";
		try {
			const res = await api.rotateWidgetAccessKey();
			if (res.isErr()) throw res.error;
			rotateOpen = false;
			onrotated();
			hint = "Access key обновлён.";
		} catch (err) {
			setError(err);
		} finally {
			wakBusy = false;
		}
	}

	onDestroy(() => {
		if (copyTimer) clearTimeout(copyTimer);
	});
</script>

<Card>
	<Section title="Access key">
		<p class="section-hint">Ключ для доступа к панелям/виджетам со стримера.</p>

		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		<div class="key-row">
			<Code block title={accessKey}>{accessKey}</Code>
			<Button
				size="sm"
				type="button"
				onclick={copyWak}
				disabled={!accessKey}
				aria-label="Скопировать ключ"
			>
				{#if copied}
					<IconCheck aria-hidden="true" />
				{:else}
					<IconCopy aria-hidden="true" />
				{/if}
			</Button>
		</div>
		<Button
			variant="primary"
			type="button"
			onclick={() => (rotateOpen = true)}
			disabled={wakBusy}
		>
			<IconRefreshCw aria-hidden="true" />
			{wakBusy ? "Генерация..." : "Сгенерировать новый"}
		</Button>
	</Section>
</Card>

<ConfirmDialog
	bind:open={rotateOpen}
	title="Сгенерировать новый access_key?"
	confirmLabel="Сгенерировать"
	danger
	busy={wakBusy}
	onconfirm={rotateWak}
>
	<p>
		Старый ключ перестанет работать. Панели и виджеты со старым ключом потеряют
		доступ.
	</p>
</ConfirmDialog>

<style>
	.key-row {
		display: flex;
		gap: 8px;
		align-items: center;
		margin-bottom: 12px;
	}
</style>
