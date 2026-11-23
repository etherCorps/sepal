// UI state shared across screens: overlays, toasts, the list's highlight and
// checked rows, and the keymap.
//
// Keys: the shell owns the one document `keydown` listener (`handle_key`).
// Screens add theirs with `use_keys({ j: next, k: previous })` while they are
// mounted. The newest registration is asked first, so the detail pane's j/k
// win over the list's while it is open.

import { tick } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';
import { mark as bench_mark } from './bench';

/**
 * One row's highlight and checkbox. Rows read these instead of comparing
 * `ui.highlighted` or asking `checked.has()`, so j/k and a click wake the rows
 * that changed, not every mounted one (5,000 after Show all).
 */
class Mark {
	active = $state(false);
	checked = $state(false);
}
// ponytail: one Mark per key ever shown, never dropped; fine at 20,000 issues.
const marks = new Map<string, Mark>();

/**
 * The row state for `key`. Read it through a `$derived` (Row does): Svelte
 * does not track state created by the effect that reads it, and the first
 * read creates it.
 */
export function mark(key: string): Mark {
	let m = marks.get(key);
	if (m === undefined) marks.set(key, (m = new Mark()));
	return m;
}

class Ui {
	palette = $state(false);
	new_issue = $state(false);
	shortcuts = $state(false);
	#highlighted = $state<string | null>(null);
	/** where Shift+click starts a range: the last row clicked or checked */
	anchor = $state<string | null>(null);
	/** the bulk bar is up: toasts stack above it */
	bulk_bar = $state(false);

	/** the issue j/k moves; kept when the pane closes */
	get highlighted(): string | null {
		return this.#highlighted;
	}
	set highlighted(key: string | null) {
		if (key === this.#highlighted) return;
		if (this.#highlighted !== null) mark(this.#highlighted).active = false;
		if (key !== null) mark(key).active = true;
		this.#highlighted = key;
		// Bench only (bench.ts drops marks otherwise): the first input a fresh
		// window accepts is a `j` (compare/shared/PROTOCOL.md, scenarios 1-2).
		if (key !== null) tick().then(() => bench_mark(`highlight:${key}`));
	}
}

export const ui = new Ui();

/** Checked rows' keys, for the bulk bar. Keeps each row's `mark(key).checked` in step. */
class Checked extends SvelteSet<string> {
	add(key: string) {
		mark(key).checked = true;
		return super.add(key);
	}
	delete(key: string) {
		if (this.has(key)) mark(key).checked = false;
		return super.delete(key);
	}
	clear() {
		for (const key of this) mark(key).checked = false;
		super.clear();
	}
}
export const checked = new Checked();

// ---------------------------------------------------------------------------
// Toasts
// ---------------------------------------------------------------------------

export interface Toast {
	id: number;
	text: string;
	/** a status glyph at the left, when the toast is about a status */
	status?: import('./seed').StatusId;
	tone?: 'default' | 'error';
	/** `Undo` or `Open`: runs, then the toast goes */
	action?: { label: string; run: () => void };
}

export const MAX_TOASTS = 3;
export const TOAST_MS = 5000;

export const toasts = $state<Toast[]>([]);
const toast_timers = new Map<number, ReturnType<typeof setTimeout>>();
let next_toast = 1;

/** Show a toast for 5 s. At most 3 show; a fourth pushes out the oldest. Returns its id. */
export function push(toast: Omit<Toast, 'id'>): number {
	const id = next_toast++;
	toasts.push({ ...toast, id });
	while (toasts.length > MAX_TOASTS) dismiss(toasts[0].id);
	// One-shot: when the last toast goes, so does the last timer.
	toast_timers.set(
		id,
		setTimeout(() => dismiss(id), TOAST_MS)
	);
	return id;
}

export function dismiss(id: number): void {
	const timer = toast_timers.get(id);
	if (timer !== undefined) clearTimeout(timer);
	toast_timers.delete(id);
	const at = toasts.findIndex((t) => t.id === id);
	if (at !== -1) toasts.splice(at, 1);
}

/** Run a toast's action and close it. */
export function act(id: number): void {
	const toast = toasts.find((t) => t.id === id);
	dismiss(id);
	toast?.action?.run();
}

// ---------------------------------------------------------------------------
// Keys
// ---------------------------------------------------------------------------

/**
 * Return `false` to say "not mine", and the next handler is asked. Anything
 * else counts as handled: the event's default is prevented and no one else
 * sees it.
 */
export type KeyHandler = (event: KeyboardEvent) => boolean | void;

/**
 * Key names, as `combo(event)` makes them:
 * - a printable key is itself, lower case for letters: `j`, `/`, `?`, `>`
 * - Shift is written only for letters and named keys: `Shift+h`, `Shift+Enter`
 * - `Mod+` is Cmd or Ctrl: `Mod+k`, `Mod+Enter`
 * - `Alt+` before them when held
 * - named keys as `KeyboardEvent.key` has them: `Enter`, `Escape`, `ArrowDown`, ` ` (space)
 * - chords, space separated: `g i`
 */
export type KeyMap = Record<string, KeyHandler>;

const layers: KeyMap[] = [];

/**
 * Listen to keys while the calling component is mounted. Call it during
 * component setup. Handlers are looked up when a key arrives, so a map can
 * close over state.
 */
export function use_keys(map: KeyMap): void {
	$effect(() => {
		layers.push(map);
		return () => {
			const at = layers.lastIndexOf(map);
			if (at !== -1) layers.splice(at, 1);
		};
	});
}

export function combo(event: KeyboardEvent): string {
	const key = event.key;
	const letter = key.length === 1 && key.toLowerCase() !== key.toUpperCase();
	let name = letter ? key.toLowerCase() : key;
	if (event.shiftKey && (letter || key.length > 1)) name = 'Shift+' + name;
	if (event.altKey) name = 'Alt+' + name;
	if (event.metaKey || event.ctrlKey) name = 'Mod+' + name;
	return name;
}

/** Is the user typing into something? Then single-key shortcuts stay out of the way. */
export function typing(): boolean {
	const active: any = (globalThis as any).document?.activeElement;
	const name = active?.nodeName;
	return name === 'INPUT' || name === 'TEXTAREA' || name === 'SELECT' || active?.isContentEditable === true;
}

/** Does Enter or Space on the focused element already do something (a button, a link, a checkbox)? */
function activates(): boolean {
	const active: any = (globalThis as any).document?.activeElement;
	if (!active || active.nodeName === 'BODY') return false;
	if (active.nodeName === 'BUTTON' || (active.nodeName === 'A' && active.hasAttribute?.('href'))) return true;
	const role = active.getAttribute?.('role');
	return role !== null && role !== undefined && role !== 'none' && role !== 'presentation';
}

const LAYER = '[data-select-content], [role=dialog], [role=alertdialog]';

/**
 * A Bits UI layer (Select list, Dialog) is open: it handles its own keys.
 * Bits UI portals layers to <body>, so only its children and their first
 * child are looked at: a `querySelector` walks every row, 150 ms per key after Show all.
 */
function layer_open(): boolean {
	const body: any = (globalThis as any).document?.body;
	for (let node = body?.firstElementChild; node; node = node.nextElementSibling) {
		if (node.matches(LAYER) || node.firstElementChild?.matches(LAYER)) return true;
	}
	return false;
}

const CHORD_MS = 1000;
let chord: string | null = null;
let chord_timer: ReturnType<typeof setTimeout> | null = null;

function end_chord() {
	chord = null;
	if (chord_timer !== null) clearTimeout(chord_timer);
	chord_timer = null;
}

function run(name: string, event: KeyboardEvent): boolean {
	for (let i = layers.length - 1; i >= 0; i--) {
		const handler = layers[i][name];
		if (handler && handler(event) !== false) {
			event.preventDefault();
			return true;
		}
	}
	return false;
}

const MODIFIERS = new Set(['Shift', 'Meta', 'Control', 'Alt', 'CapsLock']);

/** Names that start a chord: the second key has 1 s to arrive. */
const CHORD_STARTS = new Set(['g']);

/**
 * The shell's document keydown listener. Escape goes, in order, to an open
 * Select, then Dialog (palette, New issue, shortcuts and confirms are Dialogs;
 * Bits UI closes them), then the screens' `Escape` handlers (the detail pane),
 * then the newest toast.
 */
export function handle_key(event: KeyboardEvent): void {
	if (event.defaultPrevented || event.isComposing) return;
	if (MODIFIERS.has(event.key)) return;
	const name = combo(event);

	if (name === 'Escape') {
		end_chord();
		if (layer_open()) return;
		if (run('Escape', event)) return;
		const newest = toasts[toasts.length - 1];
		if (newest) {
			dismiss(newest.id);
			event.preventDefault();
		}
		return;
	}

	const modified = name.startsWith('Mod+') || name.startsWith('Alt+');
	// Single keys are text while typing; Mod combos still work.
	if (!modified && typing()) return;
	// A Select list or Dialog owns single keys while it is open: j, x or g b must not act behind it.
	if (!modified && layer_open()) return;
	if ((name === 'Enter' || name === ' ') && activates()) return;

	if (chord !== null) {
		const whole = `${chord} ${name}`;
		end_chord();
		run(whole, event);
		return;
	}
	if (CHORD_STARTS.has(name) && layers.some((map) => Object.keys(map).some((k) => k.startsWith(name + ' ')))) {
		chord = name;
		chord_timer = setTimeout(end_chord, CHORD_MS);
		event.preventDefault();
		return;
	}
	run(name, event);
}

/** For the shortcuts dialog. `keys` as written in KeyMap. */
export const SHORTCUTS: { group: 'Navigation' | 'List' | 'Issue' | 'Global'; keys: string; label: string }[] = [
	{ group: 'Global', keys: 'Mod+k', label: 'Command palette' },
	{ group: 'Global', keys: 'c', label: 'New issue' },
	{ group: 'Global', keys: '/', label: 'Search' },
	{ group: 'Global', keys: '?', label: 'Keyboard shortcuts' },
	{ group: 'Navigation', keys: 'g i', label: 'Go to Issues' },
	{ group: 'Navigation', keys: 'g m', label: 'Go to My issues' },
	{ group: 'Navigation', keys: 'g b', label: 'Go to Board' },
	{ group: 'Navigation', keys: 'g n', label: 'Go to Insights' },
	{ group: 'Navigation', keys: 'g s', label: 'Go to Settings' },
	{ group: 'List', keys: 'j', label: 'Next issue' },
	{ group: 'List', keys: 'k', label: 'Previous issue' },
	{ group: 'List', keys: 'Enter', label: 'Open issue' },
	{ group: 'List', keys: 'x', label: 'Check issue' },
	{ group: 'Issue', keys: 'Escape', label: 'Close the issue' },
	{ group: 'Issue', keys: 's', label: 'Set status (board)' },
	{ group: 'Issue', keys: 'Shift+h', label: 'Move card left' },
	{ group: 'Issue', keys: 'Shift+l', label: 'Move card right' }
];

/**
 * `Mod+k` -> `⌘K`, `g i` -> `G then I`: how SHORTCUTS keys are shown.
 * Words for Shift and Enter: the fonts' latin subsets have ⌘ but no ⇧ or ↵.
 */
export function key_label(keys: string): string {
	return keys
		.split(' ')
		.map((part) =>
			part
				.replace('Mod+', '⌘')
				.replace('Shift+', 'Shift ')
				.replace('Alt+', '⌥')
								.replace('Escape', 'Esc')
				.replace(/^[a-z]$/, (c) => c.toUpperCase())
				.replace(/(⌘|Shift |⌥)([a-z])$/, (_, m, c) => m + c.toUpperCase())
		)
		.join(' then ');
}
