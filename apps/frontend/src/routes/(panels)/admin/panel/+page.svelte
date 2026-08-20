<script lang="ts">
	import { guardAdmin, GuardStatus, logout } from "#lib/admin/session";
	import { api } from "#lib/api";
	import { goto } from "$app/navigation";
	import { resolve } from "$app/paths";
	import { HttpError, type AdminResponse } from "@sapa-tv-ru/api-client";
	import {
		Alert,
		Badge,
		Button,
		Card,
		Code,
		Input,
		Section,
		TableWrap,
	} from "@sapa-tv-ru/ui-kit";
	import { onDestroy, onMount } from "svelte";
	import IconCheck from "~icons/lucide/check";
	import IconCopy from "~icons/lucide/copy";
	import IconLogOut from "~icons/lucide/log-out";
	import IconRefreshCw from "~icons/lucide/refresh-cw";
	import IconTrash2 from "~icons/lucide/trash-2";
	import IconTwitch from "~icons/lucide/twitch";
	import IconUserPlus from "~icons/lucide/user-plus";
	import ActionsSection from "./ActionsSection.svelte";
	import RulesSection from "./RulesSection.svelte";

	let isRoot = $state(false);
	let loaded = $state(false);
	let error = $state("");
	let actionMsg = $state("");

	let accessKey = $state("");
	let wakBusy = $state(false);
	let copied = $state<"key" | "dock" | "widget" | null>(null);
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
		const res = await api.getWidgetAccessKey();
		if (res.isErr()) throw res.error;
		accessKey = res.value.widget_access_key;
	}

	async function rotatePak() {
		if (!confirm("Сгенерировать новый access_key? Старый перестанет работать."))
			return;
		wakBusy = true;
		error = "";
		try {
			const res = await api.rotateWidgetAccessKey();
			if (res.isErr()) throw res.error;
			accessKey = res.value.widget_access_key;
			actionMsg = "Access key обновлён.";
		} catch (err) {
			setError(err);
		} finally {
			wakBusy = false;
		}
	}

	async function copyPak() {
		try {
			await navigator.clipboard.writeText(accessKey);
			copied = "key";
			if (copyTimer) clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = null), COPY_FEEDBACK_MS);
		} catch {
			// clipboard unavailable
		}
	}

	function linkFor(path: string) {
		return `${location.origin}${resolve("").replace(/\/$/, "")}${path}?widget_access_key=${encodeURIComponent(accessKey)}`;
	}

	async function copyLink(target: "dock" | "widget", path: string) {
		try {
			await navigator.clipboard.writeText(linkFor(path));
			copied = target;
			if (copyTimer) clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = null), COPY_FEEDBACK_MS);
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
		await goto(resolve("admin/login"), { replaceState: true });
	}

	onMount(async () => {
		try {
			const guard = await guardAdmin();
			if (guard.status === GuardStatus.NotLoggedIn) {
				await goto(resolve("admin/login"), { replaceState: true });
				return;
			}
			if (guard.status === GuardStatus.NotAdmin) {
				await goto(resolve(""), { replaceState: true });
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
				<Badge tone="root">root</Badge>
			{/if}
			<Button size="sm" onclick={handleLogout}>
				<IconLogOut aria-hidden="true" />
				Выйти
			</Button>
		</div>
	</header>

	{#if error}
		<Alert tone="error">{error}</Alert>
	{/if}
	{#if actionMsg}
		<Alert tone="success">{actionMsg}</Alert>
	{/if}

	<Card>
		<Section title="Access key">
			<p class="section-hint">
				Ключ для доступа к панелям/виджетам с стримера.
			</p>
			<div class="key-row">
				<Code block title={accessKey}>{accessKey}</Code>
				<Button
					size="sm"
					type="button"
					onclick={copyPak}
					disabled={!accessKey}
					aria-label="Скопировать ключ"
				>
					{#if copied === "key"}
						<IconCheck aria-hidden="true" />
					{:else}
						<IconCopy aria-hidden="true" />
					{/if}
				</Button>
			</div>
			<div class="links-row">
				<Button
					size="sm"
					type="button"
					onclick={() => copyLink("dock", "/dock")}
					disabled={!accessKey}
					aria-label="Скопировать ссылку на док-панель"
				>
					{#if copied === "dock"}
						<IconCheck aria-hidden="true" />
					{:else}
						<IconCopy aria-hidden="true" />
					{/if}
					Док-панель
				</Button>
				<Button
					size="sm"
					type="button"
					onclick={() => copyLink("widget", "/roulette")}
					disabled={!accessKey}
					aria-label="Скопировать ссылку на виджет"
				>
					{#if copied === "widget"}
						<IconCheck aria-hidden="true" />
					{:else}
						<IconCopy aria-hidden="true" />
					{/if}
					Виджет
				</Button>
			</div>
			<Button
				variant="primary"
				type="button"
				onclick={rotatePak}
				disabled={wakBusy}
			>
				<IconRefreshCw aria-hidden="true" />
				{wakBusy ? "Генерация..." : "Сгенерировать новый"}
			</Button>
		</Section>
	</Card>

	{#if isRoot}
		<Card>
			<Section title="Администраторы">
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
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</TableWrap>
			</Section>
		</Card>

		<Card>
			<Section title="Twitch credentials">
				<p class="section-hint">
					Учётка, от имени которой бекенд ходит в Twitch (ингейш, стрим-статус).
				</p>
				<div class="creds-row">
					<Badge tone={credsConfigured ? "ok" : "missing"}>
						{credsConfigured ? "авторизовано" : "не авторизовано"}
					</Badge>
					<Button
						variant="primary"
						type="button"
						onclick={authorizeTwitch}
						disabled={authorizeBusy}
					>
						<IconTwitch aria-hidden="true" />
						{authorizeBusy ? "Открытие..." : "Авторизовать"}
					</Button>
					{#if credsConfigured}
						<Button size="sm" type="button" onclick={revokeCreds}>
							Отозвать
						</Button>
					{/if}
				</div>
			</Section>
		</Card>

		<ActionsSection />
		<RulesSection />
	{/if}
{:else}
	<p class="loading">Проверка доступа...</p>
{/if}

<style>
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

	.key-row {
		display: flex;
		gap: 8px;
		align-items: center;
		margin-bottom: 12px;
	}

	.links-row {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		margin-bottom: 12px;
	}

	.inline-form {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		align-items: center;
		margin-bottom: 14px;
	}

	.inline-form :global(.field-input) {
		min-width: 160px;
	}

	.creds-row {
		display: flex;
		align-items: center;
		gap: 10px;
		flex-wrap: wrap;
	}
</style>
