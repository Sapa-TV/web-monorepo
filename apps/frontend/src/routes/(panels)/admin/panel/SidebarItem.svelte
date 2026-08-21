<script lang="ts">
	import type { Component } from "svelte";

	interface Props {
		icon: Component;
		label: string;
		href: string;
		active: boolean;
	}

	let { icon: Icon, label, href, active }: Props = $props();
</script>

<a
	class="sidebar-item"
	class:active
	{href}
	aria-current={active ? "page" : undefined}
>
	<Icon aria-hidden="true" />
	{label}
</a>

<style>
	.sidebar-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		border-radius: 10px;
		border: 1px solid transparent;
		color: var(--on-surface-variant);
		text-decoration: none;
		font-size: 14px;
		font-weight: 600;
	}

	.sidebar-item:hover {
		background: var(--surface-container);
		color: var(--on-surface);
	}

	.sidebar-item.active {
		background: var(--primary-container);
		border-color: var(--outline-variant);
		color: var(--on-primary-container);
	}
</style>
