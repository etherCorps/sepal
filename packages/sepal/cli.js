#!/usr/bin/env node
// The sepal command: run, bundle and screenshot an app.
// Building is Vite's job (`vite build`, and `vite build --watch` for dev).

import { spawnSync } from 'node:child_process';
import { find_host, platform } from '@ethercorps/calyx';

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
	const bin = find_host();
	if (bin === null) {
		fail(
			`sepal: no calyx (host binary) for ${platform()}.\n` +
				'Build one from crates/host and point SEPAL_HOST at it.'
		);
	}
	const { status, error } = spawnSync(bin, args, { stdio: 'inherit' });
	if (error) fail(`sepal: cannot start ${bin}: ${error.message}`);
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
