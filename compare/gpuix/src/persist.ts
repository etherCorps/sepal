// examples/tracker/src/lib/persist.ts with Node/Bun `fs` in place of `sepal/fs`:
// edits.json and settings.json in the app's data directory. Otherwise unchanged.

import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { DATA_DIR, write_mark } from './start.ts';
import type { Comment, Issue } from './seed.ts';

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

const path = (name: string) => join(DATA_DIR, name);

async function write_text(name: string, text: string): Promise<void> {
	await mkdir(DATA_DIR, { recursive: true });
	await writeFile(path(name), text);
}

async function read_json<T>(name: string): Promise<T | null> {
	try {
		return JSON.parse(await readFile(path(name), 'utf8')) as T;
	} catch {
		// No file yet or a broken one: start from the defaults.
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

/** Bench mode: `fixture.json` from the data directory, `{ n, issues }`; marks `read` and `parsed` (no frame marks). */
export async function load_fixture(): Promise<{ n: number; issues: Issue[] }> {
	const text = await readFile(path('fixture.json'), 'utf8');
	write_mark('read');
	const fixture = JSON.parse(text);
	write_mark('parsed');
	return fixture;
}

let timer: ReturnType<typeof setTimeout> | null = null;
let pending: (() => Overlay) | null = null;
let disk: Promise<void> = Promise.resolve();
function queue(job: () => Promise<void>): Promise<void> {
	return (disk = disk.then(job).catch(failed));
}

/** Write the overlay 500 ms after the last edit. */
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
	return queue(() => write_text(EDITS, text));
}

export function save_settings(settings: Settings): Promise<void> {
	return write_text(SETTINGS, JSON.stringify(settings)).catch(failed);
}

/** Reset to sample data: forget every edit, on disk too. */
export function clear_edits(): Promise<void> {
	if (timer !== null) clearTimeout(timer);
	timer = null;
	pending = null;
	return queue(() => rm(path(EDITS), { force: true }).catch(() => {}));
}
