import { defineConfig } from '@ethercorps/svelte-blitz-vite';

export default defineConfig({
	app: {
		name: 'Counter',
		id: 'com.example.counter',
		version: '0.1.0',
		icon: 'icon.png',
		window: { title: 'Svelte on Blitz', width: 800, height: 600 }
	}
});
