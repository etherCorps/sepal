import { defineConfig } from '@ethercorps/svelte-blitz-vite';

export default defineConfig({
	app: {
		name: '__NAME__',
		// Change this to an identifier you own before you ship.
		id: '__ID__',
		icon: 'icon.png',
		window: { width: 800, height: 600 }
		// Self-update: see "Updates" in the svelte-blitz README.
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
