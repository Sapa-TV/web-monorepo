<script lang="ts">
	import { page } from "$app/state";
	import { resolve } from "$app/paths";
	import IconMoon from "~icons/lucide/moon";
	import IconSun from "~icons/lucide/sun";
	import DonateToggle from "./DonateToggle.svelte";
	import { GIT_SHA } from "#lib/build-info";
	import { setLocalStorage } from "#lib/internal/storage";

	let isStream = $derived(page.url.pathname === "/stream");

	let dark = $state<"light" | "dark">(
		typeof document !== "undefined"
			? document.documentElement.dataset.theme === "dark"
				? "dark"
				: "light"
			: "light",
	);

	$effect(() => {
		if (typeof document === "undefined") return;
		const theme = dark === "dark" ? "dark" : "light";
		document.documentElement.dataset.theme = theme;
		setLocalStorage("theme", theme);
	});
</script>

<nav class="site-nav" aria-label="Навигация по сайту">
	<div class="site-nav-inner">
		<a class="brand" href={resolve("")}>
			<span class="brand-badge" aria-hidden="true">ST</span>
			<span>Sapa TV</span>
			{#if isStream}
				<span class="badge-live"><span class="live-dot"></span>LIVE</span>
			{/if}
		</a>

		<div class="nav-actions">
			<span class="build-sha" title="Build commit">{GIT_SHA}</span>
			<!-- <a class="nav-link" href="/links">
				<IconBook aria-hidden="true" />
				Каталог
			</a> -->
			<DonateToggle />
			<button
				class="theme-toggle"
				type="button"
				onclick={() => (dark = dark === "dark" ? "light" : "dark")}
			>
				{#if dark}
					<IconSun aria-hidden="true" />
				{:else}
					<IconMoon aria-hidden="true" />
				{/if}
			</button>
		</div>
	</div>
</nav>

<style>
	.site-nav {
		position: sticky;
		top: 0;
		z-index: 50;
		height: var(--site-nav-h);
		background: var(--surface-bright);
		border-bottom: 1px solid var(--outline-variant);
	}

	.site-nav-inner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		height: 100%;
		padding: 0 1.5rem;
		gap: 1rem;
	}

	.brand {
		display: inline-flex;
		align-items: center;
		gap: 0.6rem;
		font-family: var(--font-heading);
		font-size: 1.05rem;
		font-weight: 800;
		letter-spacing: -0.01em;
		color: var(--on-background);
		text-decoration: none;
		white-space: nowrap;
	}

	.brand-badge {
		width: 1.5rem;
		height: 1.5rem;
		border-radius: 0.4rem;
		background: var(--primary);
		color: var(--on-primary);
		display: grid;
		place-items: center;
		font-family: var(--font-mono);
		font-size: 0.72rem;
		font-weight: 600;
		letter-spacing: 0.02em;
		box-shadow: inset 0 0 0 1px
			color-mix(in oklch, var(--on-primary) 25%, transparent);
	}

	.nav-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.theme-toggle {
		appearance: none;
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container-low);
		color: var(--on-surface);
		border-radius: 999px;
		padding: 0.45rem 0.9rem;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		transition:
			border-color 0.15s,
			background 0.15s,
			color 0.15s;
		font-family: inherit;
	}

	.theme-toggle:hover {
		border-color: var(--primary);
		color: var(--primary);
	}

	.theme-toggle :global(svg) {
		width: 1rem;
		height: 1rem;
		flex-shrink: 0;
	}

	.build-sha {
		font-family: var(--font-mono);
		font-size: 0.72rem;
		color: var(--on-surface-variant);
		user-select: all;
	}

	.badge-live {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		background-color: var(--error);
		color: var(--on-error);
		padding: 0.15rem 0.6rem;
		border-radius: 6px;
		font-weight: 700;
		font-size: 0.72rem;
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.live-dot {
		width: 0.4rem;
		height: 0.4rem;
		border-radius: 50%;
		background: currentColor;
		animation: pulse 1.6s ease-in-out infinite;
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
</style>
