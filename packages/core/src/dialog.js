// Native dialogs. Needs `permissions.dialog`.
//
//   import { open, save, confirm } from '@ethercorps/svelte-blitz/dialog';
//
//   const path = await open({ filters: [{ name: 'Images', extensions: ['png', 'jpg'] }] });
//
// A path the user picks here can be read and written with the file APIs for
// as long as the app runs, without being listed in `permissions.fs`.

import { host_call } from './host-call.js';

/** @typedef {{ name: string, extensions: string[] }} Filter */
/**
 * @typedef {{ title?: string, defaultPath?: string, filters?: Filter[] }} FileDialogOptions
 * `defaultPath` is the directory the dialog starts in.
 */

/**
 * Let the user choose a file, several files (`multiple`) or a folder (`directory`).
 * @template {FileDialogOptions & { multiple?: boolean, directory?: boolean }} Options
 * @param {Options} [options]
 * @returns {Promise<(Options extends { multiple: true } ? string[] : string) | null>} `null` if the user cancelled
 */
export function open(options) {
	return host_call('dialog.open', options ?? {});
}

/**
 * Let the user choose where to save a file.
 * @param {FileDialogOptions & { defaultName?: string }} [options]
 * @returns {Promise<string | null>} `null` if the user cancelled
 */
export function save(options = {}) {
	return host_call('dialog.save', options);
}

/** @typedef {{ title?: string, kind?: 'info' | 'warning' | 'error' }} MessageOptions */

/** Show a message with an OK button. @param {string} text @param {MessageOptions} [options] @returns {Promise<void>} */
export async function message(text, options = {}) {
	await host_call('dialog.message', { ...options, message: text });
}

/** Ask a question with OK and Cancel. @param {string} text @param {MessageOptions} [options] @returns {Promise<boolean>} `true` for OK */
export function confirm(text, options = {}) {
	return host_call('dialog.message', { ...options, message: text, confirm: true });
}
