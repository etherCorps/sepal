// `sepalKit()` is gone. A SvelteKit app uses SvelteKit's own plugin, with
// `withSepal()`, `sepal/adapter` and `sepal()`; this says how to move.
//
// ponytail: remove a release after 0.2.0-alpha.0.
//
// Runs in Node.

import { KIT_SETUP } from './vite.js';

/**
 * @deprecated Use SvelteKit's `sveltekit(withSepal({ adapter: adapter() }))`,
 * with `adapter` from `sepal/adapter`, and `sepal()` from `sepal/vite`.
 * @param {Record<string, unknown>} [_options]
 * @returns {never}
 */
export function sepalKit(_options) {
	throw new Error(
		`sepal: sepalKit() is gone. A SvelteKit app uses SvelteKit's own plugin, given its options through withSepal(), with Sepal's adapter, and sepal(). In vite.config.ts:\n\n${KIT_SETUP}\n\nWhat went to sepalKit({ kit }) goes to withSepal({ … }) as it is, and \`out\` to adapter({ out }).`
	);
}
