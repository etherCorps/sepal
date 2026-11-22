// `$app/navigation`. `back` and `forward` are additions: SvelteKit apps use
// `history.back()`, and there is no `history` here.
export { goto, back, forward } from './routing.svelte.js';
