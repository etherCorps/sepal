import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteBlitzKit } from '@ethercorps/svelte-blitz-vite/sveltekit';

// `svelteBlitzKit()` takes the place of `sveltekit()`; SvelteKit's own options
// go in `svelteBlitzKit({ kit: { … } })`. App settings live in app.config.ts.
export default defineConfig({
	plugins: [tailwindcss(), svelteBlitzKit()]
});
