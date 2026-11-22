// Files.
//
//   import { readTextFile, writeTextFile } from '@ethercorps/svelte-blitz/fs';
//
//   await writeTextFile('settings.json', JSON.stringify(settings), { dir: 'data' });
//   const text = await readTextFile('/Users/me/notes.txt');
//
// A path is absolute, or relative to a named directory given as `dir`:
// `'data'` (the app's own data directory), `'documents'`, `'downloads'`,
// `'desktop'` or `'home'`. The app's `permissions.fs` decides which
// directories may be used; a file the user picked in a dialog always may.

import { host_call } from './host-call.js';
import { decode, encode, to_bytes } from './base64.js';

/** @typedef {'data' | 'documents' | 'downloads' | 'desktop' | 'home'} Directory */
/** @typedef {{ dir?: Directory }} PathOptions */

/** @param {string} path @param {PathOptions} [options] @returns {Promise<string>} */
export function readTextFile(path, options = {}) {
	return host_call('fs.read_text', { path, dir: options.dir });
}

/** @param {string} path @param {PathOptions} [options] @returns {Promise<Uint8Array>} */
export async function readFile(path, options = {}) {
	return decode(await host_call('fs.read', { path, dir: options.dir }));
}

/**
 * Write a text file, replacing it if it exists. Missing parent directories are created.
 * @param {string} path @param {string} text @param {PathOptions} [options] @returns {Promise<void>}
 */
export function writeTextFile(path, text, options = {}) {
	return host_call('fs.write_text', { path, text, dir: options.dir });
}

/**
 * Write bytes, replacing the file if it exists. Missing parent directories are created.
 * @param {string} path @param {ArrayBuffer | ArrayBufferView} bytes @param {PathOptions} [options] @returns {Promise<void>}
 */
export function writeFile(path, bytes, options = {}) {
	return host_call('fs.write', { path, base64: encode(to_bytes(bytes)), dir: options.dir });
}

/** @param {string} path @param {PathOptions} [options] @returns {Promise<boolean>} */
export function exists(path, options = {}) {
	return host_call('fs.exists', { path, dir: options.dir });
}

/** Create a directory and any missing parents. @param {string} path @param {PathOptions} [options] @returns {Promise<void>} */
export function mkdir(path, options = {}) {
	return host_call('fs.mkdir', { path, dir: options.dir });
}

/** @param {string} path @param {PathOptions} [options] @returns {Promise<{ name: string, isDirectory: boolean }[]>} sorted by name */
export function readDir(path, options = {}) {
	return host_call('fs.read_dir', { path, dir: options.dir });
}

/**
 * Delete a file or an empty directory. `recursive` deletes a directory with everything in it.
 * @param {string} path @param {PathOptions & { recursive?: boolean }} [options] @returns {Promise<void>}
 */
export function remove(path, options = {}) {
	return host_call('fs.remove', { path, dir: options.dir, recursive: options.recursive });
}

/** Where a named directory is on this machine. Needs no permission. @param {Directory} name @returns {Promise<string>} */
export function dir(name) {
	return host_call('fs.dir', { name });
}
