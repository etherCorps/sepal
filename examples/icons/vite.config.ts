import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteBlitz } from '@ethercorps/svelte-blitz/vite';

export default defineConfig({
	plugins: [tailwindcss(), svelteBlitz()]
});
