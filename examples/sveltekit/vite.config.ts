import { svelteBlitzKit } from '@ethercorps/svelte-blitz-vite/sveltekit';
import { defineConfig } from 'vite';

// `svelteBlitzKit()` takes the place of `sveltekit()`. Settings for the app
// are in app.config.ts.
export default defineConfig({
	plugins: [svelteBlitzKit()]
});
