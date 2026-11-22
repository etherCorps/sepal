// SvelteKit, with its own client router, on the svelte-blitz renderer.
//
//   // vite.config.ts
//   import { svelteBlitzKit } from '@ethercorps/svelte-blitz-vite/sveltekit';
//   export default defineConfig({ plugins: [svelteBlitzKit()] });
//
// This takes the place of `sveltekit()`: it calls it, with what an app in a
// window needs. The result of `vite build` is the same as for any other
// svelte-blitz app: `dist/app.js`, `dist/app.css`, `dist/app.json` and the
// static files. Settings for the app come from `app.config.ts`, as always.
//
// An app has no server, so SvelteKit runs as a single-page app: no
// `+page.server.ts`, no form actions, no `+server.ts`, nothing prerendered.
//
// Two things here reach into SvelteKit's own files as they are bundled (see
// `patch`). A SvelteKit release can move them; the build then stops and says
// so, rather than produce an app that does not start.
//
// Runs in Node.

import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { dev_window, load_config, manifest } from './app.js';
import { element_bindings } from './bindings.js';
import { head } from './head.js';

/**
 * The edits to SvelteKit's client. Each names the file, what to find in it,
 * and what goes there instead.
 * @type {{ file: string, find: string, replace: string, prepend: string, why: string }[]}
 */
const patch = [
	{
		// The entry of a single-bundle build. The DOM and the runtime (`__sb`,
		// the host's way in) have to exist before any of SvelteKit runs; and
		// SvelteKit starts without waiting, so a failure would go unnoticed.
		file: '/kit/src/runtime/client/bundle.js',
		find: 'void kit.start(app, element, options);',
		replace: "kit.start(app, element, options).catch((error) => console.error('SvelteKit did not start:', error));",
		prepend: "import '@ethercorps/svelte-blitz/dom';\nimport '@ethercorps/svelte-blitz';\n",
		why: 'how its single bundle starts'
	},
	{
		// SvelteKit mounts its root component itself, and does not say with
		// which renderer.
		file: '/kit/src/runtime/client/client.js',
		find: 'render(Root, {',
		replace: 'render(Root, {\n\t\trenderer: __svelte_blitz_renderer,',
		prepend: "import __svelte_blitz_renderer from '@ethercorps/svelte-blitz/renderer';\n",
		why: 'how it mounts its root component'
	}
];

/** Every file under a directory, as paths relative to it. @param {string} directory @returns {string[]} */
function files_in(directory) {
	return readdirSync(directory, { recursive: true, withFileTypes: true })
		.filter((entry) => entry.isFile())
		// With `/` on Windows too: these are compared with the paths in SvelteKit's page.
		.map((entry) => relative(directory, join(entry.parentPath, entry.name)).replaceAll('\\', '/'));
}

/**
 * Turn what SvelteKit built (an HTML page that imports one module) into what
 * the host loads (a script, a stylesheet, and the other files as they are).
 * @param {string} from SvelteKit's client output, with its fallback page as `index.html`
 * @param {string} to
 */
export function convert(from, to) {
	const html = readFileSync(join(from, 'index.html'), 'utf8');
	// `__sveltekit_abc123 = { base: "", version: "…" };`, which SvelteKit's client reads.
	const payload = /__sveltekit_\w+ = \{[^}]*\};/.exec(html)?.[0];
	if (!payload) throw new Error('svelte-blitz: SvelteKit changed its start script; there is no payload in the fallback page');

	// The page says which files are this build's. The directory may hold more:
	// in watch mode SvelteKit leaves the previous build's next to them.
	const files = files_in(from);
	const bundle = /import\("\/([^"]+)"\)/.exec(html)?.[1];
	if (!bundle || !files.includes(bundle)) {
		throw new Error('svelte-blitz: SvelteKit did not produce a single bundle (output.bundleStrategy must be "single")');
	}
	let code = readFileSync(join(from, bundle), 'utf8');

	// The module's one export statement: `export{a as app,b as start}`, or
	// `export { app, start }` when not minified.
	const exported = /export\s*\{([^}]*)\};?\s*$/.exec(code);
	const start = exported?.[1]
		.split(',')
		.map((pair) => pair.trim().split(/\s+as\s+/))
		.find(([local, as = local]) => as === 'start')?.[0];
	if (!exported || !start) throw new Error('svelte-blitz: SvelteKit changed its bundle; it does not end by exporting `start`');
	code = code.slice(0, exported.index);

	// A script has no `import.meta`. The module's address only matters for
	// resolving paths next to it.
	code = code.replaceAll('import.meta.resolve', '((path) => new URL(path, __module_url).href)').replaceAll('import.meta.url', '__module_url');

	mkdirSync(to, { recursive: true });
	writeFileSync(
		join(to, 'app.js'),
		`(() => {
const __module_url = ${JSON.stringify(`http://app.localhost/${bundle}`)};
${code}
globalThis.${payload}
// Where SvelteKit puts the app: the element its start script sat in.
const element = document.createElement('div');
document.body.appendChild(element);
${start}(element);
})();
`
	);

	const styles = [...html.matchAll(/<link href="\/([^"]+)" rel="stylesheet">/g)].map((link) => link[1]).filter((file) => files.includes(file));
	writeFileSync(join(to, 'app.css'), styles.map((file) => readFileSync(join(from, file), 'utf8')).join('\n'));

	// Everything else is for the app to load by its path: static files, and
	// what SvelteKit put in `_app/immutable/assets`.
	for (const file of files) {
		if (file === 'index.html' || file.startsWith('.vite/') || file.endsWith('/version.json') || /\.(js|css)$/.test(file) && file.startsWith('_app/')) continue;
		mkdirSync(dirname(join(to, file)), { recursive: true });
		cpSync(join(from, file), join(to, file));
	}
}

/**
 * @typedef {import('./vite.js').Config & {
 *   kit?: import('@sveltejs/kit/vite').Config,
 *   out?: string
 * }} KitOptions
 * `app` and `permissions` override `app.config.ts`, key by key. `kit` is
 * passed on to SvelteKit. `out` is where the app is written (default `dist`).
 */

/**
 * @param {KitOptions} [inline]
 * @returns {Promise<import('vite').PluginOption[]>}
 */
export async function svelteBlitzKit(inline = {}) {
	const { sveltekit } = await import('@sveltejs/kit/vite').catch(() => {
		throw new Error('svelte-blitz: @sveltejs/kit is not installed. Add it to this project to use `svelteBlitzKit()`.');
	});
	const { kit = {}, out = 'dist', ...overrides } = inline;

	/** `app.config.ts` with `overrides` on top. Filled in by the `config` hook. @type {import('./vite.js').Config} */
	let options = overrides;
	let root = process.cwd();
	let watching = false;
	const show = dev_window(() => ({ root, out_dir: resolve(root, out), name: options.app?.name }));
	const static_dir = () => resolve(root, kit.files?.assets ?? 'static');

	/** @type {import('@sveltejs/kit').Adapter} */
	const adapter = {
		name: '@ethercorps/svelte-blitz-vite',
		async adapt(builder) {
			const staging = builder.getBuildDirectory('svelte-blitz');
			const to = resolve(root, out);
			builder.rimraf(staging);
			builder.rimraf(to);
			builder.writeClient(staging);
			// The page SvelteKit serves for a route it did not prerender: nothing
			// in it but the script that starts the app.
			await builder.generateFallback(join(staging, 'index.html'));
			convert(staging, to);

			const source = manifest(options, root, existsSync(static_dir()) ? static_dir() : '');
			if (source) writeFileSync(join(to, 'app.json'), source);
			builder.log.minor(`Wrote the app to ${out}`);
			if (watching) show();
		}
	};

	/** @type {import('vite').Plugin} */
	const plugin = {
		name: 'svelte-blitz:sveltekit',
		enforce: 'pre',
		async config(user, env) {
			const project = resolve(user.root ?? '.');
			options = { ...(await load_config(project, env)).config, ...overrides };
			return {
				resolve: {
					// Selects Svelte's client build with custom renderer support, and
					// tells libraries that there is a DOM (the layer loaded in `patch`).
					conditions: ['svelte-custom-renderer', 'browser', 'svelte', 'module', 'import', 'default']
				}
			};
		},
		configResolved(config) {
			root = config.root;
			watching = config.build.watch != null;
		},
		transform(code, id) {
			const edit = patch.find(({ file }) => id.endsWith(file));
			if (edit === undefined) return;
			if (!code.includes(edit.find)) {
				this.error(`svelte-blitz: this version of SvelteKit changed ${edit.why}, which svelte-blitz has to adjust. Use the SvelteKit version svelte-blitz was tested with, or update svelte-blitz.`);
			}
			return { code: edit.prepend + code.replace(edit.find, edit.replace), map: null };
		}
	};

	return [
		plugin,
		...(await sveltekit({
			...kit,
			adapter,
			// One module, not one per route: the host loads one script.
			output: { ...kit.output, bundleStrategy: 'single' },
			// Nothing is rendered ahead of time. There is no server to send it.
			prerender: { ...kit.prerender, entries: kit.prerender?.entries ?? [] },
			// Ours first: they have to see `bind:` on elements and `<svelte:head>`
			// before Svelte refuses them.
			preprocess: [element_bindings(), head(), ...[kit.preprocess ?? []].flat()],
			compilerOptions: {
				...kit.compilerOptions,
				experimental: { ...kit.compilerOptions?.experimental, customRenderer: '@ethercorps/svelte-blitz/renderer' }
			}
		}))
	];
}
