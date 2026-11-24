import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { sepal } from 'sepal/vite';

export default defineConfig({
  plugins: [tailwindcss(), sepal({
    app: {
      name: "Icons Test",
    }
	})]
});
