import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin } from 'vite';

/**
 * What SvelteKit needs to run on the svelte-blitz renderer, done to its client
 * code as it is bundled. Spike: this reaches into SvelteKit's own files, and
 * any SvelteKit release may move them.
 */
function blitz(): Plugin {
	return {
		name: 'svelte-blitz:sveltekit',
		enforce: 'pre',
		transform(code, id) {
			// The entry of a single-bundle build: load the DOM and the runtime
			// (`__sb`, the host's way in) before any of SvelteKit runs.
			if (id.endsWith('/kit/src/runtime/client/bundle.js')) {
				// SvelteKit starts without waiting, and a failure would vanish.
				const reported = code.replace(
					'void kit.start(app, element, options);',
					"kit.start(app, element, options).catch((error) => console.error('SvelteKit did not start:', error?.stack ?? error));"
				);
				if (reported === code) this.error('SvelteKit changed how its single bundle starts');
				return `import '@ethercorps/svelte-blitz/dom';\nimport '@ethercorps/svelte-blitz';\n${reported}`;
			}
			// SvelteKit mounts its root component itself, without saying which
			// renderer to use.
			if (id.endsWith('/kit/src/runtime/client/client.js')) {
				const patched = code.replace('render(Root, {', 'render(Root, {\n\t\trenderer: __blitz_renderer,');
				if (patched === code) this.error('SvelteKit changed how it mounts its root component');
				return `import __blitz_renderer from '@ethercorps/svelte-blitz/renderer';\n${patched}`;
			}
		}
	};
}

export default defineConfig({
	plugins: [
		blitz(),
		sveltekit({
			// A single-page app: no page is rendered ahead of time, one HTML shell.
			adapter: adapter({ fallback: 'index.html' }),
			output: { bundleStrategy: 'single' },
			compilerOptions: {
				experimental: { customRenderer: '@ethercorps/svelte-blitz/renderer' }
			}
		})
	],
	resolve: {
		conditions: ['svelte-custom-renderer', 'browser', 'svelte', 'module', 'import', 'default']
	}
});
