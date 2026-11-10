<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "@sapa-tv-ru/ui-kit";
	import type {
		LinkedPlatformResponse,
		SessionResponse,
	} from "@sapa-tv-ru/api-client";
	import IconTwitch from "~icons/lucide/twitch";
	import { logout } from "#lib/admin/session";
	import {
		getCurrentAccount,
		getLinkedPlatforms,
		PROVIDERS,
		providerLabel,
		startProvider,
		type Provider,
	} from "#lib/auth/provider";
	let session = $state<SessionResponse | null>(null);
	let links = $state<LinkedPlatformResponse[]>([]);
	let loading = $state(true);
	let busy = $state(false);
	let error = $state("");

	function linked(provider: Provider): LinkedPlatformResponse | undefined {
		return links.find((link) => link.platform === provider);
	}

	async function load() {
		const account = await getCurrentAccount();
		if (account.isErr()) {
			error = "Не удалось загрузить аккаунт.";
			loading = false;
			return;
		}
		session = account.value;
		if (!session) {
			loading = false;
			return;
		}
		const res = await getLinkedPlatforms();
		if (res.isOk()) links = res.value;
		else error = "Не удалось загрузить привязки.";
		loading = false;
	}

	async function signIn(provider: Provider) {
		busy = true;
		error = "";
		sessionStorage.setItem("auth_return", "account");
		const res = await startProvider(provider);
		if (res.isErr()) {
			sessionStorage.removeItem("auth_return");
			error = "Не удалось начать вход. Попробуйте ещё раз.";
			busy = false;
			return;
		}
		location.assign(res.value);
	}

	async function signOut() {
		busy = true;
		error = "";
		await logout();
		session = null;
		links = [];
		busy = false;
	}

	onMount(load);
</script>

<svelte:head>
	<title>Sapa TV | Аккаунт</title>
	<meta
		name="description"
		content="Вход и привязка Twitch и VK к аккаунту Sapa TV"
	/>
</svelte:head>

<main class="account">
	<header class="account__header">
		<h1>Аккаунт</h1>
		<p>Войдите, чтобы сохранить заявки и уведомления на всех устройствах.</p>
	</header>

	{#if loading}
		<p class="account__muted">Загрузка...</p>
	{:else}
		{#if error}
			<p class="account__error">{error}</p>
		{/if}

		<section class="account__card">
			<h2>{session && !session.is_guest ? "Вы вошли" : "Гостевой профиль"}</h2>
			{#if session}
				<p class="account__muted">
					{session.is_guest
						? "Профиль существует без входа. Привяжите Twitch или VK — заявки и подписки переедут в аккаунт."
						: `Профиль #${session.user_id}`}
				</p>
			{:else}
				<p class="account__muted">
					Сейчас вы не вошли. Можно смотреть без аккаунта, но вход сохранит
					подписку на уведомления.
				</p>
			{/if}

			<div class="account__providers">
				{#each PROVIDERS as provider (provider)}
					{@const link = linked(provider)}
					<div class="account__provider">
						<div class="account__identity">
							{#if provider === "twitch"}
								<IconTwitch aria-hidden="true" />
							{:else}
								<span class="account__vk">VK</span>
							{/if}
							<div>
								<strong>{providerLabel(provider)}</strong>
								<p class="account__muted">
									{link
										? (link.platform_username ?? link.platform_user_id)
										: "Не привязан"}
								</p>
							</div>
						</div>
						{#if !link}
							<Button
								variant="default"
								onclick={() => signIn(provider)}
								disabled={busy}
							>
								{session && !session.is_guest
									? `Привязать ${providerLabel(provider)}`
									: `Войти через ${providerLabel(provider)}`}
							</Button>
						{:else}
							<span class="account__badge">Привязан</span>
						{/if}
					</div>
				{/each}
			</div>

			{#if session && !session.is_guest}
				<Button variant="cancel" onclick={signOut} disabled={busy}>Выйти</Button
				>
			{/if}
		</section>
	{/if}
</main>

<style>
	.account {
		max-width: 720px;
		margin: 0 auto;
		padding: 32px 16px 64px;
	}

	.account__header h1 {
		margin: 0 0 8px;
		font-size: 28px;
		color: var(--on-surface);
	}

	.account__header p {
		margin: 0 0 24px;
		color: var(--on-surface-variant);
	}

	.account__card {
		display: flex;
		flex-direction: column;
		gap: 16px;
		padding: 24px;
		border: 1px solid var(--outline-variant);
		border-radius: 16px;
		background: var(--surface);
	}

	.account__card h2 {
		margin: 0;
		font-size: 18px;
		color: var(--on-surface);
	}

	.account__providers {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.account__provider {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 12px 0;
		border-top: 1px solid var(--outline-variant);
	}

	.account__identity {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.account__identity strong {
		color: var(--on-surface);
	}

	.account__identity p {
		margin: 2px 0 0;
	}

	.account__vk {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border-radius: 5px;
		background: var(--vk-brand);
		color: var(--vk-ink);
		font-size: 11px;
		font-weight: 800;
	}

	.account__badge {
		font-size: 12px;
		color: var(--on-surface-variant);
	}

	.account__muted {
		margin: 0;
		font-size: 13px;
		color: var(--on-surface-variant);
	}

	.account__error {
		margin: 0 0 16px;
		padding: 10px 14px;
		border-radius: 10px;
		background: color-mix(in srgb, var(--error) 12%, transparent);
		color: var(--error);
		font-size: 13px;
	}

	@media (max-width: 520px) {
		.account__provider {
			align-items: flex-start;
			flex-direction: column;
		}
	}
</style>
