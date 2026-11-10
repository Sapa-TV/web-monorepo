import { api } from "#lib/api";
import {
	decodeApplicationKey,
	subscribePushManager,
} from "#lib/internal/push-browser";
import { ensureServiceWorker } from "#lib/pwa/register";
import { ensureSession } from "#lib/session/ensure";

export type PushStatus =
	| "loading"
	| "unsupported"
	| "disabled"
	| "denied"
	| "ready"
	| "subscribed"
	| "error";

interface PushState {
	status: PushStatus;
	message: string | null;
	iosHint: boolean;
}

export const pushState = $state<PushState>({
	status: "loading",
	message: null,
	iosHint: false,
});

let publicKey: string | null = null;
let initialization: Promise<void> | null = null;

function isSupported(): boolean {
	return (
		typeof window !== "undefined" &&
		"serviceWorker" in navigator &&
		"PushManager" in window &&
		"Notification" in window
	);
}

function isIosWithoutInstall(): boolean {
	if (typeof window === "undefined") return false;
	const ios = /iphone|ipad|ipod/i.test(navigator.userAgent);
	const standalone =
		window.matchMedia("(display-mode: standalone)").matches ||
		(navigator as Navigator & { standalone?: boolean }).standalone === true;
	return ios && !standalone;
}

function subscriptionKeys(subscription: PushSubscription) {
	const json = subscription.toJSON();
	return {
		endpoint: subscription.endpoint,
		keys: {
			p256dh: json.keys?.p256dh ?? "",
			auth: json.keys?.auth ?? "",
		},
	};
}

async function saveSubscription(
	subscription: PushSubscription,
): Promise<boolean> {
	if (!(await ensureSession())) {
		pushState.message = "Не удалось создать сессию";
		return false;
	}
	const result = await api.subscribe(subscriptionKeys(subscription));
	if (result.isErr()) {
		pushState.message = "Не удалось сохранить подписку";
		return false;
	}
	return true;
}

async function currentSubscription(
	registration: ServiceWorkerRegistration,
): Promise<PushSubscription | null> {
	return registration.pushManager.getSubscription();
}

export async function initPush(): Promise<void> {
	if (initialization) return initialization;

	initialization = (async () => {
		if (!isSupported()) {
			pushState.status = "unsupported";
			return;
		}
		pushState.iosHint = isIosWithoutInstall();

		const config = await api.getPushConfig();
		if (config.isErr() || !config.value.enabled || !config.value.public_key) {
			pushState.status = "disabled";
			return;
		}
		publicKey = config.value.public_key;

		if (Notification.permission === "denied") {
			pushState.status = "denied";
			return;
		}

		const registration = await ensureServiceWorker();
		if (!registration) {
			pushState.status = "unsupported";
			return;
		}

		const subscription = await currentSubscription(registration);
		if (!subscription) {
			pushState.status = "ready";
			return;
		}

		pushState.status = (await saveSubscription(subscription))
			? "subscribed"
			: "error";
	})().finally(() => {
		initialization = null;
	});

	return initialization;
}

export async function subscribePush(): Promise<void> {
	if (!publicKey) await initPush();
	if (
		!publicKey ||
		(pushState.status !== "ready" &&
			pushState.status !== "denied" &&
			pushState.status !== "error")
	) {
		return;
	}
	if (isIosWithoutInstall()) {
		pushState.iosHint = true;
		return;
	}

	pushState.message = null;
	const permission = await Notification.requestPermission();
	if (permission !== "granted") {
		pushState.status = permission === "denied" ? "denied" : "ready";
		return;
	}

	const registration = await ensureServiceWorker();
	if (!registration) {
		pushState.status = "unsupported";
		return;
	}

	const applicationServerKey = decodeApplicationKey(publicKey);
	if (!applicationServerKey) {
		pushState.status = "error";
		pushState.message = "Некорректный ключ уведомлений";
		return;
	}

	let subscription = await currentSubscription(registration);
	if (subscription) {
		await subscription.unsubscribe();
	}

	subscription = await subscribePushManager(registration, applicationServerKey);
	if (!subscription) {
		pushState.status = "error";
		pushState.message = "Браузер не дал подписаться на уведомления";
		return;
	}

	pushState.status = (await saveSubscription(subscription))
		? "subscribed"
		: "error";
}

export async function unsubscribePush(): Promise<void> {
	if (pushState.status !== "subscribed") return;

	const registration = await ensureServiceWorker();
	const subscription = registration
		? await currentSubscription(registration)
		: null;
	if (!subscription) {
		pushState.status = "ready";
		return;
	}

	const result = await api.unsubscribe({ endpoint: subscription.endpoint });
	if (result.isErr()) {
		pushState.message = "Не удалось отписаться, попробуй ещё раз";
		return;
	}

	await subscription.unsubscribe();
	pushState.status = "ready";
	pushState.message = null;
}

export async function togglePush(): Promise<void> {
	if (pushState.status === "subscribed") {
		await unsubscribePush();
		return;
	}
	await subscribePush();
}
