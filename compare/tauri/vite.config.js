// The Tracker built for the DOM (WKWebView): SvelteKit as a single-page app,
// from the app's own source and with the app's own toolchain.
//
// Mirrors sepalKit() (packages/sepal/vite/sveltekit.js) with the
// renderer-specific parts taken out: no `customRenderer`, no
// `svelte-custom-renderer` condition, no `bind:`/`<svelte:head>`
// preprocessors (Svelte handles both for the DOM), no DOM layer, and
// adapter-static in place of Sepal's adapter. The SvelteKit options are
// sepalKit()'s: one bundle, nothing prerendered, no version polling.
//
// `pnpm run build:web` runs it from examples/tracker, so Vite's root,
// SvelteKit's project and Tailwind's scan are the app's. Vite, SvelteKit,
// vite-plugin-svelte and Tailwind are the installs the app's own build uses:
// both builds run one copy of SvelteKit's router.

import { fileURLToPath } from 'node:url';
import adapter from '@sveltejs/adapter-static';
// Linked from examples/tracker/node_modules (package.json): the app's installs.
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';

const here = (file) => fileURLToPath(new URL(file, import.meta.url));

const dist = here('./dist');
const bench_emit = here('./src/bench-emit.js');

export default {
	// Keep Vite's cache out of examples/tracker.
	cacheDir: here('./node_modules/.vite'),
	plugins: [
		tailwindcss(),
		{
			// First in SvelteKit's single-bundle entry, where sepalKit() puts the
			// DOM layer: `__bench_emit` has to exist before bench.ts marks `start`.
			name: 'tracker-tauri:bench-emit',
			transform: (code, id) =>
				id.endsWith('/kit/src/runtime/client/bundle.js') ? { code: `import ${JSON.stringify(bench_emit)};\n${code}`, map: null } : undefined
		},
		await sveltekit({
			adapter: adapter({ pages: dist, assets: dist, fallback: 'index.html' }),
			// Keep SvelteKit's generated files apart from the Sepal build's.
			outDir: here('./.svelte-kit'),
			output: { bundleStrategy: 'single' },
			prerender: { entries: [] },
			version: { pollInterval: 0 }
		})
	],
	resolve: {
		// sepalKit()'s list minus `svelte-custom-renderer`.
		conditions: ['browser', 'svelte', 'module', 'import', 'default'],
		alias: [{ find: /^sepal\/(fs|dialog|clipboard|window)$/, replacement: here('./src/desktop.js') }]
	}
};
