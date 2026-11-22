import { defineConfig, loadEnv } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { svelteBlitz } from '@ethercorps/svelte-blitz-vite';

// App settings live in app.config.ts. The version is repeated here
// only to show it in the UI.
export default defineConfig(({ mode }) => ({
	define: { __APP_VERSION__: JSON.stringify(loadEnv(mode, '.', '').APP_VERSION ?? '0.1.0') },
	plugins: [tailwindcss(), svelteBlitz()]
}));
