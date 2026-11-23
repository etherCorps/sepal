// Imported first by standalone.ts, so the bench build stamps `start` before any other module
// (Svelte, the GPUI addon, the app) is evaluated: an ES module's closest to "first statement".
if (BENCH_BUILD) process.stderr.write(`MARK start ${(performance.timeOrigin + performance.now()).toFixed(1)}\n`);

import { existsSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { BENCH_BUILD } from './build-flag.ts';

/** PROTOCOL.md: the app's data directory. */
export const DATA_DIR = join(homedir(), 'Library', 'Application Support', 'com.compare.tracker.gpuix');

/** bench.json, read once at start, in the bench build only (null otherwise). */
export const BENCH: { clock: boolean; rects: boolean } | null = (() => {
	if (!BENCH_BUILD) return null;
	const path = join(DATA_DIR, 'bench.json');
	if (!existsSync(path)) return { clock: false, rects: false };
	try {
		const json = JSON.parse(readFileSync(path, 'utf8'));
		return { clock: json.clock === true, rects: json.rects === true };
	} catch {
		return { clock: false, rects: false };
	}
})();

/** `MARK <name> <wall-clock ms>` on stderr, in bench mode only. */
export function write_mark(name: string): void {
	if (BENCH) process.stderr.write(`MARK ${name} ${(performance.timeOrigin + performance.now()).toFixed(1)}\n`);
}


