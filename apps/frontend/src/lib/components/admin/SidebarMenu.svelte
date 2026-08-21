<script lang="ts">
	import { resolve } from "$app/paths";
	import { page } from "$app/state";
	import IconLoaderPinwheel from "~icons/lucide/loader-pinwheel";
	import IconPuzzle from "~icons/lucide/puzzle";
	import IconTv from "~icons/lucide/tv";
	import IconUsers from "~icons/lucide/users";
	import IconZap from "~icons/lucide/zap";
	import SidebarItem from "#lib/components/admin/SidebarItem.svelte";

	const panelBase = resolve("admin/panel");

	const menu = [
		{ label: "Платформы", icon: IconTv, path: panelBase },
		{ label: "Виджеты", icon: IconPuzzle, path: `${panelBase}/widgets` },
		{
			label: "Рулетка",
			icon: IconLoaderPinwheel,
			path: `${panelBase}/roulette`,
		},
		{
			label: "Действия и триггеры",
			icon: IconZap,
			path: `${panelBase}/actions`,
		},
		{ label: "Админы", icon: IconUsers, path: `${panelBase}/admins` },
	];
</script>

<nav class="sidebar-menu" aria-label="Разделы админ-панели">
	{#each menu as item (item.path)}
		<SidebarItem
			icon={item.icon}
			label={item.label}
			href={item.path}
			active={page.url.pathname === item.path}
		/>
	{/each}
</nav>

<style>
	.sidebar-menu {
		display: flex;
		flex-direction: column;
		gap: 4px;
		position: sticky;
		top: 24px;
	}

	@media (max-width: 720px) {
		.sidebar-menu {
			position: static;
			flex-direction: row;
			flex-wrap: wrap;
		}
	}
</style>
