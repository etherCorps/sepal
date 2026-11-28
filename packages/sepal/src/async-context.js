// `AsyncLocalStorage`, as in Node's `node:async_hooks`: a value that follows
// the code through `await`, `.then`, microtasks, timers and host calls.
//
// The host carries one value through every promise job, from where a
// reaction is registered (or a job queued) to where it runs
// (`__sb_host.continuation` and `set_continuation`: QuickJS-ng #1920 patched
// into the QuickJS host, V8's continuation-preserved embedder data in the V8
// one). Timers and frame callbacks carry it in `shims.js`. That value is a
// frame here: a Map from each storage to its store, replaced, never changed,
// so that a frame a job captured stays what it was.
//
// Without that host (tests in Node), the value is a plain variable: right
// until the first `await`, as SvelteKit's own fallback is.

const host = /** @type {any} */ (globalThis).__sb_host ?? {};

/** @type {Map<AsyncLocalStorage<any>, unknown> | undefined} */
let fallback;
/** @returns {Map<AsyncLocalStorage<any>, unknown> | undefined} */
const current =
	typeof host.continuation === 'function'
		? () => {
				const value = host.continuation();
				return value instanceof Map ? value : undefined;
			}
		: () => fallback;
/** @type {(frame: Map<AsyncLocalStorage<any>, unknown> | undefined) => void} */
const set =
	typeof host.set_continuation === 'function'
		? (frame) => host.set_continuation(frame)
		: (frame) => {
				fallback = frame;
			};

/**
 * Run `fn` in `frame`, and restore the frame before.
 * @template R
 * @param {Map<AsyncLocalStorage<any>, unknown> | undefined} frame
 * @param {(...args: any[]) => R} fn
 * @param {any[]} args
 * @returns {R}
 */
function within(frame, fn, args) {
	const before = current();
	set(frame);
	try {
		return fn(...args);
	} finally {
		set(before);
	}
}

/** @template T */
export class AsyncLocalStorage {
	/**
	 * Run `fn` with `store` as this storage's value, in it and in everything
	 * it starts.
	 * @template R
	 * @param {T} store
	 * @param {(...args: any[]) => R} fn
	 * @param {...any} args
	 * @returns {R}
	 */
	run(store, fn, ...args) {
		const frame = new Map(current());
		frame.set(this, store);
		return within(frame, fn, args);
	}

	/**
	 * Run `fn` without a value for this storage.
	 * @template R
	 * @param {(...args: any[]) => R} fn
	 * @param {...any} args
	 * @returns {R}
	 */
	exit(fn, ...args) {
		const frame = new Map(current());
		frame.delete(this);
		return within(frame, fn, args);
	}

	/** @returns {T | undefined} */
	getStore() {
		return /** @type {T | undefined} */ (current()?.get(this));
	}

	/**
	 * Give this storage `store` from here on, in the current job and what it
	 * starts.
	 * @param {T} store
	 */
	enterWith(store) {
		const frame = new Map(current());
		frame.set(this, store);
		set(frame);
	}

	/** Drop this storage's value from here on. */
	disable() {
		const frame = new Map(current());
		frame.delete(this);
		set(frame);
	}

	/**
	 * The values of every storage now, to run code in later.
	 * @returns {<R>(fn: (...args: any[]) => R, ...args: any[]) => R}
	 */
	static snapshot() {
		const frame = current();
		return (fn, ...args) => within(frame, fn, args);
	}

	/**
	 * `fn`, bound to the values of every storage now.
	 * @template {(...args: any[]) => any} F
	 * @param {F} fn
	 * @returns {F}
	 */
	static bind(fn) {
		const run = AsyncLocalStorage.snapshot();
		return /** @type {F} */ ((/** @type {any[]} */ ...args) => run(fn, ...args));
	}
}
