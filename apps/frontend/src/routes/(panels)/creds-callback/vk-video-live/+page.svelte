<script lang="ts">
	import CredsCallbackPage from "#lib/components/auth/CredsCallbackPage.svelte";
	import { CredsPlatform } from "#lib/admin/creds";
</script>

<CredsCallbackPage platform={CredsPlatform.VkVideoLive} title="VK Видео Live" />
