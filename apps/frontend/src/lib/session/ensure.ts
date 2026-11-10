import { api } from "#lib/api";

let pending: Promise<boolean> | null = null;

async function createSession(): Promise<boolean> {
	const current = await api.getMe();
	if (current.isOk()) return true;

	const guest = await api.createGuestSession();
	return guest.isOk();
}

export async function ensureSession(): Promise<boolean> {
	if (!pending) {
		pending = createSession().finally(() => {
			pending = null;
		});
	}
	return pending;
}
