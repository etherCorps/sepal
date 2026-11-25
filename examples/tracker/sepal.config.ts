import { defineConfig } from 'sepal/vite';

// The benchmark app: docs/tracker-plan.md. crates/host/tests/tracker drives it.
// Bits UI, under shadcn-svelte, is written for a browser: `sepalKit()` loads
// the DOM layer for it, as for every SvelteKit app.
export default defineConfig({
	app: { name: 'Tracker', id: 'com.example.tracker', window: { width: 1200, height: 800 } },
	permissions: {
		// edits.json and settings.json live in the app's own data directory.
		fs: ['data'],
		window: true,
		clipboard: true,
		dialog: true
	}
});
