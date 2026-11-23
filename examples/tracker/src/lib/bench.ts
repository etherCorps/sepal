// Bench mode, for the runtime comparison in compare/ (compare/shared/PROTOCOL.md).
// On only when the data directory holds bench.json; then the issues come from
// fixture.json and the app writes MARK (and RECT) lines. Without bench.json
// nothing here prints or changes anything.
//
// Plain TypeScript, no runes: the Tauri build uses this file too, with its
// own `fs` aliased in and `__bench_emit` defined to send lines over IPC.

/// <reference types="vite/client" />
import { readTextFile } from '@ethercorps/svelte-blitz/fs';
import type { Issue } from './seed';

export interface Bench {
	clock: boolean;
	rects: boolean;
	issues: Issue[];
}

/** null until bench.json was read (or found missing); then the bench, or null for good. */
export let bench: Bench | null = null;

// Wall-clock ms. QuickJS's `timeOrigin` is a monotonic clock and V8 here has
// none, so those two fall back to Date.now() (1 ms, as WKWebView).
const origin = (globalThis as any).performance?.timeOrigin;
const app_ms = () => (typeof origin === 'number' && origin > 1e12 ? origin + performance.now() : Date.now());

/**
 * A bench build (`VITE_BENCH=1` at build time, compare/builds/make.sh and the
 * Tauri build): bench mode is known before anything runs, so the app does not
 * seed and draw 5,000 issues only to replace them, and marks go out at once.
 */
export const BENCH_BUILD = import.meta.env.VITE_BENCH === '1';

/** Marks made before bench mode is known (`start`): written if it turns on, dropped if not. */
let held: string[] | null = BENCH_BUILD ? null : [];

function emit(line: string) {
	const g = globalThis as any;
	if (g.__bench_emit) g.__bench_emit(line);
	// svelte-blitz: the host's log, level 2, is stderr (`[js error] ` in front;
	// console.log goes to stdout). Not console.error: on V8 `console` is V8's
	// own, which prints nothing without an inspector.
	else if (g.__sb_host?.log) g.__sb_host.log(2, line);
	else console.error(line);
}

export function mark(name: string): void {
	const line = `MARK ${name} ${app_ms().toFixed(1)}`;
	if (held) held.push(line);
	else if (bench || BENCH_BUILD) emit(line);
}

mark('start');

export const bench_ready: Promise<Bench | null> = (async () => {
	let config: { clock?: boolean; rects?: boolean };
	try {
		config = JSON.parse(await readTextFile('bench.json', { dir: 'data' }));
	} catch {
		held = null;
		return null;
	}
	// A missing or broken fixture in bench mode is the harness's mistake: let it throw.
	// `read` and `parsed` split start -> loaded into the file read and JSON.parse.
	const text = await readTextFile('fixture.json', { dir: 'data' });
	mark('read');
	const fixture = JSON.parse(text) as { n: number; issues: Issue[] };
	mark('parsed');
	bench = { clock: !!config.clock, rects: !!config.rects, issues: fixture.issues };
	for (const line of held ?? []) emit(line);
	held = null;
	if (bench.rects) {
		// Calibration only: a press may open a Dialog or a Select; report once it has settled.
		const later = () => setTimeout(rects, 400);
		document.addEventListener('pointerdown', later);
		document.addEventListener('keydown', later);
	}
	return bench;
})();

const IDS = [
	'list', 'search', 'pane', 'detail-close', 'status-select', 'new-issue', 'nav-new', 'new-issue-title',
	'new-issue-submit', 'nav-issues', 'nav-board', 'nav-insights', 'nav-settings', 'nav-my', 'bench-clock', 'toast',
	'show-all', 'show-more'
];

function rect(name: string, element: Element) {
	let { x, y, width, height } = element.getBoundingClientRect();
	// svelte-blitz's boxes leave CSS transforms out (dom.js), and a Select's
	// content is placed with an inline `transform: translate(Xpx, Ypx)`.
	// ponytail: only that form; other transforms are not added in.
	if ((globalThis as any).__sb_host) {
		for (let at: Element | null = element; at; at = at.parentElement) {
			const moved = /translate\((-?[\d.]+)px, (-?[\d.]+)px\)/.exec(at.getAttribute('style') ?? '');
			if (moved) (x += Number(moved[1])), (y += Number(moved[2]));
		}
	}
	if (width > 0 && height > 0) emit(`RECT ${name} ${Math.round(x)} ${Math.round(y)} ${Math.round(width)} ${Math.round(height)}`);
}

/** RECT lines for every protocol id on screen, when bench.json says `rects`. Rows as `row:<key>`, Select items as `status-option-<value>`. */
export function rects(): void {
	if (!bench?.rects) return;
	for (const id of IDS) for (const element of document.querySelectorAll(`[data-testid="${id}"]`)) rect(id, element);
	for (const element of document.querySelectorAll('[data-testid="row"][data-key]')) rect(`row:${element.getAttribute('data-key')}`, element);
	for (const element of document.querySelectorAll('[data-select-item][data-value]')) rect(`status-option-${element.getAttribute('data-value')}`, element);
}
