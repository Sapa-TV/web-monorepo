<script lang="ts">
	import { api } from "#lib/api";
	import type {
		AdminResponse,
		TwitchUserResponse,
	} from "@sapa-tv-ru/api-client";
	import {
		Alert,
		Badge,
		Button,
		Card,
		Input,
		Section,
		TableWrap,
	} from "@sapa-tv-ru/ui-kit";
	import { onMount } from "svelte";
	import IconTrash2 from "~icons/lucide/trash-2";
	import IconUserPlus from "~icons/lucide/user-plus";
	import { describeApiError } from "#lib/api-error-text";
	import { ApiErrorKind, normalizeApiError } from "#lib/internal/api-error";

	const SEARCH_DEBOUNCE_MS = 400;

	let admins = $state<AdminResponse[]>([]);
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let newTwitchId = $state("");
	let newDisplayName = $state("");
	let addBusy = $state(false);
	let removeBusyId = $state<string | null>(null);

	let searchLogin = $state("");
	let searchBusy = $state(false);
	let foundUser = $state<TwitchUserResponse | null>(null);

	function setError(err: unknown) {
		error = describeApiError(err);
	}

	$effect(() => {
		const login = searchLogin.trim();
		foundUser = null;
		if (!login) return;
		const timer = setTimeout(() => void lookupUser(login), SEARCH_DEBOUNCE_MS);
		return () => clearTimeout(timer);
	});

	async function lookupUser(login: string) {
		searchBusy = true;
		const res = await api.findTwitchUser({ login });
		if (res.isErr()) {
			foundUser = null;
			const err = normalizeApiError(res.error);
			if (err.kind !== ApiErrorKind.NotFound) setError(err);
		} else {
			foundUser = res.value;
		}
		searchBusy = false;
	}

	function applyFoundUser() {
		if (!foundUser) return;
		newTwitchId = foundUser.id;
		newDisplayName = foundUser.display_name;
	}

	async function loadAdmins() {
		const res = await api.listAdmins();
		if (res.isErr()) {
			setError(res.error);
			return;
		}
		admins = res.value;
	}

	async function addAdmin() {
		const twitchId = newTwitchId.trim();
		if (!twitchId) return;
		addBusy = true;
		error = "";
		const res = await api.addAdmin({
			twitch_id: twitchId,
			display_name: newDisplayName.trim() || null,
		});
		if (res.isErr()) {
			setError(res.error);
		} else {
			newTwitchId = "";
			newDisplayName = "";
			await loadAdmins();
			hint = `Админ ${res.value.display_name ?? res.value.twitch_id} добавлен.`;
		}
		addBusy = false;
	}

	async function removeAdmin(twitchId: string) {
		removeBusyId = twitchId;
		error = "";
		const res = await api.removeAdmin(twitchId);
		if (res.isErr()) {
			setError(res.error);
		} else {
			await loadAdmins();
			hint = "Админ удалён.";
		}
		removeBusyId = null;
	}

	onMount(() => {
		void loadAdmins().finally(() => (loaded = true));
	});
</script>

<Card>
	<Section title="Администраторы">
		{#if error}
			<Alert tone="error">{error}</Alert>
		{/if}
		{#if hint}
			<Alert tone="success">{hint}</Alert>
		{/if}

		<form class="search-form" onsubmit={(e) => e.preventDefault()}>
			<Input
				type="text"
				placeholder="Поиск по Twitch username"
				bind:value={searchLogin}
			/>
			{#if searchBusy}
				<span class="search-hint">Поиск...</span>
			{/if}
		</form>

		{#if foundUser}
			<div class="found-row">
				<span class="found-info">
					{foundUser.display_name} · {foundUser.login} · #{foundUser.id}
				</span>
				<Button
					size="sm"
					variant="primary"
					type="button"
					onclick={applyFoundUser}
				>
					Подставить
				</Button>
			</div>
		{/if}

		<form
			class="inline-form"
			onsubmit={(e) => {
				e.preventDefault();
				void addAdmin();
			}}
		>
			<Input
				type="text"
				placeholder="Twitch ID"
				bind:value={newTwitchId}
				required
			/>
			<Input
				type="text"
				placeholder="Отображаемое имя (опц.)"
				bind:value={newDisplayName}
			/>
			<Button variant="primary" type="submit" disabled={addBusy}>
				<IconUserPlus aria-hidden="true" />
				{addBusy ? "Добавление..." : "Добавить"}
			</Button>
		</form>

		{#if loaded}
			<TableWrap>
				<table>
					<thead>
						<tr>
							<th>Имя</th>
							<th>Twitch ID</th>
							<th></th>
						</tr>
					</thead>
					<tbody>
						{#each admins as a (a.twitch_id)}
							<tr>
								<td>
									{a.display_name ?? "—"}
									{#if a.is_root}
										<Badge tone="root">root</Badge>
									{/if}
								</td>
								<td class="mono">{a.twitch_id}</td>
								<td class="actions-cell">
									{#if !a.is_root}
										<Button
											size="sm"
											variant="danger"
											type="button"
											onclick={() => removeAdmin(a.twitch_id)}
											disabled={removeBusyId === a.twitch_id}
											aria-label="Удалить админа"
										>
											<IconTrash2 aria-hidden="true" />
										</Button>
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</TableWrap>
		{:else}
			<p class="loading">Загрузка...</p>
		{/if}
	</Section>
</Card>

<style>
	.search-form {
		display: flex;
		gap: 8px;
		align-items: center;
		margin-bottom: 8px;
	}

	.search-form :global(.field-input) {
		max-width: 280px;
	}

	.search-hint {
		font-size: 12px;
		color: var(--on-surface-variant);
	}

	.found-row {
		display: flex;
		gap: 8px;
		align-items: center;
		justify-content: space-between;
		padding: 8px 10px;
		border: 1px solid var(--outline-variant);
		border-radius: 10px;
		background: var(--surface-container-low);
		margin-bottom: 8px;
	}

	.found-info {
		font-size: 13px;
		color: var(--on-surface);
	}
</style>
