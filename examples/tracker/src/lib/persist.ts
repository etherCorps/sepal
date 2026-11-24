// What survives a restart: the user's edits, as an overlay on the seed, in
// data/edits.json, and the settings in data/settings.json. Both live in the
// app's data directory (`permissions.fs: ['data']`). Without that permission,
// or with no files yet, the app runs on the seed alone.

import { readTextFile, remove, writeTextFile } from 'sepal/fs';
import type { Comment, Issue } from './seed';

export interface Overlay {
	version: 1;
	/** issues the user made, newest first */
	created: Issue[];
	/** changed fields by key, seed issues only (created ones change in place) */
	changed: Record<string, Partial<Issue>>;
	deleted: string[];
	/** comments the user posted, by key */
	comments: Record<string, Comment[]>;
}

export interface Settings {
	displayName: string;
	compact: boolean;
	showLabels: boolean;
	defaultGroupBy: 'status' | 'assignee' | 'priority' | 'project' | 'none';
	simulate: boolean;
}

export const DEFAULT_SETTINGS: Settings = {
	displayName: 'Shivam',
	compact: false,
	showLabels: true,
	defaultGroupBy: 'status',
	simulate: false
};

export function empty_overlay(): Overlay {
	return { version: 1, created: [], changed: {}, deleted: [], comments: {} };
}

/** How many edits the overlay holds: the "37 local edits" in Settings. */
export function edit_count(overlay: Overlay): number {
	let n = overlay.created.length + overlay.deleted.length + Object.keys(overlay.changed).length;
	for (const key in overlay.comments) n += overlay.comments[key].length;
	return n;
}

const EDITS = 'edits.json';
const SETTINGS = 'settings.json';

/** For `__tracker.stats()` and Settings. */
export const persist_stats = {
	/** false once a read or write was refused or failed: edits then live in memory only */
	available: true,
	stringify_ms: 0,
	saves: 0,
	last_error: null as string | null
};

function failed(error: unknown) {
	persist_stats.available = false;
	persist_stats.last_error = String((error as any)?.message ?? error);
}

async function read_json<T>(name: string): Promise<T | null> {
	try {
		return JSON.parse(await readTextFile(name, { dir: 'data' })) as T;
	} catch {
		// No file yet, a broken one, or no permission: start from the defaults.
		return null;
	}
}

export async function load(): Promise<{ overlay: Overlay; settings: Settings }> {
	const [overlay, settings] = await Promise.all([read_json<Overlay>(EDITS), read_json<Partial<Settings>>(SETTINGS)]);
	return {
		overlay: overlay?.version === 1 ? { ...empty_overlay(), ...overlay } : empty_overlay(),
		settings: { ...DEFAULT_SETTINGS, ...settings }
	};
}

let timer: ReturnType<typeof setTimeout> | null = null;
let pending: (() => Overlay) | null = null;
/** edits.json writes and removes, one at a time: the host runs each call on its own thread, so two in flight can land out of order. */
let disk: Promise<void> = Promise.resolve();
function queue(job: () => Promise<void>): Promise<void> {
	return (disk = disk.then(job).catch(failed));
}

/**
 * Write the overlay 500 ms after the last edit. One timer, restarted by each
 * edit, so a burst of edits is one write and the app goes idle after it.
 * `get` is called when the timer fires, so the latest overlay is written.
 */
export function schedule_save(get: () => Overlay): void {
	pending = get;
	if (timer !== null) clearTimeout(timer);
	timer = setTimeout(flush, 500);
}

/** Write a scheduled save now. */
export function flush(): Promise<void> {
	if (timer !== null) clearTimeout(timer);
	timer = null;
	const get = pending;
	pending = null;
	if (get === null) return Promise.resolve();
	const start = performance.now();
	const text = JSON.stringify(get());
	persist_stats.stringify_ms = performance.now() - start;
	persist_stats.saves++;
	return queue(() => writeTextFile(EDITS, text, { dir: 'data' }));
}

export function save_settings(settings: Settings): Promise<void> {
	return writeTextFile(SETTINGS, JSON.stringify(settings), { dir: 'data' }).catch(failed);
}

/** Reset to sample data: forget every edit, on disk too. */
export function clear_edits(): Promise<void> {
	if (timer !== null) clearTimeout(timer);
	timer = null;
	pending = null;
	return queue(() => remove(EDITS, { dir: 'data' }).catch(() => {}));
}
