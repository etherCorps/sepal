// Vite preset: compile Svelte for the Blitz renderer, as a web build: the
// app's `index.html`, the ES module chunks it loads, one stylesheet, the
// static files, and `app.json` for the host.
//
// Settings come from `sepal.config.ts` (or `.js`/`.mjs`) in the project
// root, so that `vite.config.ts` stays a list of plugins. Options passed to
// `sepal()` directly override the file, key by key.
//
// The entry is `index.html`, as for any Vite app; its module script
// (`src/main.ts` in the examples) mounts the app.
//
// A SvelteKit app (one with `src/app.html`) uses SvelteKit's own plugin,
// given its options through `withSepal()` (the custom renderer, no version
// polling, the rewrites of `bind:` and `<svelte:head>`) with
// `sepal/adapter`, and this preset after it, which adds what SvelteKit's
// options cannot: the custom renderer's build conditions, the edits to
// SvelteKit's client (see `patch`), `sepal.config.ts` and the dev window.
// See `KIT_SETUP`.
//
// In watch mode (`vite build --watch`) with a host binary available, it also
// opens the app and reloads it in place after every rebuild.

import { existsSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { preprocess } from 'svelte/compiler';
import { normalizePath } from 'vite';
import { dev_window, kit, load_config, manifest } from './app.js';
import { element_bindings } from './bindings.js';
import { head } from './head.js';
import { check_css, check_page } from './page.js';

/** A SvelteKit app's vite.config.ts, for the messages that point to it. */
export const KIT_SETUP = `import { sveltekit } from '@sveltejs/kit/vite';
import adapter from 'sepal/adapter';
import { sepal, withSepal } from 'sepal/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit(withSepal({ adapter: adapter() })), sepal()]
});`;

/** What `index.html` needs, for the message when there is none. */
const PAGE = `<!doctype html>
<html lang="en">
	<head>
		<meta charset="utf-8" />
		<title>My app</title>
	</head>
	<body>
		<script type="module" src="/src/main.ts"></script>
	</body>
</html>`;

/**
 * What the app tells the host about itself. Written to `dist/app.json`.
 *
 * `name` is the app's name and the fallback window title. `icon` is a PNG
 * (square, 1024×1024 recommended) in the `static/` directory, named relative to
 * it; `static/icon.png` is used when `icon` is left out, and the Sepal icon
 * when there is no `static/icon.png` either. `width` and `height`
 * are the initial window size in logical pixels, and `minWidth` and `minHeight` how
 * small the window can be made: the size below which the app's layout cannot
 * work. The window never opens smaller. `id` (a bundle identifier such as `com.example.counter`) and
 * `version` are used when the app is bundled.
 *
 * `updater` turns on self-update: `url` is where the update feed
 * (`latest.json`) is published, `pubkey` is the public half of the key the
 * updates are signed with (`sepal signer generate`). The app then
 * updates itself through `sepal/updater`.
 *
 * @typedef {{
 *   name?: string,
 *   id?: string,
 *   version?: string,
 *   icon?: string,
 *   updater?: { url: string, pubkey: string },
 *   window?: { title?: string, width?: number, height?: number, minWidth?: number, minHeight?: number }
 * }} AppManifest
 */

/**
 * What the app may do outside its window. Everything is off unless it is
 * listed here; a call that is not allowed rejects with a message naming the
 * entry to add.
 *
 * - `http`: which URLs `fetch` may reach. Each entry is an origin, optionally
 *   with a path prefix (`https://api.example.com`, `https://example.com/v1/`),
 *   or a host with a wildcard (`https://*.example.com`). `true` allows any.
 * - `fs`: which directories the file APIs may touch: `'data'` (the app's own
 *   data directory), `'documents'`, `'downloads'`, `'desktop'`, `'home'`, or an
 *   absolute path. `true` allows any path. A file the user picks in a dialog
 *   is allowed for as long as the app runs, whatever is listed here.
 * - `dialog`: open, save and message dialogs.
 * - `clipboard`: reading and writing the clipboard.
 * - `open`: opening web and mail links, and allowed files, in their default app.
 * - `window`: changing the window: minimise, maximise, decorations, close,
 *   drag. The title needs no permission.
 *
 * @typedef {{
 *   http?: boolean | string[],
 *   fs?: boolean | string[],
 *   dialog?: boolean,
 *   clipboard?: boolean,
 *   open?: boolean,
 *   window?: boolean
 * }} Permissions
 */

/**
 * The settings in `sepal.config.ts`.
 *
 * `dom: true` is for apps that use component libraries written for the web.
 * It builds the app as a browser bundle and loads a small DOM
 * (`window`, `document`, element methods; see `sepal/dom`)
 * before anything else, so those libraries find what they reach for.
 *
 * @typedef {{
 *   app?: AppManifest,
 *   permissions?: Permissions,
 *   dom?: boolean
 * }} Config
 */

/**
 * For `sepal.config.ts`: types the settings and returns them unchanged.
 * A function is called with Vite's `{ mode, command }`.
 *
 * @template {Config | ((env: import('vite').ConfigEnv) => Config | Promise<Config>)} T
 * @param {T} config
 * @returns {T}
 */
export function defineConfig(config) {
	return config;
}

/**
 * @param {Config & { svelte?: import('@sveltejs/vite-plugin-svelte').Options }} [inline]
 *   Overrides for the config file. `svelte` (options for vite-plugin-svelte) can only be given here.
 * @returns {import('vite').PluginOption[]}
 */
export function sepal(inline = {}) {
	// A SvelteKit app has SvelteKit's page template. SvelteKit's own plugin
	// compiles it then (with the `customRenderer` option); this one adds the
	// rest. `configResolved` checks the guess against the plugins there are.
	const kit_app = existsSync(resolve('src/app.html'));
	/** The config file with `inline` on top. Filled in by the `config` hook. @type {Config} */
	let options = inline;
	/** @type {string | null} */
	let config_path = null;
	/** @type {import('vite').ConfigEnv} */
	let config_env = { mode: 'production', command: 'build' };
	/**
	 * The modules `index.html` loads. Sepal's runtime goes first in each (the
	 * DOM layer, with `dom`, which loads it): its engine fixes have to be in
	 * place before Svelte's runtime runs, which keeps its own copy of
	 * `Object.getOwnPropertyDescriptor`, whatever order the app imports in.
	 */
	const page_entries = new Set();
	/** @type {import('vite').Logger | null} */
	let logger = null;
	const user = inline.svelte ?? {};
	let root = '';
	/** The directory copied into the build as it is (`static/`). Empty when turned off. */
	let static_dir = '';
	let out_dir = '';
	let watching = false;
	const show = dev_window(() => ({ root, out_dir, name: options.app?.name }));

	/** @type {import('vite').Plugin} */
	const config_plugin = {
		name: 'sepal:config',
		async config(user, env) {
			const project = resolve(user.root ?? '.');
			config_env = env;
			const file = await load_config(project, env);
			config_path = file.path;
			options = { ...file.config, ...inline };

			if (kit_app) {
				// The adapter writes the app, and says when it has.
				kit.session = {
					root: project,
					options,
					written: (written_to) => {
						out_dir = written_to;
						if (watching) show();
					}
				};
				return {
					resolve: {
						// Svelte's client build with custom renderer support, and
						// `browser`: SvelteKit's apps always have the DOM layer.
						conditions: ['svelte-custom-renderer', 'browser', 'svelte', 'module', 'import', 'default']
					}
				};
			}

			const has_page = existsSync(join(project, 'index.html'));
			if (!has_page && existsSync(join(project, 'src/routes'))) {
				throw new Error(`sepal: this app has pages in src/routes. Pages are SvelteKit's: add SvelteKit's src/app.html, a src/routes/+layout.ts with \`export const ssr = false;\`, and in vite.config.ts:\n\n${KIT_SETUP}`);
			}
			if (!has_page) {
				throw new Error(`sepal: this app has no index.html. It is the page the app starts from, as in any Vite app, next to vite.config.ts:\n\n${PAGE}`);
			}

			return {
				// Files in `static/` are copied to the build as they are, so the app
				// can refer to them by name: `<img src="logo.svg">`. Vite calls this
				// directory `public`; a `publicDir` in the user's config still wins.
				...(user.publicDir === undefined ? { publicDir: 'static' } : {}),
				resolve: {
					// Selects Svelte's client build with custom renderer support. With
					// `dom`, `browser` as well: libraries check it at build time to
					// decide whether there is a DOM to work with.
					conditions: ['svelte-custom-renderer', ...(options.dom ? ['browser'] : []), 'svelte', 'module', 'import', 'default'],
					// The name SvelteKit 2 projects use for `src/lib`.
					alias: { $lib: join(project, 'src/lib') }
				},
				build: {
					target: 'es2022',
					// One stylesheet, linked from index.html. A chunk's own
					// stylesheet would be added with `document`, which an app
					// without the DOM layer does not have.
					cssCodeSplit: false,
					// So would preloading: Vite's modulepreload polyfill and the
					// <link>s it writes ahead of a dynamic import. The host loads
					// a module from the app folder when it is imported.
					modulePreload: false
				}
			};
		},
		resolveId: {
			// Before Vite's own resolver, which would answer first.
			order: 'pre',
			async handler(source, importer) {
				// The modules index.html loads, for `page_entries`. SvelteKit's
				// entry gets the same from `patch`.
				if (kit_app || !importer?.endsWith('.html')) return;
				const resolved = await this.resolve(source, importer, { skipSelf: true });
				if (resolved) page_entries.add(resolved.id);
				return resolved;
			}
		},
		transform(code, id) {
			if (page_entries.has(id)) return { code: `import '${options.dom ? 'sepal/dom' : 'sepal'}';\n${code}`, map: null };
			// Stylesheets, component styles included: nothing in them may load from the network.
			if (/\.css($|\?)/.test(id)) {
				const errors = check_css(code, normalizePath(relative(root, id.split('?')[0])));
				if (errors.length > 0) this.error(`sepal: ${errors.join('\nsepal: ')}`);
			}
		},
		transformIndexHtml: {
			order: 'pre',
			handler(html) {
				const { errors, warnings } = check_page(html);
				if (errors.length > 0) throw new Error(`sepal: ${errors.join('\nsepal: ')}`);
				for (const warning of warnings) logger?.warn(`sepal: ${warning}`);
			}
		},
		configResolved(config) {
			logger = config.logger;
			root = config.root;
			static_dir = config.publicDir;
			out_dir = resolve(root, config.build.outDir);
			watching = config.build.watch != null;
			const kit_plugin = config.plugins.find((plugin) => plugin.name === 'vite-plugin-sveltekit-setup');
			const has_kit = kit_plugin !== undefined;
			// SvelteKit's options, as validated. The adapter checks them too, but
			// only once everything is compiled, and a component with `bind:` on an
			// element would stop the build first, with Svelte's message.
			const kit_options = kit_plugin?.api?.options;
			if (kit_app && kit_options) {
				const missing = kit_setup_problems(kit_options);
				if (missing.length > 0) throw new Error(`sepal: ${missing.join('\nsepal: ')}`);
			}
			if (kit_app && !has_kit) {
				throw new Error(`sepal: this project has src/app.html, so it is a SvelteKit app, but vite.config.ts has no \`sveltekit()\`. Set it up as:\n\n${KIT_SETUP}`);
			}
			if (!kit_app && has_kit) {
				throw new Error(`sepal: vite.config.ts has \`sveltekit()\`, but there is no src/app.html where sepal() looks, in ${process.cwd()}. Run Vite from the project's directory.`);
			}
		},
		async buildStart() {
			// In watch mode, pick up edits to the config file. What it says about
			// the app and its permissions takes effect on the next build; a
			// change to `dom` needs Vite restarted.
			if (config_path === null) return;
			this.addWatchFile(config_path);
			const file = await load_config(root, config_env);
			options = { ...file.config, ...inline };
			if (kit.session !== null) kit.session.options = options;
		},
		generateBundle() {
			// A SvelteKit app's manifest is the adapter's to write.
			if (kit_app) return;
			let written;
			try {
				written = manifest(options, root, static_dir);
			} catch (error) {
				this.error(/** @type {Error} */ (error).message);
			}
			if (!written) return;
			this.emitFile({ type: 'asset', fileName: 'app.json', source: written.json });
			if (written.icon) this.emitFile({ type: 'asset', fileName: 'icon.png', source: readFileSync(written.icon) });
		}
	};

	if (kit_app) {
		// SvelteKit writes its own page, and warns about plugins that ask for it.
		delete config_plugin.transformIndexHtml;
		return [config_plugin, sveltekit_edits()];
	}
	return [
		svelte({
			...user,
			// Ours first: they have to see `bind:` on elements and `<svelte:head>`
			// before Svelte refuses them.
			preprocess: [element_bindings(), head(), ...[user.preprocess ?? []].flat()],
			// Component <style> blocks go to app.css; `css: 'injected'` needs a document.
			emitCss: true,
			compilerOptions: {
				...user.compilerOptions,
				css: 'external',
				experimental: {
					...user.compilerOptions?.experimental,
					customRenderer: 'sepal/renderer'
				}
			}
		}),
		config_plugin,
		{
			name: 'sepal:dev',
			// `writeBundle`, not `closeBundle`: it only runs when the build
			// succeeded, so a syntax error leaves the last good build on screen.
			writeBundle() {
				if (watching) show();
			}
		}
	];
}

/**
 * Sepal's rewrites of what Svelte refuses for a custom renderer (`bind:` on
 * elements, `<svelte:head>`), as one preprocessor. `withSepal()` adds it;
 * this is for an app that sets `sveltekit()`'s options by hand, after its
 * own preprocessors: `preprocess: [vitePreprocess(), sepalPreprocess()]`.
 * @returns {import('svelte/compiler').PreprocessorGroup}
 */
export function sepalPreprocess() {
	const rewrites = [element_bindings(), head()];
	return {
		name: 'sepal',
		async markup({ content, filename }) {
			if (!content.includes('bind:') && !content.includes('<svelte:head')) return;
			const rewritten = await preprocess(content, rewrites, { filename });
			return { code: rewritten.code, map: /** @type {any} */ (rewritten.map) };
		}
	};
}

/** `{}` or `Object.create(null)`: not an array, a function or a class instance. @param {unknown} value @returns {value is Record<string, any>} */
const plain = (value) => value !== null && typeof value === 'object' && [Object.prototype, null].includes(Object.getPrototypeOf(value));

/**
 * `ours` merged into a copy of `theirs`, level by level where both are plain
 * objects. Anything else is taken as it is; where both have a value, ours.
 * @param {Record<string, any>} theirs @param {Record<string, any>} ours
 * @returns {Record<string, any>}
 */
function merge(theirs, ours) {
	const merged = { ...theirs };
	for (const [key, value] of Object.entries(ours)) merged[key] = plain(value) && plain(theirs[key]) ? merge(theirs[key], value) : value;
	return merged;
}

/**
 * SvelteKit's options for a Sepal app: `sveltekit(withSepal({ adapter: adapter() }))`,
 * with `adapter` from `sepal/adapter`. Adds what every Sepal app needs to
 * the app's own options: components compiled for the Sepal renderer, no
 * version polling (apps update through `sepal/updater`), and Sepal's
 * rewrites after the app's own preprocessors (`sepalPreprocess()`), once.
 * Throws on options that cannot work in an app.
 * @template {import('@sveltejs/kit/vite').Config} T
 * @param {T} options
 * @returns {T}
 */
export function withSepal(options) {
	const usage = "`sveltekit(withSepal({ adapter: adapter() }))`, with `import adapter from 'sepal/adapter'`";
	const given = /** @type {any} */ (options ?? {});
	if (given.adapter === undefined) throw new Error(`sepal: withSepal() has no adapter. A Sepal app's is sepal/adapter, which writes the app for the host: ${usage}.`);
	if (given.adapter?.name !== 'sepal') throw new Error(`sepal: withSepal() was given the adapter ${given.adapter?.name ?? '(unnamed)'}. A Sepal app's is sepal/adapter, which writes the app for the host: ${usage}.`);
	const renderer = given.compilerOptions?.experimental?.customRenderer;
	if (renderer !== undefined && renderer !== 'sepal/renderer') {
		throw new Error(`sepal: compilerOptions.experimental.customRenderer is ${JSON.stringify(renderer)}. A Sepal app's components are compiled for 'sepal/renderer'; leave it out, withSepal() sets it.`);
	}
	const poll = given.version?.pollInterval;
	if (typeof poll === 'number' && poll > 0) {
		throw new Error(`sepal: version.pollInterval is ${poll}. SvelteKit would ask the server for a new version that often, waking an idle app, and an app has no server to answer. Apps update through sepal/updater; leave it out, withSepal() sets it to 0.`);
	}
	const merged = merge(given, { compilerOptions: { experimental: { customRenderer: 'sepal/renderer' } }, version: { pollInterval: 0 } });
	const groups = [given.preprocess ?? []].flat();
	merged.preprocess = groups.some((group) => group?.name === 'sepal') ? groups : [...groups, sepalPreprocess()];
	return /** @type {T} */ (merged);
}

/**
 * What SvelteKit's options lack for a Sepal app, as messages; empty when
 * nothing does. `sepal()` checks before the build, the adapter after it.
 * @param {any} config SvelteKit's options, as it validated them
 * @returns {string[]}
 */
export function kit_setup_problems(config) {
	const found = [];
	const fix = 'Pass SvelteKit\'s options through withSepal() from sepal/vite: `sveltekit(withSepal({ adapter: adapter() }))` in vite.config.ts';
	if (config.compilerOptions?.experimental?.customRenderer !== 'sepal/renderer') {
		found.push(`components are not compiled for the Sepal renderer. ${fix} (by hand: \`compilerOptions: { experimental: { customRenderer: 'sepal/renderer' } }\`).`);
	}
	if (config.version?.pollInterval !== 0) {
		found.push(`SvelteKit would ask for a new version every hour, waking an idle app; apps update through \`sepal/updater\`. ${fix} (by hand: \`version: { pollInterval: 0 }\`).`);
	}
	if (![config.preprocess ?? []].flat().some((group) => group?.name === 'sepal')) {
		found.push(`Sepal's rewrites of \`bind:\` and \`<svelte:head>\` are not among the preprocessors. ${fix} (by hand: \`sepalPreprocess()\` from sepal/vite, last in \`preprocess\`).`);
	}
	return found;
}

/**
 * The edits to SvelteKit's client. Each names the file, what to find in it,
 * and what goes there instead. SvelteKit's peer range in package.json stops
 * at the versions these were checked against.
 * @type {{ file: string, find: string, replace: string, prepend: string, why: string }[]}
 */
const patch = [
	{
		// The entry of SvelteKit's client: `entry.js` for its default split
		// build, `bundle.js` for one bundle. The DOM layer and the runtime
		// (`__sb`, the host's way in) have to exist before any of SvelteKit runs.
		file: '/kit/src/runtime/client/entry.js',
		find: 'export function init(payload) {',
		replace: 'export function init(payload) {',
		prepend: "import 'sepal/dom';\nimport 'sepal';\n",
		why: 'how its client starts'
	},
	{
		file: '/kit/src/runtime/client/bundle.js',
		find: 'export function start(element, options) {',
		replace: 'export function start(element, options) {',
		prepend: "import 'sepal/dom';\nimport 'sepal';\n",
		why: 'how its single bundle starts'
	},
	{
		// SvelteKit mounts its root component itself, and does not say with
		// which renderer.
		file: '/kit/src/runtime/client/client.js',
		find: 'render(Root, {',
		replace: 'render(Root, {\n\t\trenderer: __sepal_renderer,',
		prepend: "import __sepal_renderer from 'sepal/renderer';\n",
		why: 'how it mounts its root component'
	}
];

/**
 * The edits to SvelteKit's client. The rewrites of components are
 * SvelteKit's options' (`withSepal()`), so that svelte-check sees them too.
 * @returns {import('vite').Plugin}
 */
function sveltekit_edits() {
	return {
		name: 'sepal:sveltekit',
		enforce: 'pre',
		transform(code, id) {
			const edit = patch.find(({ file }) => id.endsWith(file));
			if (edit === undefined) return;
			if (!code.includes(edit.find)) {
				this.error(`sepal: this version of SvelteKit changed ${edit.why}, which Sepal has to adjust. Use the SvelteKit version Sepal was tested with (see sepal's peer range for @sveltejs/kit), or update Sepal.`);
			}
			return { code: edit.prepend + code.replace(edit.find, edit.replace), map: null };
		}
	};
}
