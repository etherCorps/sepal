// Build the host for this machine and put the binary into its npm package
// (packages/host-<platform>-<arch>). Run once per platform; CI runs it in a
// matrix for a release.

import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { host_package } from '../packages/renderer/src/host.js';

const root = fileURLToPath(new URL('..', import.meta.url));
const { name, file } = host_package();
const directory = join(root, 'packages', name.replace('@ethercorps/svelte-blitz', 'host'));

if (!existsSync(join(directory, 'package.json'))) {
	console.error(`There is no package for this platform yet (${name}). Add ${directory}/package.json first.`);
	process.exit(1);
}

// The main package pins its platform packages to its own version, so a
// mismatch would publish a package nobody can install.
const version = (/** @type {string} */ path) => JSON.parse(readFileSync(join(path, 'package.json'), 'utf8')).version;
const main = version(join(root, 'packages/renderer'));
if (version(directory) !== main) {
	console.error(`${name} is ${version(directory)} but @ethercorps/svelte-blitz is ${main}.`);
	process.exit(1);
}

// `screenshot` is in the shipped binary: `svelte-blitz screenshot` needs it and it costs about 1.5 MB.
execFileSync(
	'cargo',
	['build', '--release', '--features', 'screenshot', '--manifest-path', join(root, 'crates/host/Cargo.toml')],
	{ stdio: 'inherit' }
);
copyFileSync(join(root, 'crates/host/target/release', file), join(directory, file));
console.log(`${name}@${main}: ${join(directory, file)}`);
