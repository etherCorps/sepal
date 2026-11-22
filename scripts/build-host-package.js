// Build the host for this machine and put the binary where the host package
// ships it: packages/host/bin/<platform>/. Run once per platform; a release
// collects all of them into the one package.

import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { bundled_host, platform } from '../packages/host/index.js';

const root = fileURLToPath(new URL('..', import.meta.url));
const target = bundled_host();

// `screenshot` is in the shipped binary: `svelte-blitz screenshot` needs it and it costs about 1.5 MB.
execFileSync(
	'cargo',
	['build', '--release', '--features', 'screenshot', '--manifest-path', join(root, 'crates/host/Cargo.toml')],
	{ stdio: 'inherit' }
);
mkdirSync(dirname(target), { recursive: true });
copyFileSync(join(root, 'crates/host/target/release', basename(target)), target);
console.log(`${platform()}: ${target}`);

// The notices of everything in that binary travel with it.
await import('./third-party-licenses.js');
