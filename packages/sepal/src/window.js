// The app's window. Everything but the title needs `permissions.window`.
//
// Title, size and icon at startup come from the app config; these change the
// window while the app runs.

import { host_call } from './host-call.js';

/**
 * Set the window's title. Needs no permission: the title is the app's own,
 * as a page's is on the web (`document.title` does the same).
 * @param {string} title @returns {Promise<void>}
 */
export function setTitle(title) {
	return host_call('window.set_title', { title });
}

/** @returns {Promise<void>} */
export function minimize() {
	return host_call('window.minimize');
}

/** @returns {Promise<void>} */
export function maximize() {
	return host_call('window.set_maximized', { maximized: true });
}

/** @returns {Promise<void>} */
export function unmaximize() {
	return host_call('window.set_maximized', { maximized: false });
}

/** @returns {Promise<boolean>} */
export function isMaximized() {
	return host_call('window.is_maximized');
}

/** Show or hide the title bar and borders. @param {boolean} decorations @returns {Promise<void>} */
export function setDecorations(decorations) {
	return host_call('window.set_decorations', { decorations });
}

/**
 * Start moving the window with the mouse. Call it from a `pointerdown`
 * handler on whatever acts as your title bar.
 * @returns {Promise<void>}
 */
export function startDragging() {
	return host_call('window.start_dragging');
}

/** @returns {Promise<void>} */
export function close() {
	return host_call('window.close');
}
