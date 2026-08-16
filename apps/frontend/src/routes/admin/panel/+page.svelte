<script lang="ts">
	import { guardAdmin, GuardStatus, logout } from "#lib/admin/session";
	import { api } from "#lib/api";
	import { goto } from "$app/navigation";
	import { HttpError, type AdminResponse } from "@sapa-tv-ru/api-client";
	import { onDestroy, onMount } from "svelte";
	import IconCheck from "~icons/lucide/check";
	import IconCopy from "~icons/lucide/copy";
	import IconLogOut from "~icons/lucide/log-out";
	import IconRefreshCw from "~icons/lucide/refresh-cw";
	import IconTrash2 from "~icons/lucide/trash-2";
	import IconTwitch from "~icons/lucide/twitch";
	import IconUserPlus from "~icons/lucide/user-plus";

	let isRoot = $state(false);
	let loaded = $state(false);
	let error = $state("");
	let actionMsg = $state("");

	let accessKey = $state("");
	let pakBusy = $state(false);
	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | null = null;
	const COPY_FEEDBACK_MS = 1500;

	let admins = $state<AdminResponse[]>([]);
	let newTwitchId = $state("");
	let newDisplayName = $state("");
	let addBusy = $state(false);
	let removeBusyId = $state<string | null>(null);

	let credsConfigured = $state(false);
	let authorizeBusy = $state(false);
	let credsPollTimer: ReturnType<typeof setInterval> | null = null;
	const CREDS_POLL_MS = 3000;
	const CREDS_POLL_MAX_TRIES = 100;
	const UNAUTHORIZED = 401;
	const FORBIDDEN = 403;

	function setError(err: unknown) {
		error = err instanceof Error ? err.message : String(err);
	}

	async function loadPak() {
		const res = await api.getAdminPak();
		if (res.isErr()) throw res.error;
		accessKey = res.value.pak;
	}

	async function rotatePak() {
		if (!confirm("Сгенерировать новый access_key? Старый перестанет работать."))
			return;
		pakBusy = true;
		error = "";
		try {
			const res = await api.rotateAdminPak();
			if (res.isErr()) throw res.error;
			accessKey = res.value.pak;
			actionMsg = "Access key обновлён.";
		} catch (err) {
			setError(err);
		} finally {
			pakBusy = false;
		}
	}

	async function copyPak() {
		try {
			await navigator.clipboard.writeText(accessKey);
			copied = true;
			if (copyTimer) clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = false), COPY_FEEDBACK_MS);
		} catch {
			// clipboard unavailable
		}
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
			actionMsg = `Админ ${res.value.display_name ?? res.value.twitch_id} добавлен.`;
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
			actionMsg = "Админ удалён.";
		} catch (err) {
			setError(err);
		} finally {
			removeBusyId = null;
		}
	}

	async function loadCreds() {
		const res = await api.getIngressCredentials();
		if (res.isErr()) throw res.error;
		credsConfigured = res.value.configured;
	}

	function clearCredsPoll() {
		if (credsPollTimer) {
			clearInterval(credsPollTimer);
			credsPollTimer = null;
		}
	}

	function startCredsPoll() {
		clearCredsPoll();
		let tries = 0;
		credsPollTimer = setInterval(async () => {
			tries += 1;
			try {
				await loadCreds();
				if (credsConfigured) {
					clearCredsPoll();
					actionMsg = "Twitch credentials авторизованы.";
				} else if (tries >= CREDS_POLL_MAX_TRIES) {
					clearCredsPoll();
					error =
						"Таймаут авторизации: подтверди доступ в окне Twitch и нажми «Авторизовать» ещё раз.";
				}
			} catch (err) {
				const http = err instanceof HttpError ? err : null;
				if (
					http &&
					(http.status === UNAUTHORIZED || http.status === FORBIDDEN)
				) {
					clearCredsPoll();
					error = "Сессия истекла: перелогинься и попробуй снова.";
				} else if (tries >= CREDS_POLL_MAX_TRIES) {
					clearCredsPoll();
					error = "Не удалось получить статус credentials. Попробуй ещё раз.";
				}
			}
		}, CREDS_POLL_MS);
	}

	async function authorizeTwitch() {
		authorizeBusy = true;
		error = "";
		const win = window.open(
			"",
			"sapa_twitch_auth",
			"popup,width=560,height=720",
		);
		try {
			const res = await api.startTwitchAuth();
			if (res.isErr()) throw res.error;
			if (win) {
				win.location.assign(res.value.auth_url);
				actionMsg = "Авторизуйся во всплывающем окне — статус обновится сам.";
				startCredsPoll();
			} else {
				actionMsg =
					"Всплывающее окно заблокировано: разреши попапы для этого сайта и нажми «Авторизовать» ещё раз.";
			}
		} catch (err) {
			win?.close();
			setError(err);
		} finally {
			authorizeBusy = false;
		}
	}

	async function revokeCreds() {
		if (
			!confirm(
				"Отозвать Twitch credentials? Ингейш-сервис перестанет работать.",
			)
		)
			return;
		error = "";
		try {
			const res = await api.revokeIngressCredentials();
			if (res.isErr()) throw res.error;
			await loadCreds();
			actionMsg = "Credentials отозваны.";
		} catch (err) {
			setError(err);
		}
	}

	async function handleLogout() {
		await logout();
		await goto("/admin/login", { replaceState: true });
	}

	onMount(async () => {
		try {
			const guard = await guardAdmin();
			if (guard.status === GuardStatus.NotLoggedIn) {
				await goto("/admin/login", { replaceState: true });
				return;
			}
			if (guard.status === GuardStatus.NotAdmin) {
				await goto("/", { replaceState: true });
				return;
			}
			isRoot = guard.isRoot;
			loaded = true;
			await Promise.all([loadPak(), loadAdmins(), loadCreds()]);
		} catch (err) {
			setError(err);
		}
	});

	onDestroy(clearCredsPoll);
</script>

<svelte:head>
	<title>Sapa TV | Админ-панель</title>
</svelte:head>

{#if loaded}
	<header class="panel-header">
		<h1>Админ-панель</h1>
		<div class="panel-header__right">
			{#if isRoot}
				<span class="badge badge--root">root</span>
			{/if}
			<button class="btn btn--sm" type="button" onclick={handleLogout}>
				<IconLogOut aria-hidden="true" />
				Выйти
			</button>
		</div>
	</header>

	{#if error}
		<p class="alert alert--error" role="alert">{error}</p>
	{/if}
	{#if actionMsg}
		<p class="alert alert--ok">{actionMsg}</p>
	{/if}

	<section class="card">
		<div class="section-title">Access key</div>
		<p class="section-hint">Ключ для доступа к панелям/виджетам с стримера.</p>
		<div class="key-row">
			<code class="key-value" title={accessKey}>{accessKey}</code>
			<button
				class="btn btn--sm"
				type="button"
				onclick={copyPak}
				disabled={!accessKey}
				aria-label="Скопировать ключ"
			>
				{#if copied}
					<IconCheck aria-hidden="true" />
				{:else}
					<IconCopy aria-hidden="true" />
				{/if}
			</button>
		</div>
		<button
			class="btn btn--primary"
			type="button"
			onclick={rotatePak}
			disabled={pakBusy}
		>
			<IconRefreshCw aria-hidden="true" />
			{pakBusy ? "Генерация..." : "Сгенерировать новый"}
		</button>
	</section>

	{#if isRoot}
		<section class="card">
			<div class="section-title">Администраторы</div>
			<form
				class="inline-form"
				onsubmit={(e) => {
					e.preventDefault();
					void addAdmin();
				}}
			>
				<input
					type="text"
					placeholder="Twitch ID"
					bind:value={newTwitchId}
					required
				/>
				<input
					type="text"
					placeholder="Отображаемое имя (опц.)"
					bind:value={newDisplayName}
				/>
				<button class="btn btn--primary" type="submit" disabled={addBusy}>
					<IconUserPlus aria-hidden="true" />
					{addBusy ? "Добавление..." : "Добавить"}
				</button>
			</form>

			<div class="table-wrap">
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
										<span class="badge badge--root">root</span>
									{/if}
								</td>
								<td class="mono">{a.twitch_id}</td>
								<td class="actions-cell">
									<button
										class="btn btn--sm btn--danger"
										type="button"
										onclick={() => removeAdmin(a.twitch_id)}
										disabled={removeBusyId === a.twitch_id}
										aria-label="Удалить админа"
									>
										<IconTrash2 aria-hidden="true" />
									</button>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>

		<section class="card">
			<div class="section-title">Twitch credentials</div>
			<p class="section-hint">
				Учётка, от имени которой бекенд ходит в Twitch (ингейш, стрим-статус).
			</p>
			<div class="creds-row">
				<span class={`status-pill ${credsConfigured ? "ok" : "missing"}`}>
					{credsConfigured ? "авторизовано" : "не авторизовано"}
				</span>
				<button
					class="btn btn--primary"
					type="button"
					onclick={authorizeTwitch}
					disabled={authorizeBusy}
				>
					<IconTwitch aria-hidden="true" />
					{authorizeBusy ? "Открытие..." : "Авторизовать"}
				</button>
				{#if credsConfigured}
					<button class="btn btn--sm" type="button" onclick={revokeCreds}>
						Отозвать
					</button>
				{/if}
			</div>
		</section>
	{/if}
{:else}
	<p class="loading">Проверка доступа...</p>
{/if}

<style>
	:global(html) {
		background: var(--background);
	}

	:global(body) {
		margin: 0;
		padding: 24px;
		font-family: "Inter", system-ui, sans-serif;
		background: var(--background);
		color: var(--on-surface);
		font-size: 14px;
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 16px;
	}

	h1 {
		margin: 0;
		font-size: 20px;
		color: var(--on-background);
	}

	.panel-header__right {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.card {
		background: var(--surface-container);
		border: 1px solid var(--outline-variant);
		border-radius: 12px;
		padding: 18px;
		margin-bottom: 20px;
		max-width: 720px;
	}

	.section-title {
		font-size: 12px;
		font-weight: 600;
		color: var(--on-surface-variant);
		margin-bottom: 10px;
		text-transform: uppercase;
		letter-spacing: 0.08em;
	}

	.section-hint {
		margin: 0 0 12px;
		color: var(--on-surface-variant);
		font-size: 12px;
		line-height: 1.5;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		padding: 8px 14px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface-container);
		font-size: 13px;
		font-weight: 600;
		font-family: inherit;
		color: var(--on-surface);
		cursor: pointer;
		transition:
			background 0.15s,
			border-color 0.15s,
			filter 0.15s;
	}

	.btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.btn--primary {
		background: var(--primary);
		border-color: transparent;
		color: var(--on-primary);
	}

	.btn--primary:hover:not(:disabled) {
		background: var(--primary-dim);
	}

	.btn--sm {
		padding: 6px 12px;
		border-color: var(--outline-variant);
		color: var(--on-surface);
	}

	.btn--sm:hover:not(:disabled) {
		border-color: var(--outline);
		background: var(--surface-container-high);
	}

	.btn--danger:hover:not(:disabled) {
		border-color: var(--error);
		color: var(--error);
	}

	.alert {
		max-width: 720px;
		margin: 0 0 16px;
		padding: 10px 14px;
		border-radius: 10px;
		font-size: 12px;
		line-height: 1.4;
	}

	.alert--error {
		background: color-mix(in oklch, var(--error) 12%, transparent);
		color: var(--error);
	}

	.alert--ok {
		background: color-mix(in oklch, var(--secondary) 14%, transparent);
		color: var(--secondary);
	}

	.badge {
		display: inline-block;
		padding: 2px 8px;
		border-radius: 6px;
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-left: 6px;
		vertical-align: middle;
	}

	.badge--root {
		background: color-mix(in oklch, var(--primary) 16%, transparent);
		color: var(--primary);
	}

	.key-row {
		display: flex;
		gap: 8px;
		align-items: center;
		margin-bottom: 12px;
	}

	.key-value {
		flex: 1;
		overflow-x: auto;
		white-space: nowrap;
		padding: 10px 12px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface);
		font-family: "IBM Plex Mono", ui-monospace, monospace;
		font-size: 12px;
		color: var(--on-surface);
	}

	.inline-form {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		align-items: center;
		margin-bottom: 14px;
	}

	.inline-form input {
		padding: 8px 12px;
		border-radius: 10px;
		border: 1px solid var(--outline-variant);
		background: var(--surface);
		color: var(--on-surface);
		font-size: 13px;
		font-family: inherit;
		outline: none;
		min-width: 160px;
	}

	.inline-form input:focus {
		border-color: var(--primary);
	}

	.table-wrap {
		overflow: hidden;
		border-radius: 12px;
		border: 1px solid var(--outline-variant);
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th {
		text-align: left;
		padding: 10px 14px;
		font-size: 11px;
		color: var(--on-surface-variant);
		text-transform: uppercase;
		letter-spacing: 0.08em;
		border-bottom: 1px solid var(--outline-variant);
		background: var(--surface-container-high);
	}

	td {
		padding: 10px 14px;
		border-bottom: 1px solid var(--outline-variant);
	}

	tr:last-child td {
		border-bottom: none;
	}

	.mono {
		font-family: "IBM Plex Mono", ui-monospace, monospace;
		font-size: 12px;
	}

	.actions-cell {
		width: 1%;
		text-align: right;
	}

	.creds-row {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}

	.status-pill {
		padding: 3px 10px;
		border-radius: 6px;
		font-size: 11px;
		font-weight: 600;
	}

	.status-pill.ok {
		background: color-mix(in oklch, var(--secondary) 16%, transparent);
		color: var(--secondary);
	}

	.status-pill.missing {
		background: color-mix(in oklch, var(--tertiary) 16%, transparent);
		color: var(--tertiary);
	}

	.loading {
		color: var(--on-surface-variant);
		font-size: 13px;
	}
</style>
