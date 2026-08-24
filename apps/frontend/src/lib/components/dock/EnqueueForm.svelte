<script lang="ts">
	import { Button, Input, Section } from "@sapa-tv-ru/ui-kit";
	import IconPlus from "~icons/lucide/plus";

	interface Props {
		value?: string;
		busy?: boolean;
		onsubmit: () => void;
	}

	let { value = $bindable(""), busy = false, onsubmit }: Props = $props();
</script>

<Section title="Добавить в очередь">
	<div class="inline-form">
		<label class="visually-hidden" for="enq-name">Имя зрителя</label>
		<Input id="enq-name" type="text" placeholder="Имя зрителя" bind:value />
		<Button variant="primary" type="button" onclick={onsubmit} disabled={busy}>
			<IconPlus aria-hidden="true" />
			Добавить
		</Button>
	</div>
</Section>

<style>
	.inline-form :global(.field-input) {
		min-width: 140px;
	}
</style>
