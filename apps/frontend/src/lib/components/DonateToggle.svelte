<script lang="ts">
	import { onMount } from "svelte";

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
	<svg
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="1.8"
		stroke-linecap="round"
		stroke-linejoin="round"
		aria-hidden="true"
	>
		<path d="M12 2v20M2 12h20" />
	</svg>
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

	.donate-link svg {
		width: 0.95rem;
		height: 0.95rem;
	}

	.donate-link:hover {
		filter: brightness(1.08);
		background-color: var(--primary-dim, #d99a0f);
	}
</style>