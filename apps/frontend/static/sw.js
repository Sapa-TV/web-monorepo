const scope = new URL(self.registration.scope);
const base = scope.pathname.replace(/\/$/, "");

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
