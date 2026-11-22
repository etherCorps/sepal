// Vite preset: compile Svelte for the Blitz renderer and emit exactly one
// `app.js` (an IIFE the host can eval) and one `app.css`.

//
// Routing: if the project has `src/routes`, the entry is the built-in router
// and the route table is generated from that directory (see routes.js).
// Otherwise the entry is `src/main.js` or `src/main.ts`. The `router` option
// swaps in a router of your own, or turns routing off.
//
// In watch mode (`vite build --watch`) with a host binary available, it also
// opens the app and reloads it in place after every rebuild.

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { find_host } from '@ethercorps/svelte-blitz-host';
import { generate_routes } from './routes.js';

const ROUTES_ID = 'virtual:svelte-blitz/routes';
/** @param {string} file a file in this package's `src` */
const own = (file) => fileURLToPath(new URL(file, import.meta.url));

/**
 * What the app tells the host about itself. Written to `dist/app.json`.
 *
 * `name` is the app's name and the fallback window title. `icon` is a PNG
 * (square, 1024×1024 recommended), relative to the Vite root; it is copied to
 * `dist/icon.png`. `width` and `height` are the initial window size in logical
 * pixels. `id` (a bundle identifier such as `com.example.counter`) and
 * `version` are used when the app is bundled.
 *
 * @typedef {{
 *   name?: string,
 *   id?: string,
 *   version?: string,
 *   icon?: string,
 *   window?: { title?: string, width?: number, height?: number }
 * }} AppManifest
 */

/**
 * Routing. Leave it out for the default: the built-in router when the
 * project has a `src/routes` directory, no router when it does not.
 *
 * - `false` turns routing off even if `src/routes` exists.
 * - `dir` is where the route files are (default `src/routes`).
 * - `entry` is a router of your own: a module that becomes the app's entry.
 *   It mounts the app itself, and can import the generated route table
 *   (`routes`, `fallback`) from `virtual:svelte-blitz/routes`. The built-in
 *   `$app/navigation` and `$app/state` are not set up for it.
 *
 * @typedef {false | { dir?: string, entry?: string }} RouterOptions
 */

/**
 * @param {{
 *   entry?: string,
 *   app?: AppManifest,
 *   router?: RouterOptions,
 *   svelte?: import('@sveltejs/vite-plugin-svelte').Options
 * }} [options]
 * @returns {import('vite').PluginOption[]}
 */
export function svelteBlitz(options = {}) {
	let routes_dir = '';
	const user = options.svelte ?? {};
	let root = '';
	let out_dir = '';
	let watching = false;
	/** @type {import('node:child_process').ChildProcess | null} */
	let host = null;

	return [
		svelte({
			...user,
			// Component <style> blocks go to app.css; `css: 'injected'` needs a document.
			emitCss: true,
			compilerOptions: {
				...user.compilerOptions,
				css: 'external',
				experimental: {
					...user.compilerOptions?.experimental,
					customRenderer: '@ethercorps/svelte-blitz/renderer'
				}
			}
		}),
		/** @type {import('vite').Plugin} */ ({
			name: 'svelte-blitz:config',
			config(user) {
				const project = resolve(user.root ?? '.');
				const router = options.router;
				routes_dir = resolve(project, (router && router.dir) || 'src/routes');
				const custom_router = router ? router.entry : undefined;
				const built_in_router = router !== false && custom_router === undefined && existsSync(routes_dir);

				const entry = custom_router
					? resolve(project, custom_router)
					: built_in_router
						? own('./router-entry.js')
						: // No extension: Vite resolves `src/main.js` or `src/main.ts`, whichever exists.
							(options.entry ?? 'src/main');

				return {
					// Files in `static/` are copied to the build as they are, so the app
					// can refer to them by name: `<img src="logo.svg">`. Vite calls this
					// directory `public`; a `publicDir` in the user's config still wins.
					...(user.publicDir === undefined ? { publicDir: 'static' } : {}),
					resolve: {
						// Selects Svelte's client build with custom renderer support.
						conditions: ['svelte-custom-renderer', 'svelte', 'module', 'import', 'default'],
						// The names a SvelteKit project uses. The `$app` ones belong to
						// the built-in router; another router brings its own API.
						alias: {
							$lib: join(project, 'src/lib'),
							...(built_in_router
								? { '$app/navigation': own('./navigation.js'), '$app/state': own('./state.js') }
								: {})
						}
					},
					build: {
						target: 'es2022',
						cssCodeSplit: false,
						lib: {
							entry,
							name: 'app',
							formats: ['iife'],
							fileName: () => 'app.js',
							cssFileName: 'app'
						}
					}
				};
			},
			resolveId(id) {
				if (id === ROUTES_ID) return '\0' + ROUTES_ID;
			},
			load(id) {
				if (id !== '\0' + ROUTES_ID) return;
				// A custom router may not use route files at all.
				if (!existsSync(routes_dir)) return 'export const routes = [];\nexport const fallback = { layouts: [], error: null };\n';
				const { code, files } = generate_routes(routes_dir);
				// Rebuild when a route file is added, removed or changed.
				this.addWatchFile(routes_dir);
				for (const file of files) this.addWatchFile(file);
				return code;
			},
			configResolved(config) {
				root = config.root;
				out_dir = resolve(root, config.build.outDir);
				watching = config.build.watch != null;
			},
			generateBundle() {
				if (!options.app) return;
				const { icon, ...manifest } = options.app;
				if (icon) {
					// The host decodes PNG only; fail here rather than when the app starts.
					if (!icon.toLowerCase().endsWith('.png')) this.error(`app.icon must be a PNG file, got ${icon}`);
					this.emitFile({ type: 'asset', fileName: 'icon.png', source: readFileSync(resolve(root, icon)) });
				}
				this.emitFile({
					type: 'asset',
					fileName: 'app.json',
					source: JSON.stringify(icon ? { ...manifest, icon: 'icon.png' } : manifest, null, '\t') + '\n'
				});
			}
		}),
		{
			name: 'svelte-blitz:dev',
			// `writeBundle`, not `closeBundle`: it only runs when the build
			// succeeded, so a syntax error leaves the last good build on screen.
			writeBundle() {
				if (!watching) return;

				if (host !== null) {
					// The host reloads on every line it reads.
					host.stdin?.write('reload\n');
					return;
				}

				let bin = find_host();
				if (bin === null) {
					console.warn('svelte-blitz: no host binary found, so the app will not open. Set SVELTE_BLITZ_HOST.');
					watching = false;
					return;
				}

				// macOS takes an app's name and Dock icon from the bundle it runs
				// from, so in dev the host runs out of a throwaway one. It still
				// loads the app from `dist`, not from the copy inside the bundle.
				if (process.platform === 'darwin' && options.app?.name) {
					const made = spawnSync(bin, ['--bundle', out_dir, resolve(root, 'node_modules/.svelte-blitz')], {
						encoding: 'utf8'
					});
					if (made.status === 0) {
						const bundle = made.stdout.trim();
						bin = join(bundle, 'Contents/MacOS', basename(bundle, '.app'));
					} else {
						console.warn(`svelte-blitz: running unbundled: ${made.stderr.trim()}`);
					}
				}

				const started = bin;
				host = spawn(started, ['--watch', out_dir], { stdio: ['pipe', 'inherit', 'inherit'] });
				host.on('error', (error) => {
					console.error(`svelte-blitz: cannot start ${started}: ${error.message}`);
					process.exit(1);
				});
				// Closing the window ends the session. The other direction needs no
				// code: when Vite exits, the host's stdin closes and it quits.
				host.on('exit', (code) => process.exit(code ?? 0));
			}
		}
	];
}
