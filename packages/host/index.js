// The host binaries, and the lookup that picks the one for this machine.
//
// Every platform's binary ships in this one package, under bin/<platform>/.
// Anything that needs the host (the Vite preset in dev, the CLI) asks here, so
// how binaries are delivered can change without touching the callers.

import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

/**
 * The name of this machine's binary directory: `darwin-arm64`,
 * `linux-x64-gnu`, `win32-x64-msvc`.
 */
export function platform() {
	const abi = process.platform === 'linux' ? '-gnu' : process.platform === 'win32' ? '-msvc' : '';
	return `${process.platform}-${process.arch}${abi}`;
}

/** Where this machine's binary is in this package, whether or not it has been built. */
export function bundled_host() {
	const file = process.platform === 'win32' ? 'svelte-blitz-host.exe' : 'svelte-blitz-host';
	return fileURLToPath(new URL(`./bin/${platform()}/${file}`, import.meta.url));
}

/**
 * `SVELTE_BLITZ_HOST` wins, for a host you built yourself. Otherwise the
 * binary comes from this package.
 *
 * @returns {string | null} `null` when there is no binary for this platform
 */
export function find_host() {
	if (process.env.SVELTE_BLITZ_HOST) return process.env.SVELTE_BLITZ_HOST;
	const path = bundled_host();
	return existsSync(path) ? path : null;
}
