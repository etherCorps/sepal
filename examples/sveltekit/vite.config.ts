import { sveltekit } from '@sveltejs/kit/vite';
import adapter from 'sepal/adapter';
import { sepal, withSepal } from 'sepal/vite';
import { defineConfig } from 'vite';

// SvelteKit's own plugin, given its options through withSepal() (the
// renderer, no version polling, Sepal's rewrites) with Sepal's adapter, then
// sepal(). App settings live in sepal.config.ts.
export default defineConfig({
	plugins: [
		sveltekit(withSepal({ adapter: adapter() })),
		sepal()
	]
});
