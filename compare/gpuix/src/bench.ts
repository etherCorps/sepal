// The bench protocol (compare/shared/PROTOCOL.md): marks, the frame signal after each, and the
// RECT lines of the calibration launch. Everything here is a no-op without bench.json.

import { flushSync } from 'svelte';
import { get_native, type ShadowNode } from 'gpuix-svelte';
// Not exported by the package; the same module instance `render()` uses, so it sees the running frame loop.
import { next_frame } from '../node_modules/gpuix-svelte/lib/render.js';
import { BENCH, write_mark } from './start.ts';

export { BENCH, DATA_DIR } from './start.ts';

/**
 * `MARK <name>`, then `MARK frame:<name>` after the frame loop's next `tick()` (GPUI's
 * frame for what the mark's state change committed). `start` and `loaded` have no frame.
 */
export function mark(name: string): void {
	if (!BENCH) return;
	write_mark(name);
	if (name === 'start' || name === 'loaded') return;
	next_frame().then(() => write_mark(`frame:${name}`));
	if (BENCH.rects) queue_rects();
}

/**
 * Svelte's `tick()` without its async-mode branch: gpuix-svelte compiles with async mode on, where
 * `tick()` calls `requestAnimationFrame`, which Bun does not have (it throws). This is what
 * `tick()` does otherwise: a microtask, then `flushSync()`.
 */
export const tick = () => Promise.resolve().then(() => flushSync());

/** Mark once Svelte has flushed the change that led here (`tick()`), as the other apps do. */
export function mark_after_tick(name: string): void {
	if (BENCH) tick().then(() => mark(name));
}

// --- RECT lines (calibration launch only) -------------------------------------------------

/** The ids PROTOCOL.md lists. `row:<key>` is every mounted row; `row` the first one in view. */
const IDS = new Set([
	'list', 'search', 'row', 'pane', 'detail-close', 'status-select', 'new-issue', 'nav-new', 'new-issue-title',
	'new-issue-submit', 'nav-issues', 'nav-board', 'nav-insights', 'nav-settings', 'nav-my', 'bench-clock', 'toast', 'show-all', 'show-more'
]);
/** gpuix-svelte-ui's Select names its options `<testid>-<value>`; the protocol calls them `status-option-<status>`. */
const OPTION = /^status-select-(backlog|todo|progress|review|done|canceled)$/;

let root: ShadowNode | null = null;
let last = new Map<string, string>();
let queued = false;

/** The App's root element: the walk starts here (portals and popovers included, since Svelte renders them in place). */
export function set_rect_root(node: ShadowNode): void {
	root = node;
}

function queue_rects() {
	if (queued) return;
	queued = true;
	next_frame().then(() => {
		queued = false;
		emit_rects();
	});
}

/** Emit the RECT lines that are new or moved since the last emission. */
export function emit_rects(): void {
	if (!BENCH?.rects || root === null) return;
	const native = get_native();
	if (!native) return;
	const now = new Map<string, string>();
	const line = (b: { x: number; y: number; width: number; height: number }) =>
		`${Math.round(b.x)} ${Math.round(b.y)} ${Math.round(b.width)} ${Math.round(b.height)}`;
	const rows: { key: unknown; y: number; line: string }[] = [];
	const visit = (node: ShadowNode) => {
		const id = node.attrs.testid;
		if (typeof id === 'string' && node.nativeId !== null) {
			const option = OPTION.exec(id);
			const name = option ? `status-option-${option[1]}` : IDS.has(id) ? id : null;
			const box = name ? native.getElementBounds(node.nativeId) : null;
			if (name && box && box.width > 0 && box.height > 0) {
				if (id === 'row') rows.push({ key: node.attrs['data-key'], y: box.y, line: line(box) });
				else now.set(name, line(box));
			}
		}
		for (let child = node.first; child; child = child.next) visit(child);
	};
	visit(root);
	// Rows mounted for overscan are painted under the toolbar too; `row` is the first one inside the list's box.
	const top = Number(now.get('list')?.split(' ')[1] ?? 0);
	const first = rows.find((r) => r.y >= top);
	if (first) now.set('row', first.line);
	for (const r of rows) now.set(`row:${r.key}`, r.line);
	for (const [name, line] of now) if (last.get(name) !== line) process.stderr.write(`RECT ${name} ${line}\n`);
	last = now;
}

/** Calibration launches poll for changes too, so a Select's options or a toast report once they show. */
export function start_rect_poll(): void {
	if (BENCH?.rects) setInterval(emit_rects, 250);
}
