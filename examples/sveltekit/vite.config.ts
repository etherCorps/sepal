import { sepalKit } from 'sepal/sveltekit';
import { defineConfig } from 'vite';

// `sepalKit()` takes the place of `sveltekit()`. Settings for the app
// are in sepal.config.ts.
export default defineConfig({
	plugins: [sepalKit()]
});
