import { add, where, thrower } from './lib/math.js';

globalThis.results = { sum: add(2, 3), meta: import.meta.url, where: where(), source: String(add) };
try {
	thrower();
} catch (error) {
	globalThis.results.stack = error.stack;
}
import('./later.js').then((module) => (globalThis.results.later = module.value));
