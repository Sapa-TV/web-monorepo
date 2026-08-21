<script lang="ts">
	import { api } from "#lib/api";
	import { Alert } from "@sapa-tv-ru/ui-kit";
	import AccessKeyCard from "#lib/components/admin/widgets/AccessKeyCard.svelte";
	import WidgetLinksCard from "#lib/components/admin/widgets/WidgetLinksCard.svelte";
	import { onMount } from "svelte";

	let accessKey = $state("");
	let loaded = $state(false);
	let error = $state("");

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	async function loadWak() {
		const res = await api.getWidgetAccessKey();
		if (res.isErr()) throw res.error;
		accessKey = res.value.widget_access_key;
	}

	onMount(() => {
		loadWak()
			.catch(setError)
			.finally(() => (loaded = true));
	});
</script>

<svelte:head>
	<title>Sapa TV | Виджеты</title>
</svelte:head>

{#if error}
	<Alert tone="error">{error}</Alert>
{/if}

{#if loaded}
	<AccessKeyCard {accessKey} onrotated={loadWak} />
	<WidgetLinksCard {accessKey} />
{/if}
