// Shims must be evaluated before Svelte's runtime.
import { run_timers, next_timer, run_frame, frame_pending, track_rejection, report_rejections, rejection_count } from './shims.js';
import { mount as svelte_mount, unmount } from 'svelte';
import renderer, { root, dispatch, take_batch, stats, keep_alive, host_hooks } from './renderer.js';
import { settle_call } from './host-call.js';
import { fetch, Headers, Request, Response } from './fetch.js';

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

// The engine has no `fetch`. This one asks the host, which checks the app's
// permissions. The classes around it come along: libraries test for them.
for (const [name, value] of Object.entries({ fetch, Headers, Request, Response })) {
	if (typeof (/** @type {any} */ (globalThis)[name]) !== 'function') /** @type {any} */ (globalThis)[name] = value;
}

// The host's entry points into JS. It calls these and nothing else.
/** @type {any} */ (globalThis).__sb = {
	/** Deliver a DOM event. Returns DISPATCH_* flags. */
	dispatch,
	/** Take the pending op batch, or null if nothing changed. */
	take: take_batch,
	/** Run one due timer; `false` when none is left. Called until it says so. */
	timers: run_timers,
	/** The next timer deadline on the host clock, or -1. Read at the end of a turn. */
	next_timer,
	/** Run one `requestAnimationFrame` callback; `false` when this frame has none left. Called until it says so. */
	frame: run_frame,
	/** Whether a `requestAnimationFrame` callback waits for a frame. Read at the end of a turn. */
	frame_pending,
	/** Answer a call the app made to the host (see host-call.js). */
	resolve: settle_call,
	/**
	 * The viewport size and the boxes that changed, `[id, x, y, width, height, …]`,
	 * for the nodes the app measures. Only called once it has measured one.
	 * @param {number} width @param {number} height @param {number[]} boxes
	 */
	layout: (width, height, boxes) => host_hooks.layout?.(width, height, boxes),
	/** A promise was rejected with no handler, or one was added since. */
	rejection: track_rejection,
	/** Report the promises that are still rejected and unhandled. */
	rejections: report_rejections,
	/** Node counters, for tests and leak hunting. */
	stats: () => ({ ...stats(), rejections: rejection_count() }),
	/** Pins the root node and the finalizer registry for the life of the app. */
	keep_alive
};
