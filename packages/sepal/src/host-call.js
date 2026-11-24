// Asking the host for something that takes time.
//
// `host_call` hands the request over and returns a promise. The host does the
// work off the UI thread and answers through `__sb.resolve` on a later turn.
// Making the call only queues it on the host side, so it is safe anywhere,
// including inside an event handler.

const host = /** @type {any} */ (globalThis).__sb_host ?? {};

/** @type {Map<number, { resolve: (value: any) => void, reject: (error: Error) => void }>} */
const pending = new Map();
let next_id = 1;

/**
 * @param {string} name e.g. `updater.check`
 * @param {unknown} [args] anything JSON can carry
 * @returns {Promise<any>} the host's answer, parsed from JSON
 */
export function host_call(name, args = null) {
	if (typeof host.call !== 'function') {
		return Promise.reject(new Error(`this host cannot do ${name}`));
	}
	return new Promise((resolve, reject) => {
		const id = next_id++;
		pending.set(id, { resolve, reject });
		host.call(id, name, JSON.stringify(args));
	});
}

/**
 * The host's answer to a call. Wired to `__sb.resolve`.
 * @param {number} id
 * @param {boolean} ok
 * @param {string} payload JSON when `ok`, an error message otherwise
 */
export function settle_call(id, ok, payload) {
	const waiting = pending.get(id);
	if (waiting === undefined) return;
	pending.delete(id);
	if (ok) waiting.resolve(JSON.parse(payload));
	else waiting.reject(new Error(payload));
}
