import { defineConfig, loadEnv } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { sepal } from 'sepal/vite';

// App settings live in sepal.config.ts. The version is repeated here
// only to show it in the UI.
export default defineConfig(({ mode }) => ({
	define: { __APP_VERSION__: JSON.stringify(loadEnv(mode, '.', '').APP_VERSION ?? '0.1.0') },
	plugins: [tailwindcss(), sepal()]
}));
