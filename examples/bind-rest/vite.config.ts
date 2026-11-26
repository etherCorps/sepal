import { sepal } from 'sepal/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sepal({ app: { name: 'bind-rest' } })]
});
