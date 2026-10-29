import { describeApiError } from "#lib/api-error-text";
import type { Result } from "neverthrow";

export interface CrudAdapter<T, P> {
	list: () => PromiseLike<Result<T[], unknown>>;
	create: (_payload: P) => PromiseLike<Result<unknown, unknown>>;
	update: (_id: number, _payload: P) => PromiseLike<Result<unknown, unknown>>;
	remove: (_id: number) => PromiseLike<Result<unknown, unknown>>;
	hints: { created: string; updated: string; removed: string };
	confirmRemove?: string;
}

export class CrudStore<T extends { id: number }, P> {
	items = $state([]) as T[];
	loaded = $state(false);
	error = $state("");
	hint = $state("");
	formOpen = $state(false);
	editId = $state<number | null>(null);
	busy = $state(false);
	removeId = $state<number | null>(null);

	private readonly adapter: CrudAdapter<T, P>;

	constructor(adapter: CrudAdapter<T, P>) {
		this.adapter = adapter;
	}

	setError(err: unknown): void {
		this.error = describeApiError(err);
	}

	async load(): Promise<void> {
		this.error = "";
		this.hint = "";
		const res = await this.adapter.list();
		if (res.isErr()) {
			this.setError(res.error);
			return;
		}
		this.items = res.value;
		this.loaded = true;
	}

	openForm(editId: number | null): void {
		this.editId = editId;
		this.formOpen = true;
	}

	cancelForm(): void {
		this.formOpen = false;
		this.editId = null;
	}

	async save(valid: boolean, payload: P): Promise<void> {
		if (!valid) return;
		this.busy = true;
		this.error = "";
		const res =
			this.editId === null
				? await this.adapter.create(payload)
				: await this.adapter.update(this.editId, payload);
		if (res.isErr()) {
			this.setError(res.error);
		} else {
			this.hint =
				this.editId === null
					? this.adapter.hints.created
					: this.adapter.hints.updated;
			this.cancelForm();
			await this.load();
		}
		this.busy = false;
	}

	async remove(id: number): Promise<void> {
		if (this.adapter.confirmRemove && !confirm(this.adapter.confirmRemove)) {
			return;
		}
		this.removeId = id;
		this.error = "";
		const res = await this.adapter.remove(id);
		if (res.isErr()) {
			this.setError(res.error);
		} else {
			this.hint = this.adapter.hints.removed;
			await this.load();
		}
		this.removeId = null;
	}
}
