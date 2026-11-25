// Vite preset: compile Svelte for the Blitz renderer, as a web build: the
// app's `index.html`, the ES module chunks it loads, one stylesheet, the
// static files, and `app.json` for the host.
//
// Settings come from `sepal.config.ts` (or `.js`/`.mjs`) in the project
// root, so that `vite.config.ts` stays a list of plugins. Options passed to
// `sepal()` directly override the file, key by key.
//
// The entry is `index.html`, as for any Vite app; its module script
// (`src/main.ts` in the examples) mounts the app. An app with pages in
// `src/routes` is a SvelteKit app: it uses `sepalKit()` from
// `sepal/sveltekit` instead of this preset.
//
// In watch mode (`vite build --watch`) with a host binary available, it also
// opens the app and reloads it in place after every rebuild.

import { existsSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { normalizePath } from 'vite';
import { dev_window, load_config, manifest } from './app.js';
import { element_bindings } from './bindings.js';
import { head } from './head.js';
import { check_css, check_page } from './page.js';

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
 * are the initial window size in logical pixels. `id` (a bundle identifier such as `com.example.counter`) and
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
 *   window?: { title?: string, width?: number, height?: number }
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
	/** The config file with `inline` on top. Filled in by the `config` hook. @type {Config} */
	let options = inline;
	/** @type {string | null} */
	let config_path = null;
	/** @type {import('vite').ConfigEnv} */
	let config_env = { mode: 'production', command: 'build' };
	/** The modules `index.html` loads, when `dom` is on: the DOM layer goes first in each. */
	const dom_entries = new Set();
	/** @type {import('vite').Logger | null} */
	let logger = null;
	const user = inline.svelte ?? {};
	let root = '';
	/** The directory copied into the build as it is (`static/`). Empty when turned off. */
	let static_dir = '';
	let out_dir = '';
	let watching = false;
	const show = dev_window(() => ({ root, out_dir, name: options.app?.name }));

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
		/** @type {import('vite').Plugin} */ ({
			name: 'sepal:config',
			async config(user, env) {
				const project = resolve(user.root ?? '.');
				config_env = env;
				const file = await load_config(project, env);
				config_path = file.path;
				options = { ...file.config, ...inline };

				const has_page = existsSync(join(project, 'index.html'));
				if (!has_page && existsSync(join(project, 'src/routes'))) {
					throw new Error(
						"sepal: this app has pages in src/routes and no index.html. Pages are SvelteKit's: in vite.config.ts, use `sepalKit()` from 'sepal/sveltekit' in place of `sepal()`, and add a src/routes/+layout.ts with `export const ssr = false;`."
					);
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
					// The modules index.html loads: with `dom`, the DOM layer has to
					// exist before any library module runs, so it goes first in them.
					if (!options.dom || !importer?.endsWith('.html')) return;
					const resolved = await this.resolve(source, importer, { skipSelf: true });
					if (resolved) dom_entries.add(resolved.id);
					return resolved;
				}
			},
			transform(code, id) {
				if (dom_entries.has(id)) return { code: `import 'sepal/dom';\n${code}`, map: null };
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
			},
			async buildStart() {
				// In watch mode, pick up edits to the config file. What it says about
				// the app and its permissions takes effect on the next build; a
				// change to `dom` needs Vite restarted.
				if (config_path === null) return;
				this.addWatchFile(config_path);
				const file = await load_config(root, config_env);
				options = { ...file.config, ...inline };
			},
			generateBundle() {
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
		}),
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
