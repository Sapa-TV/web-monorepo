<script lang="ts">
	import { onMount } from "svelte";
	import IconPlus from "~icons/lucide/plus";

	const DEST = "https://sapushka.oda.digital/";
	const LABELS = ["На поддержку сервера", "Торну на еду"];

	let index = $state(0);

	onMount(() => {
		const id = setInterval(() => {
			index = (index + 1) % LABELS.length;
		}, 10_000);
		return () => clearInterval(id);
	});
</script>

<a class="donate-link" href={DEST} target="_blank" rel="noopener">
	<IconPlus aria-hidden="true" />
	{LABELS[index]}
</a>

<style>
	.donate-link {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		width: 13rem;
		background-color: var(--primary, #ffb01e);
		color: var(--on-primary, #1a1202);
		text-decoration: none;
		padding: 0.4rem 0.85rem;
		border-radius: 6px;
		font-size: 0.78rem;
		font-weight: 600;
		white-space: nowrap;
		transition:
			background-color 0.15s,
			filter 0.15s;
	}

	.donate-link:hover {
		filter: brightness(1.08);
		background-color: var(--primary-dim, #d99a0f);
	}
</style>
