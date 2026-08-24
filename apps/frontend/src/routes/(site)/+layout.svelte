<script lang="ts">
	import type { Snippet } from "svelte";
	import SiteNav from "#lib/components/SiteNav.svelte";
	import "../../styles/site.css";

	interface Props {
		children: Snippet;
	}

	let { children }: Props = $props();
</script>

<div class="paper">
	<SiteNav />
	{@render children()}
</div>
