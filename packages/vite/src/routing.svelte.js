// File-based routing in the style of SvelteKit, held entirely in memory.
//
// There is no address bar, `location` or History API in the host, so the
// "URL" is a string this module owns, with its own back/forward stack. The
// route table comes from the Vite preset, which reads `src/routes`.

import renderer, { root } from '@ethercorps/svelte-blitz/renderer';

/**
 * A `+layout` or `+page`: its component and the `load` from the `.js`/`.ts`
 * file next to it. A layout may have only a `load`.
 * @typedef {{ component: import('svelte').Component<any> | null, load?: (event: LoadEvent) => any }} RouteNode
 */
/**
 * @typedef {{
 *   id: string,
 *   pattern: RegExp,
 *   names: string[],
 *   layouts: RouteNode[],
 *   page: RouteNode,
 *   error: import('svelte').Component<any> | null,
 *   error_depth: number
 * }} Route
 * `error` is the nearest `+error.svelte`; `error_depth` is how many of the
 * route's layouts stay on screen around it (those down to its own directory).
 */
/**
 * What a `load` function receives.
 * @typedef {{
 *   params: Record<string, string>,
 *   url: PageUrl,
 *   route: { id: string | null },
 *   parent: () => Promise<Record<string, any>>
 * }} LoadEvent
 */
/**
 * The current location. A plain object with the commonly used parts of a
 * `URL`; the engine has no `URL` class.
 * @typedef {{
 *   href: string,
 *   pathname: string,
 *   search: string,
 *   hash: string,
 *   searchParams: { get(name: string): string | null, has(name: string): boolean }
 * }} PageUrl
 */
/**
 * What is on screen: the layouts from the outside in, then the page (or an
 * error component in its place). `data[i]` is what layout `i` receives.
 * @typedef {{
 *   layouts: RouteNode[],
 *   data: Record<string, any>[],
 *   page: import('svelte').Component<any>,
 *   page_data: Record<string, any>
 * }} View
 */

class HttpError {
	/** @param {number} status @param {string} message */
	constructor(status, message) {
		this.status = status;
		this.message = message;
	}
}

class Redirect {
	/** @param {number} status @param {string} location */
	constructor(status, location) {
		this.status = status;
		this.location = location;
	}
}

/**
 * Stop a `load` and show the nearest `+error.svelte` with this status.
 * @param {number} status
 * @param {string} [message]
 * @returns {never}
 */
export function error(status, message = 'Error') {
	throw new HttpError(status, message);
}

/**
 * Stop a `load` and go somewhere else instead.
 * @param {number} status kept for parity with SvelteKit; it has no effect here
 * @param {string} location
 * @returns {never}
 */
export function redirect(status, location) {
	throw new Redirect(status, location);
}

/** @param {string} href an absolute path, optionally with `?search` and `#hash` @returns {PageUrl} */
function parse(href) {
	const hash_at = href.indexOf('#');
	const hash = hash_at === -1 ? '' : href.slice(hash_at);
	const rest = hash_at === -1 ? href : href.slice(0, hash_at);
	const search_at = rest.indexOf('?');
	const search = search_at === -1 ? '' : rest.slice(search_at);
	let pathname = search_at === -1 ? rest : rest.slice(0, search_at);
	// `/about/` and `/about` are the same page.
	if (pathname.length > 1 && pathname.endsWith('/')) pathname = pathname.slice(0, -1);

	/** @type {Map<string, string>} */
	const query = new Map();
	for (const pair of search.slice(1).split('&')) {
		if (pair === '') continue;
		const [name, value = ''] = pair.split('=');
		const decode = (/** @type {string} */ text) => decodeURIComponent(text.replace(/\+/g, ' '));
		if (!query.has(decode(name))) query.set(decode(name), decode(value));
	}

	return {
		href: pathname + search + hash,
		pathname,
		search,
		hash,
		searchParams: { get: (name) => query.get(name) ?? null, has: (name) => query.has(name) }
	};
}

/**
 * Turn an `href` into an absolute path, the way a browser would against the
 * current location.
 * @param {string} href
 */
function resolve(href) {
	if (href.startsWith('/')) return href;
	const here = page.url.pathname;
	if (href.startsWith('?') || href.startsWith('#')) return here + href;

	const segments = here.split('/').slice(1, -1);
	const tail_at = href.search(/[?#]/);
	const tail = tail_at === -1 ? '' : href.slice(tail_at);
	for (const segment of (tail_at === -1 ? href : href.slice(0, tail_at)).split('/')) {
		if (segment === '..') segments.pop();
		else if (segment !== '.' && segment !== '') segments.push(segment);
	}
	return '/' + segments.join('/') + tail;
}

/**
 * The page that is showing. Reactive; the same shape as `page` in SvelteKit's
 * `$app/state`, except that `url` is a plain object.
 * @type {{
 *   url: PageUrl,
 *   params: Record<string, string>,
 *   route: { id: string | null },
 *   data: Record<string, any>,
 *   status: number,
 *   error: { message: string } | null
 * }}
 */
export const page = $state({
	url: parse('/'),
	params: {},
	route: { id: null },
	data: {},
	status: 200,
	error: null
});

/** @type {View | null} */
let view = $state.raw(null);

/** Read by `Router.svelte`. */
export const router = {
	get view() {
		return view;
	}
};

/** @type {Route[]} */
let routes = [];
/** Used when nothing matches: the root layouts around the root `+error.svelte`. */
let fallback = /** @type {{ layouts: RouteNode[], error: import('svelte').Component<any> | null }} */ ({
	layouts: [],
	error: null
});
/** @type {import('svelte').Component<any>} */
let default_error;

/** Visited locations. `position` is where we are in it. @type {string[]} */
const stack = [];
let position = -1;
/** Bumped by every navigation, so one that was overtaken can tell and stop. */
let latest = 0;

/**
 * Run the loads for a set of nodes, outermost first. Each node's data is its
 * ancestors' data plus its own.
 *
 * Returns the data per node, or the index of the node whose load threw.
 * A redirect is rethrown.
 *
 * ponytail: every load runs on every navigation. SvelteKit reruns only the
 * ones whose inputs changed; track `params`/`url` reads if loads get slow.
 *
 * @param {RouteNode[]} nodes
 * @param {Omit<LoadEvent, 'parent'>} event
 */
async function run_loads(nodes, event) {
	/** @type {Record<string, any>[]} */
	const data = [];
	let merged = {};
	for (let index = 0; index < nodes.length; index++) {
		const above = merged;
		try {
			const loaded = await nodes[index].load?.({ ...event, parent: async () => above });
			merged = { ...above, ...(loaded ?? {}) };
		} catch (thrown) {
			if (thrown instanceof Redirect) throw thrown;
			return { data, failed: index, thrown };
		}
		data.push(merged);
	}
	return { data, failed: -1, thrown: null };
}

/**
 * Work out what to show for a location. Does not touch any state.
 * @param {PageUrl} url
 */
async function resolve_view(url) {
	/** @type {Route | undefined} */
	let route;
	/** @type {Record<string, string>} */
	const params = {};
	for (const candidate of routes) {
		const match = candidate.pattern.exec(url.pathname);
		if (match === null) continue;
		route = candidate;
		candidate.names.forEach((name, index) => (params[name] = decodeURIComponent(match[index + 1] ?? '')));
		break;
	}

	const event = { params, url, route: { id: route?.id ?? null } };
	const nodes = route ? [...route.layouts, route.page] : fallback.layouts;
	const { data, failed, thrown } = await run_loads(nodes, event);

	if (route && failed === -1) {
		return {
			params,
			route_id: route.id,
			status: 200,
			error: null,
			view: {
				layouts: route.layouts,
				data: data.slice(0, -1),
				page: /** @type {import('svelte').Component<any>} */ (route.page.component),
				page_data: data[data.length - 1]
			}
		};
	}

	// Not found, or a load threw. Keep the layouts that loaded, down to the
	// error page's own directory; the error page takes the place of the rest.
	//
	// ponytail: when the layout next to the +error.svelte is the one that
	// failed, SvelteKit moves up to the next +error.svelte. This keeps the
	// nearest one and renders it one layout further out.
	const kept = Math.min(
		failed === -1 ? nodes.length : failed,
		route ? route.error_depth : fallback.layouts.length
	);
	const reason = /** @type {any} */ (thrown);
	return {
		params,
		route_id: route?.id ?? null,
		status: route ? (reason instanceof HttpError ? reason.status : 500) : 404,
		error: { message: route ? String(reason?.message ?? reason) : `Not found: ${url.pathname}` },
		view: {
			layouts: nodes.slice(0, kept),
			data: data.slice(0, kept),
			page: (route ? route.error : fallback.error) ?? default_error,
			page_data: data[kept - 1] ?? {}
		}
	};
}

/**
 * Go to a location and, once its loads have finished, record it with `commit`.
 * @param {string} href absolute
 * @param {(href: string) => void} commit
 */
async function navigate(href, commit) {
	const mine = ++latest;
	let url = parse(href);
	let resolved;
	for (let hops = 0; ; hops++) {
		try {
			resolved = await resolve_view(url);
			break;
		} catch (thrown) {
			if (!(thrown instanceof Redirect) || hops === 10) throw thrown;
			url = parse(resolve(thrown.location));
		}
	}
	// A newer navigation started while this one was loading: it wins.
	if (mine !== latest) return;

	commit(url.href);
	view = resolved.view;
	page.url = url;
	page.params = resolved.params;
	page.route = { id: resolved.route_id };
	page.data = resolved.view.page_data;
	page.status = resolved.status;
	page.error = resolved.error;
}

/**
 * Navigate to another page, like SvelteKit's `goto`.
 * @param {string} href a path; relative ones resolve against the current page
 * @param {{ replaceState?: boolean }} [options] `replaceState` replaces the current history entry instead of adding one
 * @returns {Promise<void>} resolves when the new page is showing
 */
export function goto(href, { replaceState = false } = {}) {
	return navigate(resolve(href), (final) => {
		if (replaceState && position >= 0) {
			stack[position] = final;
		} else {
			stack.splice(position + 1);
			position = stack.push(final) - 1;
		}
	});
}

/**
 * Go back one page, like `history.back()` in a browser. Does nothing on the first page.
 * @returns {Promise<void>}
 */
export function back() {
	if (position <= 0) return Promise.resolve();
	return navigate(stack[position - 1], () => position--);
}

/**
 * Go forward again after `back()`.
 * @returns {Promise<void>}
 */
export function forward() {
	if (position >= stack.length - 1) return Promise.resolve();
	return navigate(stack[position + 1], () => position++);
}

/**
 * A click on `<a href="/somewhere">` navigates, as it does in SvelteKit.
 * Links with a scheme (`https:`, `mailto:`) are left alone.
 * @param {any} event
 */
function on_click(event) {
	if (event.defaultPrevented || event.button || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
		return;
	}
	for (let node = event.target; node; node = node.parentNode) {
		if (node.nodeName !== 'A') continue;
		const href = node.getAttribute('href');
		if (href === null || href.startsWith('#') || href.startsWith('//') || /^[a-z][a-z0-9+.-]*:/i.test(href)) return;
		event.preventDefault();
		goto(href);
		return;
	}
}

/**
 * Called once by the generated entry module.
 * @param {{ routes: Route[], fallback: typeof fallback }} manifest
 * @param {import('svelte').Component<any>} error_component shown when no `+error.svelte` applies
 */
export function start(manifest, error_component) {
	routes = manifest.routes;
	fallback = manifest.fallback;
	default_error = error_component;
	renderer.addEventListener(root, 'click', on_click);
	return goto('/');
}
