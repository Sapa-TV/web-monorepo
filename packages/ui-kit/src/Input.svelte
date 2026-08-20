<script lang="ts">
	interface Props {
		value?: string;
		placeholder?: string;
		type?: string;
		id?: string;
		name?: string;
		required?: boolean;
		minlength?: number;
		title?: string;
	}

	let {
		value = $bindable(""),
		placeholder,
		type,
		id,
		name,
		required,
		minlength,
		title,
		...rest
	}: Props = $props();
</script>

<input
	{...rest}
	{placeholder}
	{type}
	{id}
	{name}
	{required}
	{minlength}
	{title}
	bind:value
	class="field-input"
/>

<style>
	.field-input {
		padding: 8px 12px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container);
		color: var(--on-surface);
		font: inherit;
		font-size: 13px;
		outline: none;
	}

	.field-input:focus {
		border-color: var(--primary);
	}
</style>
