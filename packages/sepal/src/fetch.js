// `fetch`, and the Fetch API classes around it (`Request`, `Headers`,
// `Response`), installed as globals when the engine has none.
//
// The host makes the request, and only to URLs the app's `permissions.http`
// allows. This is the everyday part of the Fetch API: method, headers, a
// string or byte body, and `text()`, `json()`, `arrayBuffer()`, `bytes()` on
// the response. There is no streaming, no `AbortSignal` and no `FormData`.
// The classes are there because libraries test for them and build them:
// SvelteKit's client asks `input instanceof Request` on every fetch.

import { host_call } from './host-call.js';
import { decode, encode, to_bytes } from './base64.js';

/** Headers: names are case-insensitive and kept in lower case, as in the Fetch API. */
export class Headers {
	/** @type {Map<string, string>} */
	#map = new Map();

	/** @param {unknown} [init] a plain object, a list of pairs, or another `Headers` */
	constructor(init) {
		for (const [name, value] of header_pairs(init)) this.append(name, value);
	}
	/** @param {string} name @param {string} value */
	append(name, value) {
		const key = name.toLowerCase();
		const earlier = this.#map.get(key);
		this.#map.set(key, earlier === undefined ? String(value) : `${earlier}, ${value}`);
	}
	/** @param {string} name @param {string} value */
	set(name, value) {
		this.#map.set(name.toLowerCase(), String(value));
	}
	/** @param {string} name */
	get(name) {
		return this.#map.get(name.toLowerCase()) ?? null;
	}
	/** @param {string} name */
	has(name) {
		return this.#map.has(name.toLowerCase());
	}
	/** @param {string} name */
	delete(name) {
		this.#map.delete(name.toLowerCase());
	}
	/** @param {(value: string, name: string, headers: Headers) => void} callback */
	forEach(callback) {
		this.#map.forEach((value, name) => callback(value, name, this));
	}
	entries() {
		return this.#map.entries();
	}
	keys() {
		return this.#map.keys();
	}
	values() {
		return this.#map.values();
	}
	[Symbol.iterator]() {
		return this.#map.entries();
	}
}

const METHODS = ['DELETE', 'GET', 'HEAD', 'OPTIONS', 'POST', 'PUT', 'PATCH'];

/** A request to make: what `fetch` takes, as a string, a URL or one of these. */
export class Request {
	/**
	 * @param {string | Request | { url?: string, href?: string }} input
	 * @param {{ method?: string, headers?: unknown, body?: string | ArrayBuffer | ArrayBufferView | null }} [init]
	 */
	constructor(input, init = {}) {
		/** @type {Request | null} */
		const from = input instanceof Request ? input : null;
		const target = /** @type {any} */ (input);
		/** @type {string} */
		this.url = from ? from.url : typeof input === 'string' ? input : String(target?.url ?? target?.href ?? input);
		/** @type {string} */
		const method = init.method ?? from?.method ?? 'GET';
		// As the Fetch API does: the standard methods in upper case, any other as written.
		/** @type {string} */
		this.method = METHODS.includes(method.toUpperCase()) ? method.toUpperCase() : method;
		/** @type {Headers} */
		this.headers = new Headers(init.headers ?? from?.headers);
		/** @type {string | ArrayBuffer | ArrayBufferView | null} */
		this.body = init.body ?? from?.body ?? null;
	}
}

/** A response with its body in hand: what `new Response(body, init)` makes. `fetch` answers with a subclass. */
export class Response {
	/**
	 * @param {string | ArrayBuffer | ArrayBufferView | null} [body]
	 * @param {{ status?: number, statusText?: string, headers?: unknown }} [init]
	 */
	constructor(body = null, init = {}) {
		this.status = init.status ?? 200;
		this.statusText = init.statusText ?? '';
		this.ok = this.status >= 200 && this.status < 300;
		this.headers = new Headers(init.headers);
		this.url = '';
		this.redirected = false;
		this.bodyUsed = false;
		/** @type {string | Uint8Array | null} */
		this.held = body == null || typeof body === 'string' ? body : to_bytes(body);
	}

	/** @returns {string | Uint8Array} */
	take() {
		if (this.bodyUsed) throw new TypeError('the response body was already read');
		this.bodyUsed = true;
		return this.held ?? '';
	}

	/** @returns {Promise<string>} */
	async text() {
		const body = this.take();
		if (typeof body !== 'string') throw new TypeError('text() of a byte body is not supported; use bytes()');
		return body;
	}
	async json() {
		return JSON.parse(await this.text());
	}
	/** @returns {Promise<Uint8Array>} */
	async bytes() {
		const body = this.take();
		if (typeof body === 'string') throw new TypeError('bytes() of a string body is not supported; use text()');
		return body;
	}
	async arrayBuffer() {
		return /** @type {ArrayBuffer} */ ((await this.bytes()).buffer);
	}
}

/** What `fetch` answers with: the body stays with the host until it is asked for. */
class FetchResponse extends Response {
	/** @param {{ id: number, status: number, status_text: string, url: string, redirected: boolean, headers: [string, string][] }} head */
	constructor(head) {
		super(null, { status: head.status, statusText: head.status_text, headers: head.headers });
		this.url = head.url;
		this.redirected = head.redirected;
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
	async bytes() {
		return decode(await this.read('base64'));
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
 * @param {string | Request | { url?: string, href?: string }} input
 * @param {{ method?: string, headers?: unknown, body?: string | ArrayBuffer | ArrayBufferView | null }} [init]
 * @returns {Promise<Response>}
 */
export async function fetch(input, init = {}) {
	const { url, method, headers, body } = new Request(input, init);
	// A relative URL is relative to the page the app is on (the DOM layer's
	// `location`), or to the app's origin.
	/** @type {Record<string, unknown>} */
	const request = { url, method, headers: [...headers], base: /** @type {any} */ (globalThis).location?.href };

	if (typeof body === 'string') request.body = body;
	else if (body instanceof ArrayBuffer || ArrayBuffer.isView(body)) request.body_base64 = encode(to_bytes(body));
	else if (body != null) throw new TypeError('fetch: the body must be a string, an ArrayBuffer or a typed array');

	return new FetchResponse(await host_call('http.fetch', request));
}
