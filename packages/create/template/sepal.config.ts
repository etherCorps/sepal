import { defineConfig } from 'sepal/vite';

export default defineConfig({
	app: {
		name: '__NAME__',
		// Change this to an identifier you own before you ship.
		id: '__ID__',
		// The icon is static/icon.png.
		window: { width: 800, height: 600 }
		// Self-update: see "Updates" in the Sepal README.
		// updater: { url: 'https://example.com/latest.json', pubkey: '…' }
	},

	// What the app may do outside its window. Everything is off until listed.
	permissions: {
		// http: ['https://api.example.com'],
		// fs: ['data'],
		// dialog: true,
		// clipboard: true,
		// open: true,
		// window: true
	}
});
