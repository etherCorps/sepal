import { defineConfig } from 'sepal/vite';

export default defineConfig({
	app: {
		name: 'Desktop APIs',
		id: 'com.example.desktop-apis',
		version: '0.1.0',
		window: { width: 640, height: 520 }
	},

	// Everything the app does outside its window has to be listed here.
	permissions: {
		// Which servers fetch() may talk to.
		http: ['https://api.github.com'],
		// Which directories the file APIs may use. 'data' is the app's own.
		fs: ['data'],
		dialog: true,
		clipboard: true,
		open: true,
		window: true
	}
});
