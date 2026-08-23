/**
 * Clipboard is a browser API that fails via rejection; normalize it here so
 * callers stay inside the Result world.
 */
export async function copyText(text: string): Promise<boolean> {
	try {
		await navigator.clipboard.writeText(text);
		return true;
	} catch {
		return false;
	}
}
