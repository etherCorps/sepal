// The router's runtime API in one place:
//
//   import { error, redirect, goto, page } from 'sepal/router';
//
// `$app/navigation` and `$app/state` are the SvelteKit-style names for parts of it.

export { error, redirect, goto, back, forward, page } from './routing.svelte.js';

/**
 * For typing `load` functions, and for writing a router of your own against
 * `virtual:sepal/routes`.
 *
 * @typedef {import('./routing.svelte.js').LoadEvent} LoadEvent
 * @typedef {import('./routing.svelte.js').PageUrl} PageUrl
 * @typedef {import('./routing.svelte.js').Route} Route
 * @typedef {import('./routing.svelte.js').RouteNode} RouteNode
 */
