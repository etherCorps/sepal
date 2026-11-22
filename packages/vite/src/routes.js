// Reads `src/routes` and turns it into the route table the router runs on.
// Runs in Node, inside the Vite preset.
//
// The conventions are SvelteKit's:
//
//   +page.svelte     a page                +page.js / +page.ts       its `load`
//   +layout.svelte   wraps what is below   +layout.js / +layout.ts   its `load`
//   +error.svelte    shown for errors at or below this directory
//   [name]           a parameter           [...name]   the rest of the path
//   (name)           a group: layouts apply, the path does not change
//
// Not supported, and reported as build errors rather than ignored: anything
// that needs a server (`+page.server`, `+layout.server`, `+server`), optional
// parameters (`[[name]]`) and matchers (`[name=matcher]`).

import { readdirSync } from 'node:fs';

/**
 * @typedef {{ component: string | null, module: string | null }} NodeFiles
 * @typedef {{
 *   id: string,
 *   segments: string[],
 *   layouts: NodeFiles[],
 *   page: NodeFiles,
 *   error: string | null,
 *   error_depth: number
 * }} ScannedRoute
 */

/** @param {string} message @returns {never} */
function fail(message) {
	throw new Error(`svelte-blitz routes: ${message}`);
}

/**
 * @param {string} dir absolute path of `src/routes`
 * @returns {{ routes: ScannedRoute[], fallback: { layouts: NodeFiles[], error: string | null }, files: string[] }}
 */
export function scan_routes(dir) {
	/** @type {ScannedRoute[]} */
	const routes = [];
	/** @type {string[]} */
	const files = [];
	let fallback = /** @type {{ layouts: NodeFiles[], error: string | null }} */ ({ layouts: [], error: null });

	/**
	 * @param {string} current directory being read
	 * @param {string[]} id_parts every directory name from the root, groups included
	 * @param {NodeFiles[]} layouts layouts that apply here, outermost first
	 * @param {string | null} error nearest `+error.svelte`
	 * @param {number} error_depth how many of `layouts` stay on screen around that error page
	 */
	function walk(current, id_parts, layouts, error, error_depth) {
		const entries = readdirSync(current, { withFileTypes: true }).sort((a, b) => (a.name < b.name ? -1 : 1));
		/** @type {Record<string, string>} */
		const found = {};

		for (const entry of entries) {
			if (entry.isDirectory() || !entry.name.startsWith('+')) continue;
			const path = `${current}/${entry.name}`;
			const match = /^\+(page|layout|error)\.(svelte|js|ts)$/.exec(entry.name);
			if (/^\+(page|layout)\.server\.|^\+server\./.test(entry.name)) {
				fail(`${path} needs a server, and there is none: the app runs entirely in the window. Use +page.js/+page.ts instead.`);
			}
			if (match === null || (match[1] === 'error' && match[2] !== 'svelte')) {
				fail(`${path} is not a route file this renderer knows.`);
			}
			const kind = match[2] === 'svelte' ? `${match[1]}.svelte` : `${match[1]}.module`;
			if (found[kind]) fail(`${path} and ${found[kind]} are the same route file twice.`);
			found[kind] = path;
			files.push(path);
		}

		if (found['layout.svelte'] || found['layout.module']) {
			layouts = [...layouts, { component: found['layout.svelte'] ?? null, module: found['layout.module'] ?? null }];
		}
		// After the layout: an error page renders inside the layout of its own directory.
		if (found['error.svelte']) {
			error = found['error.svelte'];
			error_depth = layouts.length;
		}
		if (id_parts.length === 0) fallback = { layouts, error };

		if (found['page.module'] && !found['page.svelte']) {
			fail(`${found['page.module']} has no +page.svelte next to it.`);
		}
		if (found['page.svelte']) {
			routes.push({
				id: '/' + id_parts.join('/'),
				segments: id_parts.filter((part) => !/^\(.*\)$/.test(part)),
				layouts,
				page: { component: found['page.svelte'], module: found['page.module'] ?? null },
				error,
				error_depth
			});
		}

		for (const entry of entries) {
			if (!entry.isDirectory()) continue;
			const path = `${current}/${entry.name}`;
			if (/\[\[|\]\]|=/.test(entry.name)) {
				fail(`${path}: optional parameters ([[name]]) and matchers ([name=matcher]) are not supported.`);
			}
			walk(path, [...id_parts, entry.name], layouts, error, error_depth);
		}
	}

	// With `/` on Windows too: the files are imported by these paths, and that
	// is how Vite writes a module's id.
	walk(dir.replaceAll('\\', '/'), [], [], null, 0);

	const seen = new Map();
	for (const route of routes) {
		const path = '/' + route.segments.join('/');
		if (seen.has(path)) fail(`${route.id} and ${seen.get(path)} are both the page for ${path}.`);
		seen.set(path, route.id);
	}

	routes.sort(by_specificity);
	return { routes, fallback, files };
}

/** 0 for fixed text, 1 for a segment with a parameter, 2 for a rest parameter. @param {string} segment */
const rank = (segment) => (segment.includes('[...') ? 2 : segment.includes('[') ? 1 : 0);

/**
 * The first route that matches wins, so the more specific one has to come
 * first: `/items/new` before `/items/[id]` before `/items/[...rest]`.
 * @param {ScannedRoute} a
 * @param {ScannedRoute} b
 */
function by_specificity(a, b) {
	const length = Math.max(a.segments.length, b.segments.length);
	for (let index = 0; index < length; index++) {
		// Running out of segments ranks between a parameter and a rest parameter:
		// `/docs` must not lose to `/docs/[...path]`, which also matches it.
		const [ra, rb] = [a, b].map((route) => (index < route.segments.length ? rank(route.segments[index]) : 1.5));
		if (ra !== rb) return ra - rb;
	}
	return a.id < b.id ? -1 : 1;
}

/**
 * @param {string[]} segments
 * @returns {{ pattern: string, names: string[] }} `pattern` is regular expression source
 */
export function route_pattern(segments) {
	/** @type {string[]} */
	const names = [];
	let pattern = '';
	for (const segment of segments) {
		const rest = /^\[\.\.\.(\w+)\]$/.exec(segment);
		if (rest) {
			// Matches nothing as well: `/docs/[...path]` is also the page for `/docs`.
			names.push(rest[1]);
			pattern += '(?:/(.*))?';
			continue;
		}
		pattern += '/';
		// Text and parameters can share a segment: `v[major].[minor]`.
		for (const part of segment.split(/(\[\w+\])/)) {
			const param = /^\[(\w+)\]$/.exec(part);
			if (param) {
				names.push(param[1]);
				pattern += '([^/]+)';
			} else {
				pattern += part.replace(/[.*+?^${}()|[\]\\/]/g, '\\$&');
			}
		}
	}
	return { pattern: `^${pattern || '/'}$`, names };
}

/**
 * The source of `virtual:svelte-blitz/routes`.
 * @param {string} dir absolute path of `src/routes`
 * @returns {{ code: string, files: string[] }} `files` are what the module was built from
 */
export function generate_routes(dir) {
	const { routes, fallback, files } = scan_routes(dir);
	/** @type {string[]} */
	const imports = [];
	/** @type {Map<string, string>} */
	const names = new Map();

	/** A variable holding the default export (a component) or the whole module. @param {string | null} file @param {boolean} whole */
	const use = (file, whole) => {
		if (file === null) return 'null';
		let name = names.get(file);
		if (name === undefined) {
			name = `_${names.size}`;
			names.set(file, name);
			imports.push(`import ${whole ? '* as ' : ''}${name} from ${JSON.stringify(file)};`);
		}
		return name;
	};
	/** @param {NodeFiles} files */
	const node = (files) => {
		const module = use(files.module, true);
		return `{ component: ${use(files.component, false)}, load: ${module === 'null' ? 'undefined' : `${module}.load`} }`;
	};
	/** @param {NodeFiles[]} list */
	const nodes = (list) => `[${list.map(node).join(', ')}]`;

	const table = routes.map((route) => {
		const { pattern, names: params } = route_pattern(route.segments);
		return (
			`\t{ id: ${JSON.stringify(route.id)}, pattern: new RegExp(${JSON.stringify(pattern)}), names: ${JSON.stringify(params)}, ` +
			`layouts: ${nodes(route.layouts)}, page: ${node(route.page)}, ` +
			`error: ${use(route.error, false)}, error_depth: ${route.error_depth} }`
		);
	});

	const code =
		`${imports.join('\n')}\n\n` +
		`export const routes = [\n${table.join(',\n')}\n];\n\n` +
		`export const fallback = { layouts: ${nodes(fallback.layouts)}, error: ${use(fallback.error, false)} };\n`;
	return { code, files };
}
