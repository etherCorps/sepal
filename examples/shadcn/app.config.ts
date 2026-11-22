import { defineConfig } from '@ethercorps/svelte-blitz-vite';

export default defineConfig({
	app: { name: 'shadcn probe', id: 'com.example.shadcn-probe', window: { width: 900, height: 700 } },
	// Bits UI, under shadcn-svelte, is written for a browser.
	dom: true
});
