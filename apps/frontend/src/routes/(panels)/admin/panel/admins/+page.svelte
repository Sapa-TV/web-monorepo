<script lang="ts">
	import { panelState } from "#lib/admin/panel-state.svelte";
	import AdminsCard from "#lib/components/admin/admins/AdminsCard.svelte";
	import { Alert } from "@sapa-tv-ru/ui-kit";
</script>

<svelte:head>
	<title>Sapa TV | Админы</title>
</svelte:head>

{#if panelState.isRoot}
	<AdminsCard />
{:else}
	<Alert tone="error">Раздел доступен только root-админам.</Alert>
{/if}
