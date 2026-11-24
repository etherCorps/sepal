import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { sepalKit } from 'sepal/sveltekit';

// App settings live in sepal.config.ts.
export default defineConfig({
	plugins: [tailwindcss(), sepalKit()]
});
