// The handful of globals Svelte and app code expect that a bare JS engine
// does not provide. Each one is only installed when missing.
//
// The host supplies `globalThis.__sb_host`:
//   log(level: 0 | 1 | 2, message: string)
//   now(): number                milliseconds, monotonic

const g = /** @type {any} */ (globalThis);
const host = g.__sb_host ?? {};

const clock = () => (host.now ? host.now() : Date.now());

/** @param {unknown} value */
function format(value) {
	if (typeof value === 'string') return value;
	if (value instanceof Error) return value.stack ? `${value.message}\n${value.stack}` : String(value);
	try {
		return JSON.stringify(value) ?? String(value);
	} catch {
		return String(value);
	}
}

if (typeof g.console === 'undefined') {
	const make = (/** @type {number} */ level) => (/** @type {unknown[]} */ ...args) => host.log?.(level, args.map(format).join(' '));
	const noop = () => {};
	g.console = {
		log: make(0),
		info: make(0),
		debug: make(0),
		trace: make(0),
		warn: make(1),
		error: make(2),
		group: noop,
		groupCollapsed: noop,
		groupEnd: noop
	};
}

if (typeof g.queueMicrotask !== 'function') {
	g.queueMicrotask = (/** @type {() => void} */ fn) => {
		Promise.resolve().then(fn);
	};
}

if (typeof g.performance === 'undefined') {
	g.performance = { now: clock };
}

// ---------------------------------------------------------------------------
// Timers
//
// Timers live in JS. The host only needs to know the next deadline, which it
// reads with `next_timer()` at the end of each turn. Nothing ticks while no
// timer is pending.
// ---------------------------------------------------------------------------

/** @typedef {{ fn: Function, args: any[], at: number, every: number }} Timer */

/** @type {Map<number, Timer>} */
const timers = new Map();
let next_timer_id = 1;

/**
 * @param {unknown} fn
 * @param {unknown} delay
 * @param {number} every 0 for a one-shot timer
 * @param {any[]} args
 */
function add_timer(fn, delay, every, args) {
	if (typeof fn !== 'function') return 0;
	const id = next_timer_id++;
	const at = clock() + Math.max(0, Number(delay) || 0);
	timers.set(id, { fn, args, at, every });
	return id;
}

/**
 * Run every timer that is due. Timers created by these callbacks (or by the
 * microtasks they queue) are picked up by `next_timer()`.
 */
export function run_timers() {
	if (timers.size === 0) return;

	const now = clock();
	/** @type {[number, Timer][]} */
	const due = [];
	for (const [id, timer] of timers) {
		if (timer.at <= now) due.push([id, timer]);
	}
	due.sort((a, b) => a[1].at - b[1].at || a[0] - b[0]);

	for (const [id, timer] of due) {
		if (!timers.has(id)) continue; // cleared by an earlier callback
		if (timer.every > 0) timer.at = now + timer.every;
		else timers.delete(id);
		try {
			timer.fn(...timer.args);
		} catch (error) {
			g.console.error('Error in timer callback:', error);
		}
	}
}

/**
 * The host asks this at the end of every turn, after microtasks have run.
 * @returns {number} the earliest deadline on the host clock, or -1 if no timer is pending
 */
export function next_timer() {
	let next = -1;
	for (const timer of timers.values()) {
		if (next === -1 || timer.at < next) next = timer.at;
	}
	return next;
}

if (typeof g.setTimeout !== 'function') {
	g.setTimeout = (/** @type {unknown} */ fn, /** @type {unknown} */ delay, /** @type {any[]} */ ...args) =>
		add_timer(fn, delay, 0, args);
	g.clearTimeout = (/** @type {number} */ id) => void timers.delete(id);
	g.setInterval = (/** @type {unknown} */ fn, /** @type {unknown} */ delay, /** @type {any[]} */ ...args) =>
		add_timer(fn, delay, Math.max(1, Number(delay) || 0), args);
	g.clearInterval = g.clearTimeout;
}

if (typeof g.requestAnimationFrame !== 'function') {
	// TODO: tie this to the host's redraw instead of a 16ms timer.
	g.requestAnimationFrame = (/** @type {(time: number) => void} */ fn) => g.setTimeout(() => fn(clock()), 16);
	g.cancelAnimationFrame = g.clearTimeout;
}
