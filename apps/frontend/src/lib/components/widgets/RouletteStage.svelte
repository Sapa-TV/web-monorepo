<script lang="ts">
	type ConnState = "connected" | "disconnected";

	interface Props {
		phase: "idle" | "spinning" | "completed" | "error" | "denied";
		stateLabel: string;
		idleText: string;
		spin: {
			entry_id: number;
			user_name: string;
			slot_name: string;
			slot_rarity: string;
		} | null;
		conn: { state: ConnState; label: string };
		badge: {
			cls: "ok" | "bad" | "missing";
			label: string;
			visible: boolean;
		};
	}

	let { phase, stateLabel, idleText, spin, conn, badge }: Props = $props();
</script>

<div class="widget">
	<div class="widget__state">
		<span class="state-dot" aria-hidden="true"></span>
		{stateLabel}
	</div>

	{#if phase === "spinning"}
		<div class="spinner" aria-hidden="true"></div>
	{/if}

	{#if phase === "idle" || phase === "denied"}
		<div class="idle-text">{idleText}</div>
	{/if}

	{#if spin && phase !== "idle" && phase !== "denied"}
		<div class="spin-info">
			<div class="user-name">{spin.user_name}</div>
			<div class="slot-name">{spin.slot_name}</div>
			<div class="slot-rarity">{spin.slot_rarity}</div>
			<div class="entry-id">
				{phase === "error"
					? `#${spin.entry_id} — таймаут`
					: `#${spin.entry_id}`}
			</div>
		</div>
	{/if}
</div>

<div class="conn-badge">
	<span class={`conn-dot ${conn.state}`}></span>
	<span>{conn.label}</span>
	{#if badge.visible}
		<span class={`key-badge ${badge.cls}`}>{badge.label}</span>
	{/if}
</div>

<style>
	.widget {
		position: relative;
		background: color-mix(in oklch, var(--widget-bg) 86%, transparent);
		backdrop-filter: blur(14px);
		border-radius: 20px;
		padding: 44px 52px;
		text-align: center;
		min-width: 360px;
		box-shadow:
			0 18px 60px var(--widget-shadow),
			inset 0 1px 0 var(--widget-hairline),
			inset 0 0 0 1px var(--widget-hairline-soft);
		border: 1px solid var(--widget-border);
		overflow: hidden;
	}

	.widget::before {
		content: "";
		position: absolute;
		inset: 0;
		pointer-events: none;
		background-image: repeating-linear-gradient(
			to bottom,
			transparent 0 2px,
			var(--widget-scanline) 2px 3px
		);
		border-radius: inherit;
	}

	.widget__state {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		font-size: 13px;
		text-transform: uppercase;
		letter-spacing: 0.22em;
		color: var(--widget-state);
		margin-bottom: 18px;
		font-weight: 600;
	}

	.state-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: currentColor;
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.3;
		}
	}

	.idle-text {
		font-size: 26px;
		color: var(--widget-idle);
		font-weight: 300;
		letter-spacing: 0.01em;
	}

	.user-name {
		font-size: 20px;
		font-weight: 600;
		margin-bottom: 8px;
		color: var(--widget-ink);
	}

	.slot-name {
		font-size: 40px;
		font-weight: 800;
		margin-bottom: 6px;
		color: var(--widget-accent);
		font-family: var(--font-heading);
		letter-spacing: -0.01em;
	}

	.slot-rarity {
		font-size: 15px;
		font-weight: 500;
		opacity: 0.8;
		color: var(--widget-ink-muted);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	.entry-id {
		font-size: 12px;
		color: var(--widget-entry);
		margin-top: 14px;
		font-family: var(--font-mono);
		letter-spacing: 0.04em;
	}

	.spinner {
		width: 48px;
		height: 48px;
		border: 3px solid var(--widget-track);
		border-top-color: var(--widget-spinner);
		border-right-color: var(--widget-spinner);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
		margin: 0 auto 18px;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.conn-badge {
		position: fixed;
		bottom: 12px;
		right: 12px;
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--widget-ink-faint);
		background: var(--widget-overlay);
		padding: 6px 12px;
		border-radius: 6px;
		font-family: var(--font-mono);
		letter-spacing: 0.03em;
	}

	.conn-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}

	.conn-dot.connected {
		background: var(--widget-ok-dot);
	}

	.conn-dot.disconnected {
		background: var(--widget-bad-dot);
	}

	.key-badge {
		padding: 2px 10px;
		border-radius: 6px;
		font-size: 11px;
		font-weight: 600;
	}

	.key-badge.ok {
		background: var(--widget-ok-dim);
		color: var(--widget-ok);
	}

	.key-badge.missing {
		background: var(--widget-missing-dim);
		color: var(--widget-missing);
	}

	.key-badge.bad {
		background: var(--widget-bad-dim);
		color: var(--widget-bad);
	}
</style>
