// `@ethercorps/svelte-blitz/{fs,dialog,clipboard,window}`, the parts the Tracker
// uses, on Tauri's plugins. Same names and signatures as packages/core/src.
import { getCurrentWindow } from '@tauri-apps/api/window';
import * as clipboard from '@tauri-apps/plugin-clipboard-manager';
import * as dialog from '@tauri-apps/plugin-dialog';
import * as fs from '@tauri-apps/plugin-fs';

/** @param {{ dir?: string }} options */
function base(options) {
	if (options.dir === undefined) return {};
	// ponytail: the app only uses `dir: 'data'`; map the others if it ever does.
	if (options.dir !== 'data') throw new Error(`desktop shim: dir ${options.dir} is not mapped`);
	return { baseDir: fs.BaseDirectory.AppData };
}

/** @param {string} path @param {{ dir?: string }} [options] */
export const readTextFile = (path, options = {}) => fs.readTextFile(path, base(options));

/** @param {string} path @param {string} text @param {{ dir?: string }} [options] */
export const writeTextFile = (path, text, options = {}) => fs.writeTextFile(path, text, base(options));

/** @param {string} path @param {{ dir?: string, recursive?: boolean }} [options] */
export const remove = (path, options = {}) => fs.remove(path, { ...base(options), recursive: options.recursive });

/** svelte-blitz's `defaultPath` is the directory, `defaultName` the file name. */
export function save(options = {}) {
	const { defaultName, defaultPath, ...rest } = options;
	const path = defaultPath && defaultName ? `${defaultPath}/${defaultName}` : (defaultName ?? defaultPath);
	return dialog.save({ ...rest, defaultPath: path });
}

export const writeText = (text) => clipboard.writeText(text);

export const setTitle = (title) => getCurrentWindow().setTitle(title);
