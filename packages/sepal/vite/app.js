// What every way of building an app shares, whichever router it uses: reading
// `sepal.config.ts`, writing the manifest the host reads (`app.json`), and
// opening the app in dev and reloading it after each rebuild.
//
// Runs in Node.

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { loadConfigFromFile } from 'vite';
import { find_host } from '@ethercorps/calyx';

const CONFIG_FILES = ['sepal.config.ts', 'sepal.config.js', 'sepal.config.mjs'];

/**
 * Read the project's config file, if it has one.
 * @param {string} project
 * @param {import('vite').ConfigEnv} env
 * @returns {Promise<{ config: import('./vite.js').Config, path: string | null }>}
 */
export async function load_config(project, env) {
	const path = CONFIG_FILES.map((file) => join(project, file)).find((file) => existsSync(file));
	if (path === undefined) return { config: {}, path: null };
	// Vite's own loader: it handles TypeScript, and calls a function export with `env`.
	const loaded = await loadConfigFromFile(env, path, project, 'silent');
	return { config: /** @type {import('./vite.js').Config} */ (loaded?.config ?? {}), path };
}

/**
 * The contents of `app.json`, or `null` when the config says nothing the host
 * needs. Throws with a message for the user when the icon is not usable.
 * @param {import('./vite.js').Config} options
 * @param {string} root the project
 * @param {string} static_dir where the icon is looked for; empty when there is none
 * @returns {string | null}
 */
export function manifest(options, root, static_dir) {
	if (!options.app && !options.permissions) return null;
	const app = { ...options.app };
	// The version in package.json is the app's version unless the config says otherwise.
	if (app.version === undefined) {
		const pkg = join(root, 'package.json');
		if (existsSync(pkg)) app.version = JSON.parse(readFileSync(pkg, 'utf8')).version;
	}
	// The icon is one of the static files: it is copied into the build with
	// them, and the manifest names it by the same relative path.
	if (app.icon === undefined && static_dir && existsSync(join(static_dir, 'icon.png'))) app.icon = 'icon.png';
	if (app.icon !== undefined) {
		// The host decodes PNG only; fail here rather than when the app starts.
		if (!app.icon.toLowerCase().endsWith('.png')) throw new Error(`app.icon must be a PNG file, got ${app.icon}`);
		if (!static_dir || !existsSync(join(static_dir, app.icon))) {
			throw new Error(`app.icon: there is no ${app.icon} in the static directory (${static_dir || 'turned off'})`);
		}
	}
	return JSON.stringify(options.permissions ? { ...app, permissions: options.permissions } : app, null, '\t') + '\n';
}

/**
 * The app's window in dev. The first call opens it; every later one reloads
 * it in place. Returns a function to call after each successful build.
 * @param {() => { root: string, out_dir: string, name: string | undefined }} project read at the first call
 */
export function dev_window(project) {
	/** @type {import('node:child_process').ChildProcess | null} */
	let host = null;
	let gave_up = false;

	return () => {
		if (gave_up) return;
		if (host !== null) {
			// The host reloads on every line it reads.
			host.stdin?.write('reload\n');
			return;
		}

		let bin = find_host();
		if (bin === null) {
			console.warn('sepal: no calyx (host binary) found, so the app will not open. Set SEPAL_HOST.');
			gave_up = true;
			return;
		}
		const { root, out_dir, name } = project();

		// macOS takes an app's name and Dock icon from the bundle it runs
		// from, so in dev the host runs out of a throwaway one. It still
		// loads the app from the build directory, not from the copy inside the bundle.
		if (process.platform === 'darwin' && name) {
			const made = spawnSync(bin, ['--bundle', out_dir, resolve(root, 'node_modules/.sepal')], {
				encoding: 'utf8'
			});
			if (made.status === 0) {
				const bundle = made.stdout.trim();
				bin = join(bundle, 'Contents/MacOS', basename(bundle, '.app'));
			} else {
				console.warn(`sepal: running unbundled: ${made.stderr.trim()}`);
			}
		}

		const started = bin;
		host = spawn(started, ['--watch', out_dir], { stdio: ['pipe', 'inherit', 'inherit'] });
		host.on('error', (error) => {
			console.error(`sepal: cannot start ${started}: ${error.message}`);
			process.exit(1);
		});
		// Closing the window ends the session. The other direction needs no
		// code: when the build tool exits, the host's stdin closes and it quits.
		host.on('exit', (code) => process.exit(code ?? 0));
	};
}
