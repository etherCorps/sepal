import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteBlitz } from '@ethercorps/svelte-blitz-vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		svelteBlitz({
			app: {
				name: '__NAME__',
				// Change this to an identifier you own before you ship.
				id: '__ID__',
				version: '0.1.0',
				icon: 'icon.png',
				window: { width: 800, height: 600 }
			}
		})
	]
});
