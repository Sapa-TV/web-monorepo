const scope = new URL(self.registration.scope);
const base = scope.pathname.replace(/\/$/, "");
const BASE64_GROUP_SIZE = 4;

self.addEventListener("install", () => {
	self.skipWaiting();
});

self.addEventListener("activate", (event) => {
	event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
	event.respondWith(fetch(event.request));
});

self.addEventListener("push", (event) => {
	let payload = {};
	if (event.data) {
		try {
			payload = event.data.json();
		} catch {
			payload = { body: event.data.text() };
		}
	}

	const title = payload.title || "Sapa TV";
	const url = payload.url || `${base}/`;
	event.waitUntil(
		self.registration.showNotification(title, {
			body: payload.body || "",
			tag: payload.tag || "sapa-tv",
			icon: `${base}/icons/icon-192.png`,
			badge: `${base}/icons/icon-192.png`,
			data: { url },
		}),
	);
});

function decodeBase64Url(value) {
	const remainder = value.length % BASE64_GROUP_SIZE;
	const padding = remainder === 0 ? "" : "=".repeat(BASE64_GROUP_SIZE - remainder);
	const padded = value.replace(/-/g, "+").replace(/_/g, "/") + padding;
	const binary = atob(padded);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i += 1) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes;
}

async function saveSubscription(subscription) {
	if (!subscription) return;
	const keys = subscription.toJSON().keys;
	if (!keys?.p256dh || !keys?.auth) return;
	await fetch(`${base}/api/push/subscriptions`, {
		method: "POST",
		credentials: "include",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({ endpoint: subscription.endpoint, keys }),
	});
}

async function resubscribe(oldSubscription) {
	const config = await fetch(`${base}/api/push/config`, {
		credentials: "include",
	}).then((response) => response.json());
	if (!config?.public_key) return;
	const applicationServerKey =
		oldSubscription?.options?.applicationServerKey ??
		decodeBase64Url(config.public_key);
	const subscription = await self.registration.pushManager.subscribe({
		userVisibleOnly: true,
		applicationServerKey,
	});
	await saveSubscription(subscription);
}

self.addEventListener("pushsubscriptionchange", (event) => {
	event.waitUntil(
		(async () => {
			if (event.newSubscription) {
				await saveSubscription(event.newSubscription);
				return;
			}
			try {
				await resubscribe(event.oldSubscription);
			} catch {
				return;
			}
		})(),
	);
});

self.addEventListener("notificationclick", (event) => {
	event.notification.close();
	const url = event.notification.data?.url || `${base}/`;

	event.waitUntil(
		self.clients
			.matchAll({ type: "window", includeUncontrolled: true })
			.then((clients) => {
				for (const client of clients) {
					if (client.url === url && "focus" in client) {
						return client.focus();
					}
				}
				return self.clients.openWindow(url);
			}),
	);
});
