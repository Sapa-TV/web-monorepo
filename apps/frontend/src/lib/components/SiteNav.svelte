<script lang="ts">
	import { page } from "$app/state";
	import { resolve } from "$app/paths";
	import { afterNavigate } from "$app/navigation";
	import { onMount } from "svelte";
	import IconMoon from "~icons/lucide/moon";
	import IconSun from "~icons/lucide/sun";
	import IconMenu from "~icons/lucide/menu";
	import IconUserRound from "~icons/lucide/user-round";
	import IconX from "~icons/lucide/x";
	import DonateToggle from "./DonateToggle.svelte";
	import PushToggle from "./PushToggle.svelte";
	import { GIT_SHA } from "#lib/build-info";
	import { setLocalStorage } from "#lib/internal/storage";

	let isStream = $derived(page.url.pathname === "/stream");
	let menuOpen = $state(false);

	let navEl: HTMLElement;

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

	afterNavigate(() => {
		menuOpen = false;
	});

	onMount(() => {
		const query = window.matchMedia("(min-width: 641px)");
		const close = () => {
			menuOpen = false;
		};
		query.addEventListener("change", close);
		return () => query.removeEventListener("change", close);
	});

	function handleWindowClick(event: MouseEvent) {
		if (!menuOpen) return;
		const target = event.target;
		if (!(target instanceof Node)) return;
		if (navEl?.querySelector(".site-menu")?.contains(target)) return;
		if (navEl?.querySelector(".menu-toggle")?.contains(target)) return;
		menuOpen = false;
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") menuOpen = false;
	}
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<nav class="site-nav" aria-label="Навигация по сайту" bind:this={navEl}>
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
			<a class="nav-link" href={resolve("games")}>Игры</a>
			<a class="nav-link" href={resolve("movies")}>Фильмы</a>
			<a
				class="nav-link nav-link--icon"
				href={resolve("account")}
				aria-label="Аккаунт"
				title="Аккаунт"
			>
				<IconUserRound aria-hidden="true" />
			</a>
			<PushToggle />
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
			<button
				class="menu-toggle"
				type="button"
				aria-label="Меню"
				aria-expanded={menuOpen}
				aria-controls="site-menu"
				onclick={() => (menuOpen = !menuOpen)}
			>
				{#if menuOpen}
					<IconX aria-hidden="true" />
				{:else}
					<IconMenu aria-hidden="true" />
				{/if}
			</button>
		</div>
	</div>

	<div id="site-menu" class="site-menu" class:open={menuOpen}>
		<a class="menu-link" href={resolve("games")}>Игры</a>
		<a class="menu-link" href={resolve("movies")}>Фильмы</a>
		<a class="menu-link" href={resolve("account")}>Аккаунт</a>
		<DonateToggle />
		<PushToggle label />
		<span class="menu-sha" title="Build commit">{GIT_SHA}</span>
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

	.theme-toggle,
	.menu-toggle {
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

	.theme-toggle:hover,
	.menu-toggle:hover {
		border-color: var(--primary);
		color: var(--primary);
	}

	.theme-toggle :global(svg),
	.menu-toggle :global(svg) {
		width: 1rem;
		height: 1rem;
		flex-shrink: 0;
	}

	.menu-toggle {
		display: none;
		padding: 0.45rem;
	}

	.nav-link {
		display: inline-flex;
		align-items: center;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container-low);
		color: var(--on-surface);
		border-radius: 999px;
		padding: 0.45rem 0.9rem;
		font-size: 0.8rem;
		font-weight: 600;
		text-decoration: none;
		transition:
			border-color 0.15s,
			color 0.15s;
	}

	.nav-link--icon {
		justify-content: center;
		width: 2.25rem;
		padding: 0;
	}

	.nav-link:hover {
		border-color: var(--primary);
		color: var(--primary);
	}

	.build-sha,
	.menu-sha {
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

	.site-menu {
		display: none;
	}

	@media (max-width: 640px) {
		.site-nav-inner {
			padding: 0 0.875rem;
			gap: 0.5rem;
		}

		.nav-actions {
			gap: 0.375rem;
		}

		.build-sha,
		.nav-link,
		.nav-actions :global(.donate-link) {
			display: none;
		}

		.menu-toggle {
			display: inline-flex;
		}

		.site-menu {
			position: absolute;
			top: calc(100% + 0.5rem);
			left: 0.75rem;
			right: 0.75rem;
			flex-direction: column;
			align-items: stretch;
			gap: 0.4rem;
			padding: 0.6rem;
			background: var(--surface-bright);
			border: 1px solid var(--outline-variant);
			border-radius: 0.85rem;
			box-shadow: 0 16px 40px
				color-mix(in oklch, var(--video-bg) 40%, transparent);
		}

		.site-menu.open {
			display: flex;
		}

		.menu-link {
			display: flex;
			align-items: center;
			min-height: 2.75rem;
			padding: 0 0.9rem;
			border-radius: 0.5rem;
			color: var(--on-surface);
			text-decoration: none;
			font-size: 0.9rem;
			font-weight: 600;
		}

		.menu-link:hover {
			background: var(--surface-container-low);
			color: var(--primary);
		}

		.site-menu :global(.donate-link) {
			width: 100%;
			min-height: 2.75rem;
			justify-content: center;
		}

		.menu-sha {
			padding: 0.25rem 0.9rem 0;
		}
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
