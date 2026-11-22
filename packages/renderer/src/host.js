// Finds the host binary. Runs in Node (the Vite preset and the CLI use it).

import { createRequire } from 'node:module';

/**
 * The package that carries the host binary for this machine, and the binary's
 * file name in it. One package per platform, each a single file; the main
 * package lists them as optional dependencies and the package manager
 * installs the one whose `os` and `cpu` match.
 */
export function host_package() {
	const { platform, arch } = process;
	const abi = platform === 'linux' ? '-gnu' : platform === 'win32' ? '-msvc' : '';
	return {
		name: `@ethercorps/svelte-blitz-${platform}-${arch}${abi}`,
		file: platform === 'win32' ? 'svelte-blitz-host.exe' : 'svelte-blitz-host'
	};
}

/**
 * `SVELTE_BLITZ_HOST` wins, for a host you built yourself. Otherwise the
 * binary comes from this platform's package.
 *
 * @returns {string | null} `null` when there is no binary for this platform
 */
export function find_host() {
	if (process.env.SVELTE_BLITZ_HOST) return process.env.SVELTE_BLITZ_HOST;

	const { name, file } = host_package();
	try {
		return createRequire(import.meta.url).resolve(`${name}/${file}`);
	} catch {
		return null;
	}
}
