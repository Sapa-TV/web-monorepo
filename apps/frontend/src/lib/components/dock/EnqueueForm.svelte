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
	<form
		class="inline-form"
		onsubmit={(e) => {
			e.preventDefault();
			onsubmit();
		}}
	>
		<label class="visually-hidden" for="enq-name">Имя зрителя</label>
		<Input
			id="enq-name"
			type="text"
			placeholder="Имя зрителя"
			bind:value
			autofocus
		/>
		<Button variant="primary" type="submit" disabled={busy}>
			<IconPlus aria-hidden="true" />
			Добавить
		</Button>
	</form>
</Section>

<style>
	.inline-form :global(.field-input) {
		min-width: 140px;
	}
</style>
