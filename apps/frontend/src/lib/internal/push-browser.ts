const BASE64_BLOCK = 4;

export function decodeApplicationKey(
	value: string,
): Uint8Array<ArrayBuffer> | null {
	try {
		const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
		const padding = "=".repeat(
			(BASE64_BLOCK - (normalized.length % BASE64_BLOCK)) % BASE64_BLOCK,
		);
		const raw = atob(normalized + padding);
		return Uint8Array.from(raw, (char) => char.charCodeAt(0));
	} catch {
		return null;
	}
}

export async function subscribePushManager(
	registration: ServiceWorkerRegistration,
	applicationServerKey: Uint8Array<ArrayBuffer>,
): Promise<PushSubscription | null> {
	try {
		return await registration.pushManager.subscribe({
			userVisibleOnly: true,
			applicationServerKey,
		});
	} catch {
		return null;
	}
}
