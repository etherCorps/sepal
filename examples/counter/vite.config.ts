import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { sepal } from 'sepal/vite';

// App settings live in sepal.config.ts.
export default defineConfig({
	plugins: [tailwindcss(), sepal()]
});
