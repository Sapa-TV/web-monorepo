<script lang="ts">
	import { onMount } from "svelte";
	import IconBell from "~icons/lucide/bell";
	import IconBellOff from "~icons/lucide/bell-off";
	import { initPush, pushState, togglePush } from "#lib/push/store.svelte";

	interface Props {
		label?: boolean;
	}

	let { label = false }: Props = $props();
	let busy = $state(false);

	onMount(() => {
		void initPush();
	});

	let visible = $derived(
		pushState.status !== "loading" &&
			pushState.status !== "unsupported" &&
			pushState.status !== "disabled",
	);
	let subscribed = $derived(pushState.status === "subscribed");
	let denied = $derived(pushState.status === "denied");
	let title = $derived(
		denied
			? "Уведомления заблокированы в настройках браузера"
			: subscribed
				? "Выключить уведомления о стриме"
				: "Включить уведомления о стриме",
	);

	function handleClick() {
		busy = true;
		void togglePush().finally(() => {
			busy = false;
		});
	}
</script>

{#if visible}
	<button
		class="push-toggle"
		class:active={subscribed}
		class:with-label={label}
		type="button"
		{title}
		aria-label={title}
		aria-pressed={subscribed}
		disabled={busy || denied}
		onclick={handleClick}
	>
		{#if denied}
			<IconBellOff aria-hidden="true" />
		{:else}
			<IconBell aria-hidden="true" />
		{/if}
		{#if label}
			<span>
				{#if denied}
					Уведомления заблокированы
				{:else if pushState.iosHint}
					Установи приложение для уведомлений
				{:else if subscribed}
					Уведомления включены
				{:else}
					Уведомления
				{/if}
			</span>
		{/if}
	</button>
{/if}

<style>
	.push-toggle {
		appearance: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container-low);
		color: var(--on-surface);
		border-radius: 999px;
		padding: 0.45rem;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			border-color 0.15s,
			background 0.15s,
			color 0.15s;
		font-family: inherit;
	}

	.push-toggle:hover:not(:disabled) {
		border-color: var(--primary);
		color: var(--primary);
	}

	.push-toggle.active {
		border-color: var(--primary);
		color: var(--primary);
	}

	.push-toggle:disabled {
		cursor: not-allowed;
		opacity: 0.55;
	}

	.push-toggle :global(svg) {
		width: 1rem;
		height: 1rem;
		flex-shrink: 0;
	}

	.push-toggle.with-label {
		width: 100%;
		min-height: 2.75rem;
		justify-content: flex-start;
		padding: 0 0.9rem;
	}
</style>
