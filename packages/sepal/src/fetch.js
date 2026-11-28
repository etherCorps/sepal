// `fetch`, and the Fetch API classes around it (`Request`, `Headers`,
// `Response`), installed as globals by `web.js` when the engine has none.
//
// The host makes the request, and only to URLs the app's `permissions.http`
// allows. `fetch` resolves once the status and headers are in, and the body
// stays with the host until it is read: all at once (`text()`, `bytes()`,
// `json()`, ...) or a chunk at a time as it arrives (`response.body`, a
// `ReadableStream`). A body can be a string, bytes, a `Blob`, `FormData`,
// `URLSearchParams` or a `ReadableStream`; `AbortSignal` stops a request and
// its body. SvelteKit's client asks `input instanceof Request` on every
// fetch, and its server builds and reads these classes.

import { Blob, FormData } from 'formdata-node';
import { ReadableStream } from 'web-streams-polyfill';
import { AbortController, abort_error } from './abort.js';
import { decode, encode, to_bytes } from './base64.js';
import { concat, encode_multipart, make_boundary, parse_form } from './form-data.js';
import { host_call } from './host-call.js';
import { TextDecoder, TextEncoder } from './text.js';
import { URLSearchParams } from './url.js';

/** @typedef {import('./abort.js').AbortSignal} AbortSignal */

const encoder = new TextEncoder();

/** Headers: names are case-insensitive and kept in lower case, as in the Fetch API. */
export class Headers {
	/** @type {Map<string, string>} */
	#map = new Map();
	/** `Set-Cookie` lines, which are never joined: one cookie each. @type {string[]} */
	#cookies = [];

	/** @param {unknown} [init] a plain object, a list of pairs, or another `Headers` */
	constructor(init) {
		for (const [name, value] of header_pairs(init)) this.append(name, value);
	}
	/** @param {string} name @param {string} value */
	append(name, value) {
		const key = name.toLowerCase();
		if (key === 'set-cookie') {
			this.#cookies.push(String(value));
			return;
		}
		const earlier = this.#map.get(key);
		this.#map.set(key, earlier === undefined ? String(value) : `${earlier}, ${value}`);
	}
	/** @param {string} name @param {string} value */
	set(name, value) {
		const key = name.toLowerCase();
		if (key === 'set-cookie') this.#cookies = [String(value)];
		else this.#map.set(key, String(value));
	}
	/** @param {string} name */
	get(name) {
		const key = name.toLowerCase();
		if (key === 'set-cookie') return this.#cookies.length > 0 ? this.#cookies.join(', ') : null;
		return this.#map.get(key) ?? null;
	}
	/** Each `Set-Cookie` line, as the Fetch API keeps them. */
	getSetCookie() {
		return [...this.#cookies];
	}
	/** @param {string} name */
	has(name) {
		const key = name.toLowerCase();
		return key === 'set-cookie' ? this.#cookies.length > 0 : this.#map.has(key);
	}
	/** @param {string} name */
	delete(name) {
		const key = name.toLowerCase();
		if (key === 'set-cookie') this.#cookies = [];
		else this.#map.delete(key);
	}
	/** @param {(value: string, name: string, headers: Headers) => void} callback */
	forEach(callback) {
		for (const [name, value] of this) callback(value, name, this);
	}
	/** In name order, with each `Set-Cookie` line on its own, as the Fetch API iterates. */
	*entries() {
		/** @type {[string, string][]} */
		const pairs = [...this.#map];
		for (const cookie of this.#cookies) pairs.push(['set-cookie', cookie]);
		pairs.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
		yield* pairs;
	}
	*keys() {
		for (const [name] of this.entries()) yield name;
	}
	*values() {
		for (const [, value] of this.entries()) yield value;
	}
	[Symbol.iterator]() {
		return this.entries();
	}
}

/**
 * What a body was made from, and how it is read.
 * @typedef {{ text: string } | { bytes: Uint8Array } | { blob: Blob } | { form: FormData, boundary: string } | { stream: ReadableStream<Uint8Array> } | { host: number }} Source
 */

/**
 * A body as given to `new Request` or `new Response`, and the content type it
 * implies.
 * @param {unknown} body
 * @returns {{ source: Source | null, type: string | null }}
 */
function extract(body) {
	if (body == null) return { source: null, type: null };
	if (typeof body === 'string') return { source: { text: body }, type: 'text/plain;charset=UTF-8' };
	if (body instanceof URLSearchParams) {
		return { source: { text: body.toString() }, type: 'application/x-www-form-urlencoded;charset=UTF-8' };
	}
	if (body instanceof FormData) {
		const boundary = make_boundary();
		return { source: { form: body, boundary }, type: `multipart/form-data; boundary=${boundary}` };
	}
	if (body instanceof Blob) return { source: { blob: body }, type: body.type || null };
	if (body instanceof ArrayBuffer || ArrayBuffer.isView(body)) return { source: { bytes: to_bytes(body).slice() }, type: null };
	if (body instanceof ReadableStream) return { source: { stream: body }, type: null };
	return { source: { text: String(body) }, type: 'text/plain;charset=UTF-8' };
}

/** @param {ReadableStream<Uint8Array>} stream */
async function read_stream(stream) {
	const reader = stream.getReader();
	/** @type {Uint8Array[]} */
	const chunks = [];
	for (;;) {
		const { done, value } = await reader.read();
		if (done) break;
		chunks.push(typeof value === 'string' ? encoder.encode(value) : to_bytes(value));
	}
	return concat(chunks);
}

/** A body source, all of it, as bytes. @param {Source} source @returns {Promise<Uint8Array>} */
async function source_bytes(source) {
	if ('text' in source) return encoder.encode(source.text);
	if ('bytes' in source) return source.bytes;
	if ('blob' in source) return new Uint8Array(await source.blob.arrayBuffer());
	if ('form' in source) return encode_multipart(source.form, source.boundary);
	if ('stream' in source) return read_stream(source.stream);
	return decode(await host_call('http.body', { id: source.host, as: 'base64' }));
}

/** A body source as a stream. @param {Source} source @param {AbortSignal | null} [signal] @returns {ReadableStream<Uint8Array>} */
function source_stream(source, signal) {
	if ('stream' in source) return source.stream;
	if ('host' in source) return host_stream(source.host, signal ?? null);
	return new ReadableStream({
		async start(controller) {
			const bytes = await source_bytes(source);
			if (bytes.length > 0) controller.enqueue(bytes);
			controller.close();
		}
	});
}

/** A response body the host still has, read a chunk at a time as it arrives. @param {number} id @param {AbortSignal | null} signal */
function host_stream(id, signal) {
	return new ReadableStream({
		/** @param {import('web-streams-polyfill').ReadableStreamDefaultController<Uint8Array>} controller */
		async pull(controller) {
			if (signal?.aborted) {
				controller.error(signal.reason);
				return;
			}
			const { chunk, done } = await host_call('http.read', { id });
			if (signal?.aborted) controller.error(signal.reason);
			else if (done) controller.close();
			else controller.enqueue(decode(chunk));
		}
	});
}

/** What `Request` and `Response` share: a body, read once. */
class Body {
	/** @type {Source | null} */
	#source;
	/** @type {ReadableStream<Uint8Array> | null} */
	#stream = null;
	#used = false;

	/** @param {Source | null} source */
	constructor(source) {
		this.#source = source;
	}

	/** What this body belongs to, for messages. */
	get kind() {
		return 'response';
	}

	/** For `clone()`: the source, turned into a stream that can be split. @returns {Source | null} */
	split_source() {
		if (this.#source === null) return null;
		if (this.#used) throw new TypeError(`the ${this.kind} body was already read`);
		const [mine, theirs] = this.body_stream().tee();
		this.#stream = mine;
		this.#source = { stream: mine };
		return { stream: theirs };
	}

	/** @returns {AbortSignal | null} */
	body_signal() {
		return null;
	}

	/** @returns {ReadableStream<Uint8Array>} */
	body_stream() {
		if (this.#stream === null) this.#stream = source_stream(/** @type {Source} */ (this.#source), this.body_signal());
		return this.#stream;
	}

	/** Whether there is a body, without making its stream. */
	has_body() {
		return this.#source !== null;
	}

	/** The body as a stream, or `null` for none. */
	get body() {
		if (this.#source === null) return null;
		return this.body_stream();
	}

	/** Read, or its stream taken to be read. */
	get bodyUsed() {
		return this.#used || (this.#stream?.locked ?? false);
	}

	/** The source, once: reading a body uses it up. @returns {Source | null} */
	take() {
		if (this.#used) throw new TypeError(`the ${this.kind} body was already read`);
		this.#used = true;
		if (this.#stream !== null) return { stream: this.#stream };
		return this.#source;
	}

	/** @returns {string | null} */
	content_type() {
		return null;
	}

	/** The body for the host: text as it is, anything else as base64. @returns {Promise<{ body: string } | { body_base64: string }>} */
	async for_host() {
		const source = this.take();
		if (source !== null && 'text' in source) return { body: source.text };
		return { body_base64: encode(source === null ? new Uint8Array(0) : await source_bytes(source)) };
	}

	/** @returns {Promise<Uint8Array>} */
	async bytes() {
		const source = this.take();
		return source === null ? new Uint8Array(0) : source_bytes(source);
	}
	async arrayBuffer() {
		const bytes = await this.bytes();
		return /** @type {ArrayBuffer} */ (bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength));
	}
	/** @returns {Promise<string>} */
	async text() {
		const source = this.take();
		if (source === null) return '';
		if ('text' in source) return source.text;
		if ('host' in source) return host_call('http.body', { id: source.host, as: 'text' });
		return new TextDecoder().decode(await source_bytes(source));
	}
	async json() {
		return JSON.parse(await this.text());
	}
	async blob() {
		return new Blob([await this.bytes()], { type: this.content_type() ?? '' });
	}
	async formData() {
		const source = this.take();
		if (source !== null && 'form' in source) return source.form;
		return parse_form(source === null ? new Uint8Array(0) : await source_bytes(source), this.content_type());
	}
}

const METHODS = ['DELETE', 'GET', 'HEAD', 'OPTIONS', 'POST', 'PUT', 'PATCH'];

/** A request to make: what `fetch` takes, as a string, a URL or one of these. */
export class Request extends Body {
	/**
	 * @param {string | Request | { url?: string, href?: string }} input
	 * @param {{ method?: string, headers?: unknown, body?: unknown, signal?: AbortSignal | null, credentials?: string, mode?: string, cache?: string, redirect?: string, referrer?: string, integrity?: string, keepalive?: boolean, duplex?: string }} [init]
	 */
	constructor(input, init = {}) {
		/** @type {Request | null} */
		const from = input instanceof Request ? input : null;
		const given = init.body !== undefined ? extract(init.body) : null;
		super(given ? given.source : from ? from.split_source() : null);
		const target = /** @type {any} */ (input);
		/** @type {string} */
		this.url = from ? from.url : typeof input === 'string' ? input : String(target?.url ?? target?.href ?? input);
		const method = init.method ?? from?.method ?? 'GET';
		// As the Fetch API does: the standard methods in upper case, any other as written.
		/** @type {string} */
		this.method = METHODS.includes(method.toUpperCase()) ? method.toUpperCase() : method;
		/** @type {Headers} */
		this.headers = new Headers(init.headers ?? from?.headers);
		if (given?.type && !this.headers.has('content-type')) this.headers.set('content-type', given.type);
		/** @type {AbortSignal} */
		this.signal = init.signal ?? from?.signal ?? new AbortController().signal;
		/** @type {string} */
		this.credentials = init.credentials ?? from?.credentials ?? 'same-origin';
		/** @type {string} */
		this.mode = init.mode ?? from?.mode ?? 'cors';
		/** @type {string} */
		this.cache = init.cache ?? from?.cache ?? 'default';
		/** @type {string} */
		this.redirect = init.redirect ?? from?.redirect ?? 'follow';
		/** @type {string} */
		this.referrer = init.referrer ?? from?.referrer ?? 'about:client';
		this.referrerPolicy = '';
		/** @type {string} */
		this.integrity = init.integrity ?? from?.integrity ?? '';
		/** @type {boolean} */
		this.keepalive = init.keepalive ?? from?.keepalive ?? false;
		this.destination = '';
		this.duplex = init.duplex ?? 'half';
	}
	content_type() {
		return this.headers.get('content-type');
	}
	get kind() {
		return 'request';
	}
	clone() {
		return new Request(this);
	}
}

/** A response with its body: what `new Response(body, init)` makes. `fetch` answers with a subclass. */
export class Response extends Body {
	/**
	 * @param {unknown} [body]
	 * @param {{ status?: number, statusText?: string, headers?: unknown }} [init]
	 * @param {Source | null} [source] for `clone()` and `fetch`: the body as it is already held
	 */
	constructor(body = null, init = {}, source) {
		const given = source === undefined ? extract(body) : { source, type: null };
		super(given.source);
		this.status = init.status ?? 200;
		if (this.status !== 0 && (this.status < 200 || this.status > 599)) {
			throw new RangeError(`Response: ${this.status} is not a status between 200 and 599`);
		}
		this.statusText = init.statusText ?? '';
		this.ok = this.status >= 200 && this.status < 300;
		this.headers = new Headers(init.headers);
		if (given.type && !this.headers.has('content-type')) this.headers.set('content-type', given.type);
		this.url = '';
		this.redirected = false;
		/** @type {'default' | 'basic' | 'error'} */
		this.type = 'default';
	}
	content_type() {
		return this.headers.get('content-type');
	}
	clone() {
		const copy = new Response(null, { status: this.status, statusText: this.statusText, headers: this.headers }, this.split_source());
		copy.url = this.url;
		copy.redirected = this.redirected;
		copy.type = this.type;
		return copy;
	}
	/** @param {unknown} data @param {{ status?: number, statusText?: string, headers?: unknown }} [init] */
	static json(data, init = {}) {
		const headers = new Headers(init.headers);
		if (!headers.has('content-type')) headers.set('content-type', 'application/json');
		return new Response(JSON.stringify(data), { ...init, headers });
	}
	/** @param {string | { href: string }} url @param {number} [status] */
	static redirect(url, status = 302) {
		if (![301, 302, 303, 307, 308].includes(status)) throw new RangeError(`Response.redirect: ${status} is not a redirect status`);
		return new Response(null, { status, headers: { location: typeof url === 'string' ? url : url.href } });
	}
	static error() {
		const response = new Response(null, { status: 0 });
		response.type = 'error';
		return response;
	}
}

/** What `fetch` answers with: the body stays with the host until it is read. */
class FetchResponse extends Response {
	/** @type {AbortSignal | null} */
	#signal;

	/** @param {{ id: number, status: number, status_text: string, url: string, redirected: boolean, headers: [string, string][] }} head @param {AbortSignal | null} signal */
	constructor(head, signal) {
		super(null, { status: head.status, statusText: head.status_text, headers: head.headers }, { host: head.id });
		this.url = head.url;
		this.redirected = head.redirected;
		this.type = /** @type {'basic'} */ ('basic');
		this.#signal = signal;
	}
	body_signal() {
		return this.#signal;
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
	if (object instanceof Headers) {
		for (const pair of object) pairs.push(pair);
	} else if (typeof object.forEach === 'function') {
		object.forEach((/** @type {unknown} */ value, /** @type {unknown} */ name) => pairs.push([String(name), String(value)]));
	} else {
		for (const name of Object.keys(object)) pairs.push([name, String(object[name])]);
	}
	return pairs;
}

/**
 * @param {string | Request | { url?: string, href?: string }} input
 * @param {ConstructorParameters<typeof Request>[1]} [init]
 * @returns {Promise<Response>}
 */
export async function fetch(input, init = {}) {
	// A request on its own is sent as it is: its body is used up, as the Fetch API says.
	const request = input instanceof Request && Object.keys(init).length === 0 ? input : new Request(input, init);
	const signal = request.signal;
	signal.throwIfAborted();
	// A relative URL is relative to the page the app is on (the DOM layer's
	// `location`), or to the app's origin.
	/** @type {Record<string, unknown>} */
	const call = { url: request.url, method: request.method, headers: [...request.headers], base: /** @type {any} */ (globalThis).location?.href };
	if (request.has_body()) Object.assign(call, await request.for_host());
	/** @type {Promise<never>} */
	const aborted = new Promise((_, reject) => {
		if (signal.aborted) reject(signal.reason);
		signal.addEventListener('abort', () => reject(signal.reason ?? abort_error('the request was aborted')), { once: true });
	});
	aborted.catch(() => {});
	const head = await Promise.race([host_call('http.fetch', call), aborted]);
	return new FetchResponse(head, signal);
}
