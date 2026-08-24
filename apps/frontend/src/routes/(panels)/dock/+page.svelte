<script lang="ts">
	import EnqueueForm from "#lib/components/dock/EnqueueForm.svelte";
	import EventLog from "#lib/components/dock/EventLog.svelte";
	import QueueTables from "#lib/components/dock/QueueTables.svelte";
	import { DockStore } from "#lib/dock/dock-store.svelte";
	import { Badge, Button } from "@sapa-tv-ru/ui-kit";
	import { onDestroy, onMount } from "svelte";
	import IconKeyRound from "~icons/lucide/key-round";
	import IconList from "~icons/lucide/list";
	import IconPlay from "~icons/lucide/play";
	import IconPlus from "~icons/lucide/plus";
	import IconRefreshCw from "~icons/lucide/refresh-cw";

	const widgetAccessKey =
		typeof window !== "undefined"
			? (new URLSearchParams(window.location.search).get("widget_access_key") ??
				"")
			: "";

	const dock = new DockStore(widgetAccessKey);

	let showEnqueue = $state(false);
	let showLog = $state(false);
	let enqName = $state("");

	async function enqueueEntry() {
		const added = await dock.enqueue(enqName);
		if (added) enqName = "";
	}

	onMount(() => dock.start());
	onDestroy(() => dock.stop());
</script>

<svelte:head>
	<title>Док-панель</title>
</svelte:head>

<header class="panel-header">
	<h1>Док-панель</h1>
	<div class="panel-header__right">
		{#if dock.keyState}
			<Badge tone={dock.keyState}>
				<IconKeyRound class="icon-sm" aria-hidden="true" />
				{dock.keyState === "ok"
					? "ключ ок"
					: dock.keyState === "missing"
						? "нет ключа"
						: "ключ неверный"}
			</Badge>
		{/if}
		<Badge tone={dock.connState} dot></Badge>
		{#if dock.connState === "connected"}
			<Badge tone={dock.widgetOnline ? "connected" : "disconnected"}>
				виджет {dock.widgetOnline ? "онлайн" : "офлайн"}
			</Badge>
		{/if}
		<Button
			size="sm"
			icon
			title="Обновить"
			aria-label="Обновить"
			onclick={() => void dock.loadAll()}
		>
			<IconRefreshCw aria-hidden="true" />
		</Button>
	</div>
</header>

<div class="toolbar">
	<Button
		variant="primary"
		type="button"
		onclick={() => void dock.dequeueNext()}
		disabled={dock.dequeueBusy}
	>
		<IconPlay aria-hidden="true" />
		{dock.dequeueLabel}
	</Button>
	<span class="next-user">{dock.nextUser}</span>
	<Button
		size="sm"
		type="button"
		onclick={() => void (showEnqueue = !showEnqueue)}
	>
		<IconPlus aria-hidden="true" />
		{showEnqueue ? "Закрыть" : "Добавить"}
	</Button>
	<span class="spacer"></span>
	<Button size="sm" type="button" onclick={() => void (showLog = !showLog)}>
		<IconList aria-hidden="true" />
		{showLog ? "Скрыть лог" : "Лог"}
	</Button>
</div>

{#if showEnqueue}
	<EnqueueForm
		bind:value={enqName}
		busy={dock.enqueueBusy}
		onsubmit={enqueueEntry}
	/>
{/if}

<QueueTables
	active={dock.active}
	done={dock.done}
	oncomplete={(id) => void dock.complete(id)}
	oncancel={(id) => void dock.cancel(id)}
/>

{#if showLog}
	<EventLog events={dock.events} />
{/if}

<style>
	h1 {
		font-size: 22px;
		margin-bottom: 20px;
		color: var(--on-background);
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 16px;
	}

	.panel-header h1 {
		font-size: 18px;
		color: var(--on-background);
		margin: 0;
	}

	.panel-header__right {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.panel-header__right :global(.icon-sm) {
		width: 0.95rem;
		height: 0.95rem;
	}

	.toolbar {
		display: flex;
		gap: 10px;
		margin-bottom: 20px;
		flex-wrap: wrap;
		align-items: center;
	}

	.next-user {
		font-size: 12px;
		color: var(--on-surface-variant);
		min-width: 80px;
	}

	.spacer {
		flex: 1;
	}
</style>
