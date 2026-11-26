// Finds the calyx binary for this machine.
//
// Each platform's binary is a package of its own, `@ethercorps/calyx-<platform>`,
// listed in this package's optionalDependencies, so a package manager installs
// only the one that fits the machine. Anything that needs the host (the Vite
// preset, after a build and in dev, the CLI) asks here, so how binaries are delivered can change
// without touching the callers.

import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);

/**
 * This machine in the form package names use: `darwin-arm64`,
 * `linux-x64-gnu`, `win32-x64-msvc`.
 */
export function platform() {
	const abi = process.platform === 'linux' ? '-gnu' : process.platform === 'win32' ? '-msvc' : '';
	return `${process.platform}-${process.arch}${abi}`;
}

/** The package that holds this machine's binary. */
export function platform_package() {
	return `@ethercorps/calyx-${platform()}`;
}

/**
 * The calyx binary to run. `SEPAL_HOST` wins, for a host you built yourself;
 * otherwise it is this machine's platform package, resolved from here.
 * `version` is that package's version, or `null` for `SEPAL_HOST`.
 *
 * Throws, saying which package this machine needs and why it may be missing,
 * when there is no binary.
 *
 * @returns {{ path: string, version: string | null }}
 */
export function find_host() {
	if (process.env.SEPAL_HOST) return { path: process.env.SEPAL_HOST, version: null };

	const name = platform_package();
	const own = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));
	if (!(name in (own.optionalDependencies ?? {}))) {
		throw new Error(
			`there is no prebuilt calyx for ${platform()}. Build one from crates/host and point SEPAL_HOST at it.`
		);
	}

	let manifest;
	try {
		manifest = require.resolve(`${name}/package.json`);
	} catch {
		throw new Error(
			`this machine needs ${name}, and it is not installed. Usually either:\n` +
				'  - optional dependencies were turned off (npm --omit=optional, pnpm --no-optional,\n' +
				'    yarn --ignore-optional), or\n' +
				'  - node_modules was copied from another OS or CPU.\n' +
				`Install again on this machine with optional dependencies on, or point SEPAL_HOST at a calyx you built.`
		);
	}
	const path = join(dirname(manifest), 'bin', process.platform === 'win32' ? 'calyx.exe' : 'calyx');
	if (!existsSync(path)) {
		throw new Error(`${name} is installed but has no binary at ${path}. In this repository, run \`pnpm run build:host\`.`);
	}
	return { path, version: JSON.parse(readFileSync(manifest, 'utf8')).version };
}
