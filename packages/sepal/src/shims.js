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

// ---------------------------------------------------------------------------
// Engine fix: accessor descriptors through a Proxy
//
// In QuickJS-ng, `Object.getOwnPropertyDescriptor(proxy, key)` returns a data
// descriptor (`value`, `writable`) even when the proxy's trap returned an
// accessor (`get`, `set`). Svelte asks exactly that question to find the
// setter of a bound prop that travels through `{...rest}` props, so without
// this a `bind:` stops at the first component that also spreads its props.
//
// The fix: remember each proxy's handler, and answer from its trap directly.
// ---------------------------------------------------------------------------

(() => {
	const probe = new Proxy({}, { getOwnPropertyDescriptor: () => ({ get: () => 1, set() {}, enumerable: true, configurable: true }) });
	// An engine without the bug keeps its own, complete, behaviour.
	if (typeof Object.getOwnPropertyDescriptor(probe, 'x')?.set === 'function') return;

	const NativeProxy = Proxy;
	/** @type {WeakMap<object, { target: object, handler: ProxyHandler<any> }>} */
	const proxies = new WeakMap();
	/** @this {any} @param {object} target @param {ProxyHandler<any>} handler */
	const Tracked = function (target, handler) {
		const proxy = new NativeProxy(target, handler);
		proxies.set(proxy, { target, handler });
		return proxy;
	};
	Tracked.revocable = (/** @type {object} */ target, /** @type {ProxyHandler<any>} */ handler) => {
		const result = NativeProxy.revocable(target, handler);
		proxies.set(result.proxy, { target, handler });
		return result;
	};
	g.Proxy = Tracked;

	for (const owner of [Object, Reflect]) {
		const native = owner.getOwnPropertyDescriptor;
		owner.getOwnPropertyDescriptor = function (/** @type {any} */ object, /** @type {PropertyKey} */ key) {
			const known = object !== null && typeof object === 'object' ? proxies.get(object) : undefined;
			const trap = known?.handler.getOwnPropertyDescriptor;
			// Only an accessor is lost, so only an accessor is answered from here.
			const answer = typeof trap === 'function' ? trap.call(known?.handler, known?.target, /** @type {any} */ (key)) : undefined;
			if (answer && (answer.get || answer.set)) return { enumerable: false, configurable: false, ...answer };
			return native.call(owner, object, key);
		};
	}
})();

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

// One clock for scripts, timers and frame timestamps: the host's, which
// stands still headless until a test moves it. QuickJS-ng has a `performance`
// of its own, on real time, whose `now` cannot be replaced; animation code
// compares a frame's timestamp with `performance.now()`, so the whole object
// is. Its `timeOrigin` stays.
if (typeof g.performance === 'undefined' || host.now) {
	g.performance = { ...g.performance, now: clock };
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

// ---------------------------------------------------------------------------
// Promises rejected with nobody listening
//
// The engine tells the host about every such promise, and again if a handler
// turns up later; the host passes both on. Once the microtasks of a turn have
// run, whatever is still unhandled is reported, the way a browser's console
// says "Uncaught (in promise)". Without this an `async` function that throws
// fails without a word.
// ---------------------------------------------------------------------------

/** @type {Map<object, unknown>} the promise, and what it was rejected with */
const unhandled = new Map();
let rejections_reported = 0;

/** @param {object} promise @param {unknown} reason @param {boolean} handled */
export function track_rejection(promise, reason, handled) {
	if (handled) unhandled.delete(promise);
	else unhandled.set(promise, reason);
}

/** Called by the host when a turn's microtasks are done. */
export function report_rejections() {
	for (const reason of unhandled.values()) {
		rejections_reported++;
		g.console.error('Uncaught (in promise):', reason);
	}
	unhandled.clear();
}

/** How many have been reported, for tests. */
export const rejection_count = () => rejections_reported;

/**
 * The timers that were due when the host started this round, soonest first.
 * @type {[number, Timer][] | null}
 */
let round = null;

/**
 * Run one timer that is due, and say whether there was one. The host calls
 * this until the answer is no, and runs microtasks after every call. That is
 * how a browser does it, each timer a task of its own: what one callback
 * changed, and the effects that follow from it, are in place when the next
 * callback runs.
 *
 * A timer that comes due during a round waits for the next one, so an endless
 * chain of `setTimeout(fn, 0)` cannot keep a frame from being drawn.
 *
 * @returns {boolean}
 */
export function run_timers() {
	if (round === null) {
		if (timers.size === 0) return false;
		const now = clock();
		round = [];
		for (const entry of timers) {
			if (entry[1].at <= now) round.push(entry);
		}
		round.sort((a, b) => a[1].at - b[1].at || a[0] - b[0]);
	}

	for (;;) {
		const next = round.shift();
		if (next === undefined) {
			round = null;
			return false;
		}
		const [id, timer] = next;
		if (!timers.has(id)) continue; // cleared by an earlier callback
		if (timer.every > 0) timer.at = clock() + timer.every;
		else timers.delete(id);
		try {
			timer.fn(...timer.args);
		} catch (error) {
			g.console.error('Error in timer callback:', error);
		}
		return true;
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

// ---------------------------------------------------------------------------
// Animation frames
//
// A `requestAnimationFrame` callback runs when the host draws its next frame,
// before it lays the page out and paints. The host asks `frame_pending()` at
// the end of every turn and draws a frame only when the answer is yes, so
// with no callback queued nothing is drawn and nothing wakes it.
// ---------------------------------------------------------------------------

/** Queued callbacks, in the order they were queued. @type {Map<number, (time: number) => void>} */
const frame_callbacks = new Map();
let next_frame_id = 1;
/** The callbacks that were queued when the host started this frame. @type {number[] | null} */
let frame = null;

/**
 * Run one callback of this frame, and say whether there was one. As with
 * timers, the host calls this until the answer is no, and runs microtasks
 * after every call. Every callback of a frame gets the same `time`; one
 * queued during the frame runs in the next.
 *
 * @param {number} time the frame's timestamp, on the host clock
 * @returns {boolean}
 */
export function run_frame(time) {
	if (frame === null) {
		if (frame_callbacks.size === 0) return false;
		frame = [...frame_callbacks.keys()];
	}
	for (;;) {
		const id = frame.shift();
		if (id === undefined) {
			frame = null;
			return false;
		}
		const fn = frame_callbacks.get(id);
		if (fn === undefined) continue; // cancelled by an earlier callback
		frame_callbacks.delete(id);
		try {
			fn(time);
		} catch (error) {
			g.console.error('Error in requestAnimationFrame callback:', error);
		}
		return true;
	}
}

/** Whether the app waits for a frame. The host asks at the end of every turn. */
export const frame_pending = () => frame_callbacks.size > 0;

if (typeof g.requestAnimationFrame !== 'function') {
	g.requestAnimationFrame = (/** @type {unknown} */ fn) => {
		if (typeof fn !== 'function') throw new TypeError('requestAnimationFrame: the callback is not a function');
		const id = next_frame_id++;
		frame_callbacks.set(id, /** @type {(time: number) => void} */ (fn));
		return id;
	};
	g.cancelAnimationFrame = (/** @type {number} */ id) => void frame_callbacks.delete(id);
}
