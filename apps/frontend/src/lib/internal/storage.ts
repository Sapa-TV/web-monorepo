/**
 * localStorage throws in private-mode browsers; normalize here so callers
 * stay inside the Result world.
 */
export function setLocalStorage(key: string, value: string): boolean {
	try {
		localStorage.setItem(key, value);
		return true;
	} catch {
		return false;
	}
}

export function getLocalStorage(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		return null;
	}
}
