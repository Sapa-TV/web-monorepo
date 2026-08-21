<script lang="ts">
	import { page } from "$app/state";
	import { resolve } from "$app/paths";
	import IconMoon from "~icons/lucide/moon";
	import IconSun from "~icons/lucide/sun";
	import DonateToggle from "./DonateToggle.svelte";
	import { GIT_SHA } from "#lib/build-info";

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
	.build-sha {
		font-family: ui-monospace, monospace;
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
