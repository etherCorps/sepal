import { defineConfig } from '@ethercorps/svelte-blitz-vite';

export default defineConfig({
	app: {
		name: 'SvelteKit on Blitz',
		id: 'com.example.sveltekit',
		window: { width: 800, height: 600 }
	},
	// `<svelte:head><title>` sets the window's title.
	permissions: { window: true }
});
