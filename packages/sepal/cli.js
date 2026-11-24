#!/usr/bin/env node
// The sepal command: run, bundle and screenshot an app.
// Building is Vite's job (`vite build`, and `vite build --watch` for dev).

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { find_host, platform_package } from '@ethercorps/calyx';

const own = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));

const USAGE = `usage:
  sepal run [dist]                          open the built app in a window
  sepal bundle [dist] [out-dir] [--sign <identity>] [--notarize <keychain-profile>]
               [--update-key <private-key-file>] [--update-notes <text>]
                                            build a macOS .app; with a key, also the signed update
  sepal signer generate <private-key-file>  make the key pair updates are signed with
  sepal screenshot <out.png> [dist] [WIDTHxHEIGHT]

Build first with \`vite build\`. For dev, \`vite build --watch\` opens the app
and reloads it on every save.
To start a project: pnpm create sepal my-app`;

/** @param {string} message @returns {never} */
function fail(message) {
	console.error(message);
	process.exit(1);
}

/** This machine's calyx, once its version is known to fit. @returns {string} */
function calyx() {
	/** @type {ReturnType<typeof find_host>} */
	let found;
	try {
		found = find_host();
	} catch (error) {
		fail(`sepal: ${/** @type {Error} */ (error).message}`);
	}
	// The pins are exact (sepal -> calyx -> the platform package), so this only
	// happens to a node_modules put together by hand or half updated. A
	// calyx from SEPAL_HOST is the user's choice and is not checked.
	if (found.version !== null && found.version !== own.version) {
		fail(
			`sepal: this is sepal ${own.version}, but its calyx binary (${platform_package()}) is ${found.version}.\n` +
				`They have to be the same version. Reinstall sepal@${own.version} (or remove node_modules and install\n` +
				'again). To run a different calyx on purpose, point SEPAL_HOST at it.'
		);
	}
	return found.path;
}

/** Run a binary with these arguments and exit with its status. @param {string} bin @param {string[]} args @returns {never} */
function start(bin, args) {
	const { status, error } = spawnSync(bin, args, { stdio: 'inherit' });
	if (error) fail(`sepal: cannot start ${bin}: ${error.message}`);
	process.exit(status ?? 1);
}

/** Run the host with these arguments and exit with its status. @param {string[]} args @returns {never} */
function host(args) {
	start(calyx(), args);
}

/**
 * macOS takes the name in the Dock, the app switcher and Activity Monitor
 * from the bundle a process runs from; a bare calyx is called "calyx". So on
 * macOS an app with a name runs out of a throwaway bundle in
 * node_modules/.sepal, as it does in dev, and still loads from `dir`.
 * @param {string} bin @param {string} dir
 */
function bundled(bin, dir) {
	if (process.platform !== 'darwin') return bin;
	let name;
	try {
		name = JSON.parse(readFileSync(join(dir, 'app.json'), 'utf8')).name;
	} catch {
		return bin;
	}
	if (!name) return bin;
	const made = spawnSync(bin, ['--bundle', dir, resolve('node_modules/.sepal')], { encoding: 'utf8' });
	if (made.status !== 0) {
		console.warn(`sepal: running unbundled: ${made.stderr.trim()}`);
		return bin;
	}
	const bundle = made.stdout.trim();
	return join(bundle, 'Contents/MacOS', basename(bundle, '.app'));
}

const [command, ...args] = process.argv.slice(2);
// `bundle --sign x` with no directory still means `dist`.
const dist = args[0] !== undefined && !args[0].startsWith('--') ? args : ['dist', ...args];

switch (command) {
	case 'run':
		start(bundled(calyx(), dist[0]), dist);
	case 'bundle':
		host(['--bundle', ...dist]);
	case 'signer':
		if (args[0] !== 'generate' || !args[1]) fail(USAGE);
		host(['--updater-keygen', args[1]]);
	case 'screenshot':
		if (!args[0]) fail(USAGE);
		host(['--screenshot', args[0], ...(args.length > 1 ? args.slice(1) : ['dist'])]);
	default:
		fail(USAGE);
}
