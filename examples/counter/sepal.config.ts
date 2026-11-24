import { defineConfig } from 'sepal/vite';

export default defineConfig({
	app: {
		name: 'Counter',
		id: 'com.example.counter',
		version: '0.1.0',
		window: { title: 'Svelte on Blitz', width: 800, height: 600 }
	}
});
