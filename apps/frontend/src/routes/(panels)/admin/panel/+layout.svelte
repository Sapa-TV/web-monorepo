<script lang="ts">
	import { panelState } from "#lib/admin/panel-state.svelte";
	import { guardAdmin, GuardStatus } from "#lib/admin/session";
	import PanelHeader from "#lib/components/admin/PanelHeader.svelte";
	import SidebarMenu from "#lib/components/admin/SidebarMenu.svelte";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { Alert } from "@sapa-tv-ru/ui-kit";
	import { onMount, type Snippet } from "svelte";

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();

	let error = $state("");

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	onMount(async () => {
		try {
			const guard = await guardAdmin();
			if (guard.status === GuardStatus.NotLoggedIn) {
				await goto(resolve("admin/login"), { replaceState: true });
				return;
			}
			if (guard.status === GuardStatus.NotAdmin) {
				await goto(resolve(""), { replaceState: true });
				return;
			}
			panelState.isRoot = guard.isRoot;
			panelState.loaded = true;
		} catch (err) {
			setError(err);
		}
	});
</script>

<PanelHeader />

{#if error}
	<Alert tone="error">{error}</Alert>
{/if}

{#if panelState.loaded}
	<div class="panel-body">
		<SidebarMenu />
		<main class="panel-content">
			{@render children()}
		</main>
	</div>
{:else if !error}
	<p class="loading">Проверка доступа...</p>
{/if}

<style>
	.panel-body {
		display: grid;
		grid-template-columns: 220px minmax(0, 1fr);
		gap: 16px;
		align-items: start;
	}

	@media (max-width: 720px) {
		.panel-body {
			grid-template-columns: 1fr;
		}
	}
</style>
