// Shims must be evaluated before Svelte's runtime.
import { run_timers, next_timer } from './shims.js';
import { mount as svelte_mount, unmount } from 'svelte';
import renderer, { root, dispatch, take_batch, stats, keep_alive } from './renderer.js';
import { settle_call } from './host-call.js';
import { fetch } from './fetch.js';

/**
 * Mount a component into the window.
 *
 * @template {Record<string, any>} Props
 * @template {Record<string, any>} Exports
 * @param {import('svelte').Component<Props, Exports, any>} component
 * @param {{ props?: Props, context?: Map<any, any> }} [options]
 * @returns {Exports} the component's exports; pass it to `unmount`
 */
export function mount(component, options = {}) {
	// Svelte's own signature makes `props` required when the component has
	// required props; ours keeps it optional, hence the cast.
	return svelte_mount(component, /** @type {any} */ ({ ...options, target: root, renderer }));
}

export { unmount, renderer, root };

// The engine has no `fetch`. This one asks the host, which checks the app's permissions.
if (typeof (/** @type {any} */ (globalThis).fetch) !== 'function') /** @type {any} */ (globalThis).fetch = fetch;

// The host's entry points into JS. It calls these and nothing else.
/** @type {any} */ (globalThis).__sb = {
	/** Deliver a DOM event. Returns DISPATCH_* flags. */
	dispatch,
	/** Take the pending op batch, or null if nothing changed. */
	take: take_batch,
	/** Run due timers. */
	timers: run_timers,
	/** The next timer deadline on the host clock, or -1. Read at the end of a turn. */
	next_timer,
	/** Answer a call the app made to the host (see host-call.js). */
	resolve: settle_call,
	/** Node counters, for tests and leak hunting. */
	stats,
	/** Pins the root node and the finalizer registry for the life of the app. */
	keep_alive
};
