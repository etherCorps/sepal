// `AbortController` and `AbortSignal`, for `fetch`, streams and listeners
// (`addEventListener(..., { signal })`). Installed as globals by `web.js`.
//
// A signal is an event target of its own here, not the DOM layer's
// `EventTarget`: the DOM layer's `Event` and `EventTarget` know about the
// renderer's events, and with the DOM layer loaded its globals stay its own.

/** An `abort` event: what a signal hands its listeners. */
class AbortEvent {
	/** @param {AbortSignal} target */
	constructor(target) {
		this.type = 'abort';
		this.target = this.currentTarget = target;
		this.bubbles = false;
		this.cancelable = false;
		this.defaultPrevented = false;
		this.timeStamp = /** @type {any} */ (globalThis).performance?.now?.() ?? 0;
	}
	preventDefault() {}
	stopPropagation() {}
	stopImmediatePropagation() {}
}

/** Why a signal was aborted, when no reason was given: the error `fetch` rejects with. @param {string} message @param {string} name */
export function abort_error(message, name = 'AbortError') {
	const DOMException = /** @type {any} */ (globalThis).DOMException;
	if (typeof DOMException === 'function') return new DOMException(message, name);
	const error = new Error(message);
	error.name = name;
	return error;
}

export class AbortSignal {
	/** @type {Map<string, Set<Function>>} */
	#listeners = new Map();

	constructor() {
		this.aborted = false;
		/** @type {any} */
		this.reason = undefined;
		/** @type {((event: AbortEvent) => void) | null} */
		this.onabort = null;
	}

	throwIfAborted() {
		if (this.aborted) throw this.reason;
	}

	/** @param {string} type @param {any} handler @param {any} [options] */
	addEventListener(type, handler, options) {
		if (handler == null) return;
		let set = this.#listeners.get(type);
		if (set === undefined) this.#listeners.set(type, (set = new Set()));
		const once = typeof options === 'object' && options !== null && options.once;
		const listener = once
			? (/** @type {AbortEvent} */ event) => {
					this.removeEventListener(type, listener);
					call(handler, this, event);
				}
			: handler;
		if (once) set.add(listener);
		else set.add(handler);
		const signal = typeof options === 'object' && options !== null ? options.signal : null;
		if (signal instanceof AbortSignal) signal.addEventListener('abort', () => this.removeEventListener(type, listener), { once: true });
	}

	/** @param {string} type @param {any} handler */
	removeEventListener(type, handler) {
		this.#listeners.get(type)?.delete(handler);
	}

	/** @param {{ type: string }} event */
	dispatchEvent(event) {
		if (event.type === 'abort' && typeof this.onabort === 'function') this.onabort(/** @type {AbortEvent} */ (event));
		for (const handler of [...(this.#listeners.get(event.type) ?? [])]) call(handler, this, event);
		return true;
	}

	/** @param {any} [reason] */
	static abort(reason) {
		const controller = new AbortController();
		controller.abort(reason);
		return controller.signal;
	}

	/** A signal that aborts after `ms` milliseconds, with a `TimeoutError`. @param {number} ms */
	static timeout(ms) {
		const controller = new AbortController();
		/** @type {any} */ (globalThis).setTimeout(() => controller.abort(abort_error('the operation timed out', 'TimeoutError')), ms);
		return controller.signal;
	}

	/** A signal that aborts when the first of `signals` does, with its reason. @param {Iterable<AbortSignal>} signals */
	static any(signals) {
		const controller = new AbortController();
		for (const signal of signals) {
			if (signal.aborted) {
				controller.abort(signal.reason);
				break;
			}
			signal.addEventListener('abort', () => controller.abort(signal.reason), { once: true });
		}
		return controller.signal;
	}
}

/** @param {any} handler @param {AbortSignal} target @param {any} event */
function call(handler, target, event) {
	try {
		if (typeof handler === 'function') handler.call(target, event);
		else handler.handleEvent(event);
	} catch (error) {
		/** @type {any} */ (globalThis).console?.error?.('Error in an abort listener:', error);
	}
}

export class AbortController {
	constructor() {
		this.signal = new AbortSignal();
	}
	/** @param {any} [reason] */
	abort(reason) {
		const signal = this.signal;
		if (signal.aborted) return;
		signal.aborted = true;
		signal.reason = reason === undefined ? abort_error('signal is aborted without reason') : reason;
		signal.dispatchEvent(new AbortEvent(signal));
	}
}
