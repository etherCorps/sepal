// `fetch`, installed as a global when the engine has none.
//
// The host makes the request, and only to URLs the app's `permissions.http`
// allows. This is the everyday part of the Fetch API: method, headers, a
// string or byte body, and `text()`, `json()`, `arrayBuffer()`, `bytes()` on
// the response. There is no streaming, no `AbortSignal`, and no `Request`,
// `Headers` or `FormData` classes.

import { host_call } from './host-call.js';
import { decode, encode, to_bytes } from './base64.js';

/** Response headers: names are case-insensitive, as in the Fetch API. */
class ResponseHeaders {
	/** @param {[string, string][]} pairs */
	constructor(pairs) {
		/** @type {Map<string, string>} */
		this.values = new Map();
		for (const [name, value] of pairs) {
			const key = name.toLowerCase();
			const earlier = this.values.get(key);
			this.values.set(key, earlier === undefined ? value : `${earlier}, ${value}`);
		}
	}
	/** @param {string} name */
	get(name) {
		return this.values.get(name.toLowerCase()) ?? null;
	}
	/** @param {string} name */
	has(name) {
		return this.values.has(name.toLowerCase());
	}
	/** @param {(value: string, name: string) => void} callback */
	forEach(callback) {
		this.values.forEach(callback);
	}
	entries() {
		return this.values.entries();
	}
	[Symbol.iterator]() {
		return this.values.entries();
	}
}

class FetchResponse {
	/** @param {{ id: number, status: number, status_text: string, url: string, redirected: boolean, headers: [string, string][] }} head */
	constructor(head) {
		this.status = head.status;
		this.statusText = head.status_text;
		this.ok = head.status >= 200 && head.status < 300;
		this.url = head.url;
		this.redirected = head.redirected;
		this.headers = new ResponseHeaders(head.headers);
		this.bodyUsed = false;
		/** The body stays with the host until it is asked for. */
		this.body_id = head.id;
	}

	/** @param {'text' | 'base64'} format @returns {Promise<string>} */
	read(format) {
		if (this.bodyUsed) return Promise.reject(new TypeError('the response body was already read'));
		this.bodyUsed = true;
		return host_call('http.body', { id: this.body_id, as: format });
	}

	text() {
		return this.read('text');
	}
	async json() {
		return JSON.parse(await this.read('text'));
	}
	async bytes() {
		return decode(await this.read('base64'));
	}
	async arrayBuffer() {
		return (await this.bytes()).buffer;
	}
}

/**
 * @param {unknown} headers a plain object, a list of pairs, or anything with `forEach`
 * @returns {[string, string][]}
 */
function header_pairs(headers) {
	if (headers == null) return [];
	if (Array.isArray(headers)) return headers.map(([name, value]) => [String(name), String(value)]);
	/** @type {[string, string][]} */
	const pairs = [];
	const object = /** @type {any} */ (headers);
	if (typeof object.forEach === 'function') {
		object.forEach((/** @type {unknown} */ value, /** @type {unknown} */ name) => pairs.push([String(name), String(value)]));
	} else {
		for (const name of Object.keys(object)) pairs.push([name, String(object[name])]);
	}
	return pairs;
}

/**
 * @param {string | { url: string } | { href: string }} input
 * @param {{ method?: string, headers?: unknown, body?: string | ArrayBuffer | ArrayBufferView | null }} [init]
 * @returns {Promise<FetchResponse>}
 */
export async function fetch(input, init = {}) {
	const target = /** @type {any} */ (input);
	const url = typeof input === 'string' ? input : String(target.url ?? target.href ?? input);
	/** @type {Record<string, unknown>} */
	const request = { url, method: init.method ?? 'GET', headers: header_pairs(init.headers) };

	const body = init.body;
	if (typeof body === 'string') request.body = body;
	else if (body instanceof ArrayBuffer || ArrayBuffer.isView(body)) request.body_base64 = encode(to_bytes(body));
	else if (body != null) throw new TypeError('fetch: the body must be a string, an ArrayBuffer or a typed array');

	return new FetchResponse(await host_call('http.fetch', request));
}
