import { asset } from "$app/paths";

export async function ensureServiceWorker(): Promise<ServiceWorkerRegistration | null> {
	if (!("serviceWorker" in navigator)) return null;

	const url = asset("sw.js");
	const scope = url.slice(0, url.lastIndexOf("/") + 1);
	const existing = await navigator.serviceWorker.getRegistration(scope);
	if (existing) return existing;

	return navigator.serviceWorker.register(url, { scope });
}
