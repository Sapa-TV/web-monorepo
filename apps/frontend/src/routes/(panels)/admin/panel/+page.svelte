<script lang="ts">
	import { api } from "#lib/api";
	import PlatformCredentialsCard from "#lib/components/admin/platforms/PlatformCredentialsCard.svelte";
	import IconTwitch from "~icons/lucide/twitch";
	import IconVideo from "~icons/lucide/video";

	async function startTwitchAuth() {
		return api.startTwitchAuth();
	}

	async function startVkAuth() {
		return api.startVkVideoLiveAuth();
	}
</script>

<svelte:head>
	<title>Sapa TV | Платформы</title>
</svelte:head>

<PlatformCredentialsCard
	platform="twitch"
	title="Twitch"
	description="Учётка, от имени которой бекенд ходит в Twitch (стрим-статус, чтение чата)."
	buttonVariant="twitch"
	startAuth={startTwitchAuth}
>
	{#snippet icon()}
		<IconTwitch aria-hidden="true" />
	{/snippet}
</PlatformCredentialsCard>

<PlatformCredentialsCard
	platform="vk_video_live"
	title="VK Видео Live"
	description="Учётка, от имени которой бекенд читает чат канала и пишет в него."
	startAuth={startVkAuth}
>
	{#snippet icon()}
		<IconVideo aria-hidden="true" />
	{/snippet}
</PlatformCredentialsCard>
