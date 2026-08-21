<script lang="ts">
	import { resolve } from "$app/paths";
	import { Button, Card, Section } from "@sapa-tv-ru/ui-kit";
	import { onDestroy } from "svelte";
	import IconCheck from "~icons/lucide/check";
	import IconCopy from "~icons/lucide/copy";

	interface Props {
		accessKey: string;
	}

	let { accessKey }: Props = $props();

	type LinkTarget = "dock" | "widget";

	let copied = $state<LinkTarget | null>(null);

	let copyTimer: ReturnType<typeof setTimeout> | null = null;
	const COPY_FEEDBACK_MS = 1500;

	function linkFor(path: string) {
		return `${location.origin}${resolve("").replace(/\/$/, "")}${path}?widget_access_key=${encodeURIComponent(accessKey)}`;
	}

	async function copyLink(target: LinkTarget, path: string) {
		try {
			await navigator.clipboard.writeText(linkFor(path));
			copied = target;
			if (copyTimer) clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = null), COPY_FEEDBACK_MS);
		} catch {
			// clipboard unavailable
		}
	}

	onDestroy(() => {
		if (copyTimer) clearTimeout(copyTimer);
	});
</script>

<Card>
	<Section title="Ссылки">
		<p class="section-hint">
			Ссылки с подставленным access key — открывать со стримера.
		</p>

		<div class="links-row">
			<Button
				size="sm"
				type="button"
				onclick={() => copyLink("dock", "/dock")}
				disabled={!accessKey}
				aria-label="Скопировать ссылку на док-панель"
			>
				{#if copied === "dock"}
					<IconCheck aria-hidden="true" />
				{:else}
					<IconCopy aria-hidden="true" />
				{/if}
				Док-панель
			</Button>
			<Button
				size="sm"
				type="button"
				onclick={() => copyLink("widget", "/roulette")}
				disabled={!accessKey}
				aria-label="Скопировать ссылку на виджет"
			>
				{#if copied === "widget"}
					<IconCheck aria-hidden="true" />
				{:else}
					<IconCopy aria-hidden="true" />
				{/if}
				Виджет
			</Button>
		</div>
	</Section>
</Card>

<style>
	.links-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}
</style>
