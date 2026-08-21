<script lang="ts">
	import { api } from "#lib/api";
	import type { AdminResponse } from "@sapa-tv-ru/api-client";
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

	let admins = $state<AdminResponse[]>([]);
	let loaded = $state(false);
	let error = $state("");
	let hint = $state("");

	let newTwitchId = $state("");
	let newDisplayName = $state("");
	let addBusy = $state(false);
	let removeBusyId = $state<string | null>(null);

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	async function loadAdmins() {
		const res = await api.listAdmins();
		if (res.isErr()) throw res.error;
		admins = res.value;
	}

	async function addAdmin() {
		const twitchId = newTwitchId.trim();
		if (!twitchId) return;
		addBusy = true;
		error = "";
		try {
			const res = await api.addAdmin({
				twitch_id: twitchId,
				display_name: newDisplayName.trim() || null,
			});
			if (res.isErr()) throw res.error;
			newTwitchId = "";
			newDisplayName = "";
			await loadAdmins();
			hint = `Админ ${res.value.display_name ?? res.value.twitch_id} добавлен.`;
		} catch (err) {
			setError(err);
		} finally {
			addBusy = false;
		}
	}

	async function removeAdmin(twitchId: string) {
		removeBusyId = twitchId;
		error = "";
		try {
			const res = await api.removeAdmin(twitchId);
			if (res.isErr()) throw res.error;
			await loadAdmins();
			hint = "Админ удалён.";
		} catch (err) {
			setError(err);
		} finally {
			removeBusyId = null;
		}
	}

	onMount(() => {
		loadAdmins()
			.catch(setError)
			.finally(() => (loaded = true));
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
