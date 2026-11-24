// The clipboard, text only. Needs `permissions.clipboard`.

import { host_call } from './host-call.js';

/** @returns {Promise<string>} rejects when the clipboard holds no text */
export function readText() {
	return host_call('clipboard.read_text');
}

/** @param {string} text @returns {Promise<void>} */
export function writeText(text) {
	return host_call('clipboard.write_text', { text });
}
