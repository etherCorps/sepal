import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteBlitz } from '@ethercorps/svelte-blitz-vite';

// App settings live in svelte-blitz.config.ts.
export default defineConfig({
	plugins: [tailwindcss(), svelteBlitz()]
});
