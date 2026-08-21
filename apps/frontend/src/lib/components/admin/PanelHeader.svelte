<script lang="ts">
	import { panelState } from "#lib/admin/panel-state.svelte";
	import { logout } from "#lib/admin/session";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { Badge, Button } from "@sapa-tv-ru/ui-kit";
	import IconLogOut from "~icons/lucide/log-out";

	async function handleLogout() {
		await logout();
		await goto(resolve("admin/login"), { replaceState: true });
	}
</script>

<header class="panel-header">
	<h1>Админ-панель</h1>
	<div class="panel-header__right">
		{#if panelState.isRoot}
			<Badge tone="root">root</Badge>
		{/if}
		<Button size="sm" onclick={handleLogout}>
			<IconLogOut aria-hidden="true" />
			Выйти
		</Button>
	</div>
</header>

<style>
	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 16px;
	}

	h1 {
		margin: 0;
		font-size: 20px;
		color: var(--on-background);
	}

	.panel-header__right {
		display: flex;
		align-items: center;
		gap: 8px;
	}
</style>
