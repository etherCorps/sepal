import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { sepalKit } from 'sepal/sveltekit';

// `sepalKit()` takes the place of `sveltekit()`; SvelteKit's own options
// go in `sepalKit({ kit: { … } })`. App settings live in sepal.config.ts.
export default defineConfig({
	plugins: [tailwindcss(), sepalKit()]
});
