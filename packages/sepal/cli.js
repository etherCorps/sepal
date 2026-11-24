#!/usr/bin/env node
// The sepal command: run, bundle and screenshot an app.
// Building is Vite's job (`vite build`, and `vite build --watch` for dev).

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
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

/** Run the host with these arguments and exit with its status. @param {string[]} args @returns {never} */
function host(args) {
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
	const { status, error } = spawnSync(found.path, args, { stdio: 'inherit' });
	if (error) fail(`sepal: cannot start ${found.path}: ${error.message}`);
	process.exit(status ?? 1);
}

const [command, ...args] = process.argv.slice(2);
// `bundle --sign x` with no directory still means `dist`.
const dist = args[0] !== undefined && !args[0].startsWith('--') ? args : ['dist', ...args];

switch (command) {
	case 'run':
		host(dist);
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
