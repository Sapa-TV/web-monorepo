<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/state";
	import DonateToggle from "./DonateToggle.svelte";
	import IconBook from "~icons/lucide/book";
	import IconSun from "~icons/lucide/sun";
	import IconMoon from "~icons/lucide/moon";

	let isStream = $derived(page.url.pathname === "/stream");

	let dark = $state<"light" | "dark">(
		typeof document !== "undefined"
			? document.documentElement.dataset.theme === "dark"
				? "dark"
				: "light"
			: "light",
	);

	onMount(() => {
		const stored = localStorage.getItem("theme");
		const prefersDark = window.matchMedia(
			"(prefers-color-scheme: dark)",
		).matches;
		dark = stored
			? stored === "dark"
				? "dark"
				: "light"
			: prefersDark
				? "dark"
				: "light";
	});

	$effect(() => {
		if (typeof document === "undefined") return;
		document.documentElement.dataset.theme = dark === "dark" ? "dark" : "light";
		try {
			localStorage.setItem("theme", dark === "dark" ? "dark" : "light");
		} catch {
			/* приватный режим — игнорируем */
		}
	});
</script>

<nav class="site-nav" aria-label="Навигация по сайту">
	<div class="site-nav-inner">
		<a class="brand" href="/">
			<span class="brand-badge" aria-hidden="true">ST</span>
			<span>Sapa TV</span>
			{#if isStream}
				<span class="badge-live"><span class="live-dot"></span>LIVE</span>
			{/if}
		</a>

		<div class="nav-actions">
			<a class="nav-link" href="/links">
				<IconBook aria-hidden="true" />
				Каталог
			</a>
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
	.badge-live {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		background-color: var(--error, #e64040);
		color: var(--on-error, #ffffff);
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
