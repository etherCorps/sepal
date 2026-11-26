// Build the host for this machine and put it where its platform package ships
// it: packages/calyx-<platform>/bin/calyx, with THIRD-PARTY-LICENSES.txt next
// to bin/. Run once per platform; a release builds each platform's on its own
// runner.

import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { platform } from '../packages/host/index.js';

const root = fileURLToPath(new URL('..', import.meta.url));
const dir = join(root, 'packages', `calyx-${platform()}`);
if (!existsSync(join(dir, 'package.json'))) {
	throw new Error(`there is no platform package for ${platform()} (packages/calyx-${platform()})`);
}
const file = process.platform === 'win32' ? 'calyx.exe' : 'calyx';

// `screenshot` is in the shipped binary: `sepal screenshot` needs it and it costs about 1.5 MB.
// The `dist` profile is release with fat LTO in one codegen unit (crates/host/Cargo.toml).
execFileSync(
	'cargo',
	['build', '--profile', 'dist', '--features', 'screenshot', '--manifest-path', join(root, 'crates/host/Cargo.toml')],
	{ stdio: 'inherit' }
);
mkdirSync(join(dir, 'bin'), { recursive: true });
copyFileSync(join(root, 'crates/host/target/dist', file), join(dir, 'bin', file));
console.log(`${platform()}: ${join(dir, 'bin', file)}`);

// The notices of everything in that binary travel with it.
await import('./third-party-licenses.js');
