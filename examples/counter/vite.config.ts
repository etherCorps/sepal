import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteBlitz } from '@ethercorps/svelte-blitz/vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		svelteBlitz({
			app: {
				name: 'Counter',
				id: 'com.example.counter',
				version: '0.1.0',
				icon: 'icon.png',
				window: { title: 'Svelte on Blitz', width: 800, height: 600 }
			}
		})
	]
});
