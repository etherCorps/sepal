import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import adapter from 'sepal/adapter';
import { sepal } from 'sepal/vite';
import { defineConfig } from 'vite';

// SvelteKit's own plugin, with Sepal's adapter, then sepal(). App settings
// live in sepal.config.ts.
export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			adapter: adapter(),
			compilerOptions: { experimental: { customRenderer: 'sepal/renderer' } },
			version: { pollInterval: 0 }
		}),
		sepal()
	]
});
