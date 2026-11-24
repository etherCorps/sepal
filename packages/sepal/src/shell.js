// Hand something to another app. Needs `permissions.open`.

import { host_call } from './host-call.js';

/**
 * Open a web or mail link (`https:`, `http:`, `mailto:`) in the default app,
 * or a file the app is allowed to read. Other kinds of links are refused.
 * @param {string} target a link or an absolute path
 * @returns {Promise<void>}
 */
export function open(target) {
	return host_call('open.open', { target });
}
