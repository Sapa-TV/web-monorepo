<script lang="ts">
	import {
		TRACK_WIN_INDEX,
		type TrackItem,
	} from "#lib/widgets/roulette-store.svelte";
	import RouletteTrackItem from "./RouletteTrackItem.svelte";
	import type { RarityResponse } from "@sapa-tv-ru/api-client";

	type ConnState = "connected" | "disconnected";

	interface Props {
		phase: "idle" | "spinning" | "completed" | "error" | "denied";
		stateLabel: string;
		idleText: string;
		spin: {
			entry_id: number;
			user_name: string;
			slot_id: number | null;
			slot_name: string;
			slot_rarity: string;
		} | null;
		track: TrackItem[];
		rarities: RarityResponse[];
		conn: { state: ConnState; label: string };
		badge: {
			cls: "ok" | "bad" | "missing";
			label: string;
			visible: boolean;
		};
	}

	let {
		phase,
		stateLabel,
		idleText,
		spin,
		track,
		rarities,
		conn,
		badge,
	}: Props = $props();

	const ITEM_W = 108;
	const GAP = 10;
	const VIEW_W = 676;
	const SPIN_MS = 7_000;
	const MAX_TILT = 16;
	const FULL_TILT_SPEED = 2.5;

	let offset = $state(0);
	let rolling = $state(false);
	let trackEl = $state<HTMLDivElement | null>(null);
	let arrowAngle = $state(0);

	function rarityOf(rarityId: number): RarityResponse | undefined {
		return rarities.find((r) => r.id === rarityId);
	}

	function itemImage(rarityId: number): string {
		return `/roulette/${rarityOf(rarityId)?.image ?? "common.png"}`;
	}

	function itemColor(rarityId: number): string {
		return rarityOf(rarityId)?.color ?? "var(--widget-accent)";
	}

	$effect(() => {
		if (phase !== "spinning" || track.length === 0) return;
		const target = TRACK_WIN_INDEX * (ITEM_W + GAP);
		rolling = false;
		offset = 0;
		requestAnimationFrame(() =>
			requestAnimationFrame(() => {
				offset = -target;
				rolling = true;
			}),
		);
	});

	$effect(() => {
		const el = trackEl;
		if (phase !== "spinning" || !el) {
			arrowAngle = 0;
			return;
		}
		let raf = 0;
		let lastX = 0;
		const step = ITEM_W + GAP;
		const tick = () => {
			const x = -new DOMMatrixReadOnly(getComputedStyle(el).transform).m41;
			const speed = Math.min(1, Math.abs(x - lastX) / FULL_TILT_SPEED);
			lastX = x;
			const frac = (((x / step) % 1) + 1) % 1;
			arrowAngle = MAX_TILT * Math.sin(frac * Math.PI) * speed;
			raf = requestAnimationFrame(tick);
		};
		raf = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(raf);
	});

	let showTrack = $derived(
		track.length > 0 && (phase === "spinning" || phase === "completed"),
	);
	let showFallback = $derived(
		spin != null &&
			track.length === 0 &&
			phase !== "idle" &&
			phase !== "denied",
	);
	let showConn = $derived(conn.state === "disconnected" || badge.visible);
</script>

{#if phase === "denied"}
	<div class="widget">
		<div class="widget__state">{stateLabel}</div>
		<div class="idle-text">{idleText}</div>
	</div>
{:else if phase !== "idle"}
	<div class="roulette">
		{#if spin}
			<div class="user">рулетка для: {spin.user_name}</div>
		{/if}

		{#if showTrack}
			<!-- eslint-disable svelte/no-inline-styles : dynamic animation values -->
			<div class="viewport" style:width="{VIEW_W}px">
				<img
					class="arrow"
					class:arrow--live={phase === "spinning"}
					src="/roulette/arrow.png"
					alt=""
					style:transform="translateX(-50%) rotate({arrowAngle}deg)"
				/>
				<div class="clip">
					<div
						class="track"
						bind:this={trackEl}
						style:transform="translateX({offset}px)"
						style:transition={rolling
							? `transform ${SPIN_MS}ms cubic-bezier(0.12, 0.8, 0.08, 1)`
							: "none"}
					>
						{#each track as item (item.key)}
							<RouletteTrackItem
								name={item.name}
								image={itemImage(item.rarity_id)}
								glow={itemColor(item.rarity_id)}
								win={phase === "completed" && item.key === TRACK_WIN_INDEX}
							/>
						{/each}
					</div>
				</div>
			</div>
			<!-- eslint-enable svelte/no-inline-styles -->
		{:else if showFallback && spin}
			{#if phase === "spinning"}
				<div class="spinner" aria-hidden="true"></div>
			{/if}
			<div class="spin-info">
				<div class="slot-name">{spin.slot_name}</div>
				<div class="slot-rarity">{spin.slot_rarity}</div>
			</div>
		{/if}

		<div class="result-area">
			{#if phase === "completed" && spin}
				<div class="result">выпало: {spin.slot_name}</div>
			{:else if phase === "error" && spin}
				<div class="result result--error">#{spin.entry_id} — таймаут</div>
			{/if}
		</div>
	</div>
{/if}

{#if showConn}
	<div class="conn-badge">
		<span class={`conn-dot ${conn.state}`}></span>
		<span>{conn.label}</span>
		{#if badge.visible}
			<span class={`key-badge ${badge.cls}`}>{badge.label}</span>
		{/if}
	</div>
{/if}

<style>
	.roulette {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
		padding: 8px;
	}

	.user {
		font-size: 18px;
		font-weight: 700;
		color: var(--widget-ink);
		text-shadow: 0 2px 8px var(--widget-shadow);
		letter-spacing: 0.06em;
		text-transform: uppercase;
	}

	.viewport {
		position: relative;
		overflow: hidden;
		background-image: url("/roulette/track.png");
		background-size: 100% 100%;
		padding: 34px 48px 18px;
	}

	.arrow {
		position: absolute;
		top: -12px;
		left: 50%;
		height: 66px;
		transform: translateX(-50%);
		z-index: 2;
		transform-origin: 50% 15%;
		filter: drop-shadow(0 3px 6px var(--widget-shadow));
		transition: transform 0.45s cubic-bezier(0.34, 1.56, 0.64, 1);
	}

	.arrow--live {
		transition: none;
	}

	.clip {
		overflow: hidden;
		mask-image: linear-gradient(
			to right,
			transparent,
			var(--widget-mask-solid) 40px,
			var(--widget-mask-solid) calc(100% - 40px),
			transparent
		);
	}

	.track {
		display: flex;
		gap: 10px;
		margin-left: calc(50% - 54px);
		will-change: transform;
	}

	.result-area {
		height: 72px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.result {
		font-size: 26px;
		font-weight: 800;
		color: var(--widget-ink);
		font-family: var(--font-heading);
		text-shadow: 0 2px 10px var(--widget-shadow);
	}

	.result--error {
		color: var(--widget-bad);
		font-size: 16px;
	}

	.widget {
		background: color-mix(in oklch, var(--widget-bg) 86%, transparent);
		border-radius: 20px;
		padding: 28px 32px;
		text-align: center;
		border: 1px solid var(--widget-border);
	}

	.widget__state {
		font-size: 15px;
		text-transform: uppercase;
		letter-spacing: 0.22em;
		color: var(--widget-state);
		margin-bottom: 12px;
		font-weight: 600;
	}

	.idle-text {
		font-size: 20px;
		color: var(--widget-idle);
		font-weight: 300;
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
