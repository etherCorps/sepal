import { defineConfig } from 'sepal/vite';

// Earthquakes from the USGS feeds, on a map: a SvelteKit app made with
// `pnpm create sepal quakes`, with shadcn-svelte.
export default defineConfig({
	app: {
		name: 'Quakes',
		id: 'com.example.quakes',
		window: { width: 1200, height: 800 }
	},
	permissions: {
		// The feeds and the detail of each quake: one origin, nothing else.
		http: ['https://earthquake.usgs.gov'],
		// "Open on USGS" hands the event page to the browser.
		open: true,
		// <svelte:head><title> names the window after the page.
		window: true
	}
});
