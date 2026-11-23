// UI state shared across screens: overlays, toasts, the list's highlight and
// checked rows, and the keymap.
//
// Keys: the shell owns the one document `keydown` listener (`handle_key`).
// Screens add theirs with `use_keys({ j: next, k: previous })` while they are
// mounted. The newest registration is asked first, so the detail pane's j/k
// win over the list's while it is open.

//
// gpuix port: keys arrive through `on_window_key` as GPUI key events; `key_event()` turns one
// into the KeyboardEvent fields this module reads, so the keymap and `combo()` are unchanged.
// Escape reaches the shell through a dismiss layer under every other (App.svelte), which is
// how gpuix-svelte gives open Selects and Dialogs the key first.

import { untrack } from 'svelte';
import { SvelteSet } from 'svelte/reactivity';
import { focused_element, type GpuixEvent, type ShadowNode } from 'gpuix-svelte';
import { mark_after_tick } from './bench.ts';

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
	/** gpuix port: confirm Dialogs open (bulk delete, reset), for `layer_open()` */
	modals = $state(0);
	/** gpuix port: the search field, for `/` (the Tracker finds it with querySelector) */
	search: ShadowNode | null = null;

	/** the issue j/k moves; kept when the pane closes */
	get highlighted(): string | null {
		return this.#highlighted;
	}
	set highlighted(key: string | null) {
		if (key === this.#highlighted) return;
		if (this.#highlighted !== null) mark(this.#highlighted).active = false;
		if (key !== null) mark(key).active = true;
		this.#highlighted = key;
		if (key !== null) mark_after_tick(`highlight:${key}`);
	}
}

export const ui = new Ui();

/** gpuix port: count a confirm Dialog as an open layer while `open()` holds. Call during component setup. */
export function use_modal(open: () => boolean): void {
	$effect(() => {
		if (!open()) return;
		untrack(() => ui.modals++);
		return () => untrack(() => ui.modals--);
	});
}

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
	status?: import('./seed.ts').StatusId;
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
export type KeyHandler = (event: KeyEvent) => boolean | void;

/** The KeyboardEvent fields the keymap reads; `editing` is gpuix's "a text field has focus". */
export interface KeyEvent {
	key: string;
	shiftKey: boolean;
	altKey: boolean;
	metaKey: boolean;
	ctrlKey: boolean;
	editing: boolean;
	defaultPrevented: boolean;
	preventDefault(): void;
}

/** GPUI's key names (`enter`, `down`, `space`) as KeyboardEvent.key has them. */
const NAMED: Record<string, string> = {
	enter: 'Enter', escape: 'Escape', up: 'ArrowUp', down: 'ArrowDown', left: 'ArrowLeft', right: 'ArrowRight',
	space: ' ', tab: 'Tab', backspace: 'Backspace', delete: 'Delete', home: 'Home', end: 'End', pageup: 'PageUp', pagedown: 'PageDown'
};

export function key_event(e: GpuixEvent): KeyEvent {
	const m = e.modifiers ?? { shift: false, ctrl: false, alt: false, cmd: false };
	const raw = e.key ?? '';
	// The character typed (`?` for Shift+/, `H` for Shift+h), as a browser's `key`; named keys by name.
	const char = e.keyChar && e.keyChar.length === 1 && !m.cmd && !m.ctrl && !m.alt ? e.keyChar : null;
	const key = NAMED[raw] ?? char ?? raw;
	return {
		key,
		shiftKey: m.shift,
		altKey: m.alt,
		metaKey: m.cmd,
		ctrlKey: m.ctrl,
		editing: e.editing,
		get defaultPrevented() {
			return e.defaultPrevented;
		},
		preventDefault: () => e.preventDefault()
	};
}

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

export function combo(event: KeyEvent): string {
	const key = event.key;
	const letter = key.length === 1 && key.toLowerCase() !== key.toUpperCase();
	let name = letter ? key.toLowerCase() : key;
	if (event.shiftKey && (letter || key.length > 1)) name = 'Shift+' + name;
	if (event.altKey) name = 'Alt+' + name;
	if (event.metaKey || event.ctrlKey) name = 'Mod+' + name;
	return name;
}

/** Is the user typing into something? Then single-key shortcuts stay out of the way. */
export function typing(event: KeyEvent): boolean {
	return event.editing;
}

/** Does Enter or Space on the focused element already do something (a button, a link, a checkbox)? */
function activates(): boolean {
	const active = focused_element();
	if (!active) return false;
	if (active.name === 'button') return true;
	const role = active.attrs.role;
	return typeof role === 'string' && role !== 'none' && role !== 'presentation';
}

/**
 * A layer (Select list, Dialog) is open: it handles its own keys. gpuix-svelte keeps its layer
 * stack to itself, so: the app's own overlays by their flags, and a gpuix-svelte-ui Select by its
 * trigger, which keeps the focus while its list is open.
 */
function layer_open(): boolean {
	if (ui.palette || ui.new_issue || ui.shortcuts || ui.modals > 0) return true;
	return focused_element()?.attrs['aria-expanded'] === true;
}

const CHORD_MS = 1000;
let chord: string | null = null;
let chord_timer: ReturnType<typeof setTimeout> | null = null;

function end_chord() {
	chord = null;
	if (chord_timer !== null) clearTimeout(chord_timer);
	chord_timer = null;
}

function run(name: string, event: KeyEvent): boolean {
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
export function handle_key(event: KeyEvent): void {
	if (event.defaultPrevented) return;
	if (MODIFIERS.has(event.key)) return;
	const name = combo(event);

	// Escape: `escape()`, from the dismiss layer under every other, once no Select or Dialog took it.
	if (name === 'Escape') {
		end_chord();
		return;
	}

	const modified = name.startsWith('Mod+') || name.startsWith('Alt+');
	// Single keys are text while typing; Mod combos still work.
	if (!modified && typing(event)) return;
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

/** Escape that no open layer took: the screens' `Escape` handlers (the detail pane), then the newest toast. */
export function escape(): void {
	const event: KeyEvent = { key: 'Escape', shiftKey: false, altKey: false, metaKey: false, ctrlKey: false, editing: false, defaultPrevented: false, preventDefault() {} };
	if (run('Escape', event)) return;
	const newest = toasts[toasts.length - 1];
	if (newest) dismiss(newest.id);
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
