// `URL` and `URLSearchParams` for an engine that has neither (QuickJS).
//
// Covers what applications and routers do with URLs: parse an absolute one,
// resolve a relative one against a base, read and write the parts, and work
// with the query. It does not cover what a browser needs to fetch arbitrary
// URLs from the wild: no IDNA, no IPv6 literals, no percent-encoding fix-ups
// of what was already there. A URL without `//` (`mailto:a@b`, `a:`) keeps what
// follows the scheme as its path, untouched. `test/url.test.js` compares it with Node's own.

import { lru } from './lru.js';

const ABSOLUTE = /^([a-zA-Z][a-zA-Z0-9+.-]*):\/\/([^/?#]*)([^?#]*)(\?[^#]*)?(#.*)?$/;
/** Schemes whose URLs always have a host and a path, and an origin to speak of. */
const SPECIAL = /^(https?|wss?|ftp):$/;
const DEFAULT_PORTS = /** @type {Record<string, string>} */ ({ 'http:': '80', 'https:': '443', 'ws:': '80', 'wss:': '443', 'ftp:': '21' });

/** Take `.` and `..` segments out of a path. @param {string} path */
function normalize_path(path) {
	/** @type {string[]} */
	const out = [];
	const segments = path.split('/');
	for (let i = 1; i < segments.length; i++) {
		const segment = segments[i];
		const last = i === segments.length - 1;
		if (segment === '..') {
			out.pop();
			if (last) out.push('');
		} else if (segment === '.') {
			if (last) out.push('');
		} else {
			out.push(segment);
		}
	}
	return '/' + out.join('/');
}

/** What a URL keeps percent-encoded in a path, a query or a hash. @param {string} text @param {RegExp} unsafe */
const encode = (text, unsafe) => text.replace(unsafe, (c) => encodeURIComponent(c));
const PATH_UNSAFE = /[\s"<>`{}]|[^\x00-\x7f]/g;
const QUERY_UNSAFE = /[\s"<>']|[^\x00-\x7f]/g;
const HASH_UNSAFE = /[\s"<>`]|[^\x00-\x7f]/g;

/**
 * @param {string} input
 * @param {URL | undefined} base
 * @returns {{ protocol: string, username: string, password: string, hostname: string, port: string, pathname: string, search: string, hash: string, opaque?: boolean } | null}
 */
function parse(input, base) {
	input = input.trim();
	const absolute = ABSOLUTE.exec(input);
	if (absolute !== null) {
		const protocol = absolute[1].toLowerCase() + ':';
		let authority = absolute[2];
		let username = '';
		let password = '';
		const at = authority.lastIndexOf('@');
		if (at !== -1) {
			[username, password = ''] = authority.slice(0, at).split(':');
			authority = authority.slice(at + 1);
		}
		const colon = authority.lastIndexOf(':');
		let hostname = colon === -1 ? authority : authority.slice(0, colon);
		let port = colon === -1 ? '' : authority.slice(colon + 1);
		if (port !== '' && !/^\d+$/.test(port)) return null;
		// Other schemes (`a://`, which SvelteKit resolves paths against) may
		// have no host, and no path either.
		const special = SPECIAL.test(protocol);
		if (hostname === '' && special) return null;
		if (port === DEFAULT_PORTS[protocol]) port = '';
		else if (port !== '') port = String(Number(port));
		hostname = hostname.toLowerCase();
		return {
			protocol,
			username,
			password,
			hostname,
			port,
			pathname: absolute[3] === '' && !special ? '' : encode(normalize_path(absolute[3] || '/'), PATH_UNSAFE),
			search: clean(absolute[4], QUERY_UNSAFE),
			hash: clean(absolute[5], HASH_UNSAFE)
		};
	}

	// `scheme:whatever`, with no authority: the rest is the path, as written.
	const opaque = /^([a-zA-Z][a-zA-Z0-9+.-]*):(?!\/\/)([^?#]*)(\?[^#]*)?(#.*)?$/.exec(input);
	if (opaque !== null) {
		const protocol = opaque[1].toLowerCase() + ':';
		if (SPECIAL.test(protocol)) return null;
		return { protocol, username: '', password: '', hostname: '', port: '', pathname: opaque[2], search: clean(opaque[3], QUERY_UNSAFE), hash: clean(opaque[4], HASH_UNSAFE), opaque: true };
	}
	if (base === undefined) return null;
	if (no_authority(base)) {
		// Nothing resolves against such a URL except a hash.
		if (!input.startsWith('#')) return null;
		return { protocol: base.protocol, username: '', password: '', hostname: '', port: '', pathname: base.pathname, search: raw_search(base), hash: clean(input, HASH_UNSAFE), opaque: true };
	}
	// Relative to the base: `//host/path`, `/path`, `path`, `?query`, `#hash`, or nothing.
	if (input.startsWith('//')) return parse(base.protocol + input, undefined);
	const relative = /^([^?#]*)(\?[^#]*)?(#.*)?$/.exec(input);
	if (relative === null) return null;
	const [, path, search, hash] = relative;
	const parts = { protocol: base.protocol, username: base.username, password: base.password, hostname: base.hostname, port: base.port };
	if (path === '') {
		return {
			...parts,
			pathname: base.pathname,
			search: search === undefined ? raw_search(base) : clean(search, QUERY_UNSAFE),
			hash: clean(hash, HASH_UNSAFE)
		};
	}
	// A base without a path (`a://`) resolves as if it had `/`.
	const directory = base.pathname.slice(0, base.pathname.lastIndexOf('/') + 1) || '/';
	return {
		...parts,
		pathname: encode(normalize_path(path.startsWith('/') ? path : directory + path), PATH_UNSAFE),
		search: clean(search, QUERY_UNSAFE),
		hash: clean(hash, HASH_UNSAFE)
	};
}

/**
 * A query or hash as it was written, encoded. A lone `?` or `#` is kept: it
 * stays in `href`, though `search` and `hash` read as empty.
 * @param {string | undefined} part @param {RegExp} unsafe
 */
function clean(part, unsafe) {
	return part === undefined ? '' : encode(part, unsafe);
}

/**
 * What `parse` made of an input and a base, by both. Routers and Vite's
 * preload helper parse the same few URLs on every navigation; each `new URL`
 * still gets its own object, copied from the cached parts, which nothing
 * writes to. A base `URL` is keyed by its `href` (and whether it has an
 * authority), which is everything `parse` reads from it. SvelteKit's look
 * at a page's links after a navigation parses two per link (the Tracker's
 * Board has 316): the limit leaves room for a few such pages.
 * @type {ReturnType<typeof lru<string, ReturnType<typeof parse>>>}
 */
const parsed = lru('urls', 2048);

/** @param {string} input @param {string | URL | undefined} base */
function parse_cached(input, base) {
	const base_key = base === undefined ? '' : base instanceof URL ? (no_authority(base) ? 'o' : 'u') + base.href : 's' + String(base);
	// The base's length first, so that no input can make two pairs one key.
	const key = `${base_key.length}:${base_key}${input}`;
	let parts = parsed.get(key);
	if (parts === undefined) {
		const against = base === undefined ? undefined : base instanceof URL ? base : new URL(String(base));
		parts = parse(input, against);
		parsed.set(key, parts);
	}
	return parts;
}

/** The query of a URL as written, lone `?` included, for resolving against it. Set by the class. @type {(url: URL) => string} */
let raw_search;
/** Whether a URL is of the `scheme:path` kind. Set by the class. @type {(url: URL) => boolean} */
let no_authority;

export class URL {
	/** @param {string | URL} input @param {string | URL} [base] */
	constructor(input, base) {
		const parts = parse_cached(String(input), base);
		if (parts === null) throw new TypeError(`Invalid URL: ${String(input)}`);
		this.protocol = parts.protocol;
		this.username = parts.username;
		this.password = parts.password;
		this.hostname = parts.hostname;
		this.port = parts.port;
		this.pathname = parts.pathname;
		this.#search = parts.search;
		this.#hash = parts.hash;
		this.#opaque = parts.opaque === true;
	}

	// With their `?` and `#`, which may be all there is: what a URL resolved
	// against this one keeps, lone `?` included.
	#search = '';
	#hash = '';
	/** A `scheme:path` URL, with no host to print or to resolve against. */
	#opaque = false;
	/**
	 * Made when it is first asked for. Routers and Vite's preload helper
	 * make URLs for their parts, many a navigation (SvelteKit: one per link
	 * on the page), and a `URLSearchParams` each was a third of that.
	 * @type {URLSearchParams | null}
	 */
	#params = null;

	static {
		raw_search = (url) => url.#search;
		no_authority = (url) => url.#opaque;
	}

	get searchParams() {
		if (this.#params === null) {
			this.#params = new URLSearchParams(this.#search);
			link.set(this.#params, (/** @type {string} */ query) => this.#set_search(query === '' ? '' : '?' + query));
		}
		return this.#params;
	}

	/** @param {string} search */
	#set_search(search) {
		this.#search = search;
	}

	get search() {
		return this.#search === '?' ? '' : this.#search;
	}
	set search(value) {
		value = String(value);
		this.#set_search(value === '' ? '' : clean(value.startsWith('?') ? value : '?' + value, QUERY_UNSAFE));
		if (this.#params !== null) reset.get(this.#params)?.(this.#search);
	}
	get hash() {
		return this.#hash === '#' ? '' : this.#hash;
	}
	set hash(value) {
		value = String(value);
		this.#hash = value === '' ? '' : clean(value.startsWith('#') ? value : '#' + value, HASH_UNSAFE);
	}
	get host() {
		return this.port === '' ? this.hostname : `${this.hostname}:${this.port}`;
	}
	set host(value) {
		[this.hostname, this.port = ''] = String(value).split(':');
	}
	get origin() {
		return SPECIAL.test(this.protocol) ? `${this.protocol}//${this.host}` : 'null';
	}
	get href() {
		if (this.#opaque) return `${this.protocol}${this.pathname}${this.#search}${this.#hash}`;
		const user = this.username === '' ? '' : this.username + (this.password === '' ? '' : ':' + this.password) + '@';
		return `${this.protocol}//${user}${this.host}${this.pathname}${this.#search}${this.#hash}`;
	}
	set href(value) {
		const parts = parse(String(value), undefined);
		if (parts === null) throw new TypeError(`Invalid URL: ${String(value)}`);
		const { search, hash, opaque, ...rest } = parts;
		this.#opaque = opaque === true;
		Object.assign(this, rest);
		this.search = search;
		this.#hash = hash;
	}
	toString() {
		return this.href;
	}
	toJSON() {
		return this.href;
	}

	/** @param {string | URL} input @param {string | URL} [base] */
	static canParse(input, base) {
		try {
			new URL(input, base);
			return true;
		} catch {
			return false;
		}
	}
	/** @param {string | URL} input @param {string | URL} [base] */
	static parse(input, base) {
		try {
			return new URL(input, base);
		} catch {
			return null;
		}
	}
}


/** How a `searchParams` tells its URL that it changed, and the other way round. */
/** @type {WeakMap<URLSearchParams, (query: string) => void>} */
const link = new WeakMap();
/** @type {WeakMap<URLSearchParams, (search: string) => void>} */
const reset = new WeakMap();

/** Form encoding: a space is `+`. @param {string} text */
const form_encode = (text) => encodeURIComponent(text).replace(/%20/g, '+').replace(/[!'()~]/g, (c) => '%' + c.charCodeAt(0).toString(16).toUpperCase());
/** @param {string} text */
function form_decode(text) {
	text = text.replace(/\+/g, ' ');
	try {
		return decodeURIComponent(text);
	} catch {
		return text;
	}
}

export class URLSearchParams {
	/** @type {[string, string][]} */
	#pairs = [];

	/** @param {string | Record<string, string> | Iterable<[string, string]> | URLSearchParams} [init] */
	constructor(init = '') {
		reset.set(this, (search) => this.#read(search));
		if (typeof init === 'string') this.#read(init);
		else if (init instanceof URLSearchParams) this.#pairs = init.#pairs.map(([k, v]) => [k, v]);
		else if (typeof (/** @type {any} */ (init)[Symbol.iterator]) === 'function') {
			for (const [key, value] of /** @type {Iterable<[string, string]>} */ (init)) this.#pairs.push([String(key), String(value)]);
		} else {
			for (const key of Object.keys(init)) this.#pairs.push([key, String(/** @type {Record<string, string>} */ (init)[key])]);
		}
	}

	/** @param {string} search */
	#read(search) {
		this.#pairs = [];
		if (search.startsWith('?')) search = search.slice(1);
		if (search === '') return;
		for (const part of search.split('&')) {
			if (part === '') continue;
			const equals = part.indexOf('=');
			const key = equals === -1 ? part : part.slice(0, equals);
			const value = equals === -1 ? '' : part.slice(equals + 1);
			this.#pairs.push([form_decode(key), form_decode(value)]);
		}
	}
	#changed() {
		link.get(this)?.(this.toString());
	}

	get size() {
		return this.#pairs.length;
	}
	/** @param {string} name @param {string} value */
	append(name, value) {
		this.#pairs.push([String(name), String(value)]);
		this.#changed();
	}
	/** @param {string} name @param {string} [value] */
	delete(name, value) {
		this.#pairs = this.#pairs.filter(([k, v]) => !(k === name && (value === undefined || v === String(value))));
		this.#changed();
	}
	/** @param {string} name */
	get(name) {
		return this.#pairs.find(([k]) => k === name)?.[1] ?? null;
	}
	/** @param {string} name */
	getAll(name) {
		return this.#pairs.filter(([k]) => k === name).map(([, v]) => v);
	}
	/** @param {string} name @param {string} [value] */
	has(name, value) {
		return this.#pairs.some(([k, v]) => k === name && (value === undefined || v === String(value)));
	}
	/** @param {string} name @param {string} value */
	set(name, value) {
		const first = this.#pairs.findIndex(([k]) => k === name);
		if (first === -1) this.#pairs.push([String(name), String(value)]);
		else {
			this.#pairs[first][1] = String(value);
			this.#pairs = this.#pairs.filter(([k], index) => k !== name || index === first);
		}
		this.#changed();
	}
	sort() {
		// Stable, by key only: values keep their order.
		this.#pairs = this.#pairs.map((pair, index) => ({ pair, index })).sort((a, b) => (a.pair[0] < b.pair[0] ? -1 : a.pair[0] > b.pair[0] ? 1 : a.index - b.index)).map(({ pair }) => pair);
		this.#changed();
	}
	/** @param {(value: string, key: string, params: URLSearchParams) => void} callback @param {any} [self] */
	forEach(callback, self) {
		for (const [key, value] of this.#pairs) callback.call(self, value, key, this);
	}
	*entries() {
		for (const [key, value] of this.#pairs) yield /** @type {[string, string]} */ ([key, value]);
	}
	*keys() {
		for (const [key] of this.#pairs) yield key;
	}
	*values() {
		for (const [, value] of this.#pairs) yield value;
	}
	[Symbol.iterator]() {
		return this.entries();
	}
	toString() {
		return this.#pairs.map(([k, v]) => `${form_encode(k)}=${form_encode(v)}`).join('&');
	}
}
