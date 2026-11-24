// The Tracker built for the DOM (WKWebView), from the app's own source.
//
// Mirrors packages/sepal/vite/vite.js (the Sepal preset) with the
// renderer-specific parts taken out: no `customRenderer`, no
// `svelte-custom-renderer` condition, no `bind:`/`<svelte:head>` preprocessors
// (Svelte handles both for the DOM), no DOM layer. Same root, same library
// build (one IIFE `app.js`, one `app.css`, assets inlined, es2022), so the
// Tailwind scan and the CSS are the Sepal build's.

import { fileURLToPath } from 'node:url';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { generate_routes } from '../../packages/sepal/vite/routes.js';

const here = (file) => fileURLToPath(new URL(file, import.meta.url));
const app = here('../../examples/tracker');
const preset = here('../../packages/sepal/vite');
const desktop = here('./src/desktop.js');
const ROUTES_ID = 'virtual:sepal/routes';

export default defineConfig({
	// Tailwind scans the Vite root: it must be the app's, as in its own build.
	root: app,
	// Keep Vite's cache out of examples/tracker.
	cacheDir: here('./node_modules/.vite'),
	// index.html, copied into dist as it is.
	publicDir: here('./public'),
	plugins: [
		tailwindcss(),
		svelte({ emitCss: true, compilerOptions: { css: 'external' } }),
		{
			name: 'tracker-tauri:routes',
			resolveId: (id) => (id === ROUTES_ID ? '\0' + ROUTES_ID : undefined),
			load: (id) => (id === '\0' + ROUTES_ID ? generate_routes(`${app}/src/routes`).code : undefined)
		}
	],
	resolve: {
		// Sepal's list minus `svelte-custom-renderer`.
		conditions: ['browser', 'svelte', 'module', 'import', 'default'],
		alias: [
			// The preset's router modules import their runtime by this name; give
			// them the DOM copy. Nothing in the app has a file of this name.
			{ find: /^\.\/routing\.svelte\.js$/, replacement: here('./src/routing.svelte.js') },
			{ find: '$lib', replacement: `${app}/src/lib` },
			{ find: '$app/navigation', replacement: `${preset}/navigation.js` },
			{ find: '$app/state', replacement: `${preset}/state.js` },
			{ find: 'sepal/router', replacement: `${preset}/router.js` },
			{ find: /^sepal\/(fs|dialog|clipboard|window)$/, replacement: desktop }
		]
	},
	build: {
		outDir: here('./dist'),
		emptyOutDir: true,
		target: 'es2022',
		cssCodeSplit: false,
		lib: {
			entry: here('./src/entry.js'),
			name: 'app',
			formats: ['iife'],
			fileName: () => 'app.js',
			cssFileName: 'app'
		}
	}
});
