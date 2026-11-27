// Caches of what is looked up by its text: selectors, links, URLs. Each has a
// limit. A desktop app runs for days, and libraries make selectors and URLs
// with generated ids in them (Bits UI's, an app's own), so a cache without
// one grows for as long as the app runs.
//
// For module-level caches only: every cache made here is kept for
// `__sb.stats()`, so one made per component or element would never be freed.

/** The caches made so far, by name, for `__sb.stats()`. @type {Map<string, Map<any, any>>} */
const made = new Map();

/**
 * A least-recently-used cache of at most `limit` entries, in a Map's order:
 * a hit moves its entry to the end, and past the limit the oldest goes.
 * `undefined` is never stored: it is what a miss returns.
 * @template K, V
 * @param {string} name for `cache_sizes`
 * @param {number} limit
 */
export function lru(name, limit) {
	/** @type {Map<K, V>} */
	const map = new Map();
	made.set(name, map);
	return {
		/** @param {K} key @returns {V | undefined} */
		get(key) {
			const value = map.get(key);
			if (value === undefined) return undefined;
			map.delete(key);
			map.set(key, value);
			return value;
		},
		/** @param {K} key @param {V} value */
		set(key, value) {
			map.delete(key);
			map.set(key, value);
			if (map.size > limit) map.delete(/** @type {K} */ (map.keys().next().value));
		}
	};
}

/** How many entries each cache holds, by name. */
export function cache_sizes() {
	return Object.fromEntries([...made].map(([name, map]) => [name, map.size]));
}
