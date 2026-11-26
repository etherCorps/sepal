// What every way of building an app shares, whichever router it uses: reading
// `sepal.config.ts`, writing the manifest the host reads (`app.json`),
// compiling the modules to bytecode, and opening the app in dev and
// reloading it after each rebuild.
//
// Runs in Node.

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadConfigFromFile } from 'vite';
import { find_host } from '@ethercorps/calyx';

const CONFIG_FILES = ['sepal.config.ts', 'sepal.config.js', 'sepal.config.mjs'];

/**
 * What `sepal()` tells the SvelteKit adapter (`sepal/adapter`), which runs
 * inside SvelteKit's build and has no other way to reach it: the project,
 * the settings from `sepal.config.ts`, and what to do once the app is
 * written to a directory (open or reload the dev window). `null` until
 * `sepal()` has run.
 * @type {{ session: { root: string, options: import('./vite.js').Config, written: (out_dir: string) => void } | null }}
 */
export const kit = { session: null };

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

/** The Sepal app icon, for an app that has none of its own. A byte copy of assets/brand/sepal-app-icon-1024.png. */
export const SEPAL_ICON = fileURLToPath(new URL('./icon.png', import.meta.url));

/**
 * The contents of `app.json`, or `null` when the config says nothing the host
 * needs. `icon` is a file to write into the build as `icon.png`: the Sepal
 * icon, when the app has no icon of its own. Throws with a message for the
 * user when the icon is not usable.
 * @param {import('./vite.js').Config} options
 * @param {string} root the project
 * @param {string} static_dir where the icon is looked for; empty when there is none
 * @returns {{ json: string, icon: string | null } | null}
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
	let icon = null;
	if (app.icon === undefined) {
		// No icon of the app's own: the build gets the Sepal icon, under the
		// name a static one would have had.
		app.icon = 'icon.png';
		icon = SEPAL_ICON;
	} else {
		// The host decodes PNG only; fail here rather than when the app starts.
		if (!app.icon.toLowerCase().endsWith('.png')) throw new Error(`app.icon must be a PNG file, got ${app.icon}`);
		if (!static_dir || !existsSync(join(static_dir, app.icon))) {
			throw new Error(`app.icon: there is no ${app.icon} in the static directory (${static_dir || 'turned off'})`);
		}
	}
	const json = JSON.stringify(options.permissions ? { ...app, permissions: options.permissions } : app, null, '\t') + '\n';
	return { json, icon };
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

		let bin;
		try {
			bin = find_host().path;
		} catch (error) {
			console.warn(`sepal: the app will not open: ${/** @type {Error} */ (error).message}`);
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

/** What the host names a module's bytecode: the module's file name and this. */
const BYTECODE = '.qbc';

/**
 * Compile the app's modules to bytecode with this machine's calyx (`calyx
 * --compile`), which writes `x.js.qbc` next to each `x.js`: the host then
 * loads them without parsing them. Only what changed since the last time
 * is compiled. Without a calyx that can, the app loads its JavaScript, as it
 * always could: the build says so, and goes on.
 * @param {string} dir the app folder
 * @param {(message: string) => void} warn
 * @returns {string | null} calyx's summary, or null when nothing was compiled
 */
export function compile_bytecode(dir, warn) {
	let bin;
	try {
		bin = find_host().path;
	} catch (error) {
		remove_bytecode(dir);
		warn(`sepal: the app's modules were not compiled to bytecode, so it loads their JavaScript: ${/** @type {Error} */ (error).message}`);
		return null;
	}
	const done = spawnSync(bin, ['--compile', dir], { encoding: 'utf8' });
	if (done.error || done.status !== 0) {
		remove_bytecode(dir);
		const why = done.error?.message ?? done.stderr.trim();
		warn(`sepal: compiling the app's modules to bytecode failed, so it loads their JavaScript: ${why}`);
		return null;
	}
	// Modules that stayed JavaScript, one line each.
	for (const line of done.stderr.split('\n').filter(Boolean)) warn(`sepal: ${line}`);
	return done.stdout.trim() || null;
}

/**
 * The last build's bytecode, kept for a calyx that then could not compile:
 * next to modules rebuilt since, none of it may ship. Outside dev the host
 * compares only a module's size with what its bytecode was compiled from.
 * @param {string} dir
 */
function remove_bytecode(dir) {
	for (const file of keep_bytecode(dir).keys()) rmSync(join(dir, file));
}

/**
 * Empty the app folder for a new build, but keep the last build's bytecode:
 * `compile_bytecode` then compiles only the modules that changed, and
 * removes the bytecode of those that are gone.
 * @param {string} dir
 */
export function clear_but_bytecode(dir) {
	if (!existsSync(dir)) return;
	for (const entry of readdirSync(dir, { withFileTypes: true })) {
		const path = join(dir, entry.name);
		if (entry.isDirectory()) {
			clear_but_bytecode(path);
			if (readdirSync(path).length === 0) rmSync(path, { recursive: true });
		} else if (!entry.name.endsWith(BYTECODE)) {
			rmSync(path);
		}
	}
}

/**
 * The bytecode in the app folder, by path, read before a bundler empties it
 * (Vite does, before every build): `restore_bytecode` puts it back after.
 * @param {string} dir
 * @returns {Map<string, Buffer>}
 */
export function keep_bytecode(dir) {
	const kept = new Map();
	if (!existsSync(dir)) return kept;
	for (const entry of readdirSync(dir, { recursive: true, withFileTypes: true })) {
		if (!entry.isFile() || !entry.name.endsWith(BYTECODE)) continue;
		const path = join(entry.parentPath, entry.name);
		kept.set(relative(dir, path), readFileSync(path));
	}
	return kept;
}

/**
 * Put back what `keep_bytecode` read, next to the modules the new build
 * wrote. Whether each still fits its module is for calyx to say.
 * @param {string} dir
 * @param {Map<string, Buffer>} kept
 */
export function restore_bytecode(dir, kept) {
	for (const [file, bytes] of kept) {
		const path = join(dir, file);
		if (existsSync(path) || !existsSync(path.slice(0, -BYTECODE.length))) continue;
		mkdirSync(dirname(path), { recursive: true });
		writeFileSync(path, bytes);
	}
}
