<script lang="ts">
	import type { Snippet } from "svelte";
	import Button from "./Button.svelte";

	interface Props {
		open: boolean;
		title: string;
		confirmLabel?: string;
		cancelLabel?: string;
		danger?: boolean;
		busy?: boolean;
		onconfirm?: () => void;
		onclose?: () => void;
		children?: Snippet;
	}

	let {
		open = $bindable(false),
		title,
		confirmLabel = "Подтвердить",
		cancelLabel = "Отмена",
		danger = false,
		busy = false,
		onconfirm,
		onclose,
		children,
	}: Props = $props();

	let dialog: HTMLDivElement | null = $state(null);

	function close() {
		if (busy) return;
		open = false;
		onclose?.();
	}

	$effect(() => {
		if (!open) return;
		dialog?.focus();
		const onKey = (event: KeyboardEvent) => {
			if (event.key === "Escape") close();
		};
		document.addEventListener("keydown", onKey);
		const prevOverflow = document.body.style.overflow;
		document.body.style.overflow = "hidden";
		return () => {
			document.removeEventListener("keydown", onKey);
			document.body.style.overflow = prevOverflow;
		};
	});
</script>

{#if open}
	<div class="backdrop">
		<button
			type="button"
			class="backdrop-hit"
			aria-label={cancelLabel}
			tabindex="-1"
			onclick={close}
		></button>
		<div
			class="dialog"
			role="dialog"
			aria-modal="true"
			aria-labelledby="confirm-dialog-title"
			tabindex="-1"
			bind:this={dialog}
		>
			<h2 id="confirm-dialog-title">{title}</h2>
			{#if children}
				<div class="body">
					{@render children()}
				</div>
			{/if}
			<div class="actions">
				<Button variant="cancel" onclick={close} disabled={busy}>
					{cancelLabel}
				</Button>
				<Button
					variant={danger ? "danger" : "primary"}
					onclick={onconfirm}
					disabled={busy}
				>
					{busy ? "..." : confirmLabel}
				</Button>
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 16px;
		background: color-mix(in oklch, var(--inverse-surface) 45%, transparent);
		backdrop-filter: blur(2px);
	}

	.backdrop-hit {
		position: absolute;
		inset: 0;
		border: none;
		background: transparent;
		cursor: default;
		padding: 0;
	}

	.dialog {
		position: relative;
		width: min(420px, 100%);
		background: var(--surface-bright);
		border: 1px solid var(--outline-variant);
		border-radius: 14px;
		padding: 20px;
		box-shadow: 0 16px 40px -12px
			color-mix(in oklch, var(--inverse-surface) 55%, transparent);
		outline: none;
	}

	h2 {
		margin: 0;
		font-size: 1.05rem;
		color: var(--on-surface);
	}

	.body {
		margin-top: 8px;
		font-size: 0.9rem;
		color: var(--on-surface-variant);
		line-height: 1.45;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 18px;
	}
</style>
