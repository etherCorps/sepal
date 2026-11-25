// `sepal/adapter`: the SvelteKit adapter for Sepal apps.
//
//   // vite.config.ts
//   import { sveltekit } from '@sveltejs/kit/vite';
//   import adapter from 'sepal/adapter';
//   import { sepal } from 'sepal/vite';
//
//   export default defineConfig({
//     plugins: [
//       sveltekit({
//         adapter: adapter(),
//         compilerOptions: { experimental: { customRenderer: 'sepal/renderer' } },
//         version: { pollInterval: 0 }
//       }),
//       sepal()
//     ]
//   });
//
// It writes SvelteKit's output as SvelteKit writes it, a single-page app:
// index.html as the shell every address starts from, the client chunks,
// stylesheets, fonts, static files and `_app/version.json`. Then `app.json`,
// for the host. Before that it checks what it was given, and stops the
// build saying what to add.
//
// Runs in Node.

import { cpSync, existsSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { kit, manifest } from './app.js';
import { check_css, check_page } from './page.js';

/** Every file under a directory, as paths relative to it, with `/`. @param {string} directory @returns {string[]} */
function files_in(directory) {
	return readdirSync(directory, { recursive: true, withFileTypes: true })
		.filter((entry) => entry.isFile())
		.map((entry) => relative(directory, join(entry.parentPath, entry.name)).replaceAll('\\', '/'));
}

/**
 * The files under `src` that SvelteKit would run on a server, which an app
 * does not have: they would build, and then never run. Relative to `src`.
 *
 * ponytail: comes out when the backend thread lands; that is where they will run.
 * @param {string} src
 */
export function server_files(src) {
	if (!existsSync(src)) return [];
	return files_in(src).filter((file) => /(^|\/)(\+(page|layout)\.server\.|\+server\.|hooks\.server\.)|\.remote\.[^/]+$/.test(file));
}

/**
 * What is missing from the setup, as messages saying what to add.
 * @param {import('@sveltejs/kit').Builder['config']} config
 * @param {string} root the project
 * @returns {string[]}
 */
export function problems(config, root) {
	const found = [];
	if (kit.session === null) {
		found.push('add `sepal()` from `sepal/vite` to the plugins in vite.config.ts, after `sveltekit()`: it builds the app for the Sepal renderer, and the adapter needs it.');
	}
	if (config.compilerOptions?.experimental?.customRenderer !== 'sepal/renderer') {
		found.push("add `compilerOptions: { experimental: { customRenderer: 'sepal/renderer' } }` to `sveltekit({ … })` in vite.config.ts: components have to be compiled for the Sepal renderer.");
	}
	if (config.version?.pollInterval !== 0) {
		found.push('add `version: { pollInterval: 0 }` to `sveltekit({ … })` in vite.config.ts: SvelteKit would ask for a new version every hour, waking an idle app. Apps update through `sepal/updater`.');
	}
	const src = resolve(root, config.files?.src ?? 'src');
	const server = server_files(src).map((file) => relative(root, join(src, file)).replaceAll('\\', '/'));
	if (server.length > 0) {
		found.push(
			`remove ${server.join(', ')}: ${server.length === 1 ? 'it is' : 'they are'} server code, which would run in the app's backend thread, and Sepal does not have one yet. Use +page.ts and +layout.ts (\`load\` runs in the app) until then.`
		);
	}
	return found;
}

/**
 * SvelteKit's route announcer is a 1px `overflow: hidden` box at the top
 * left. Blitz hit-tests what overflows such a box, so after the first
 * navigation its invisible text took clicks meant for what is drawn there.
 * ponytail: goes when Blitz's hit testing clips to `overflow: hidden` (HANDOVER.md, "Blitz hit testing clipped to overflow: hidden").
 */
const ANNOUNCER = '<style>#svelte-announcer { pointer-events: none }</style>';

/**
 * @param {{ out?: string }} [options] `out` is where the app is written (default `dist`).
 * @returns {import('@sveltejs/kit').Adapter}
 */
export default function adapter({ out = 'dist' } = {}) {
	return {
		name: 'sepal',
		async adapt(builder) {
			const session = kit.session;
			const root = session?.root ?? process.cwd();
			const missing = problems(builder.config, root);
			if (missing.length > 0) throw new Error(`sepal: ${missing.join('\nsepal: ')}`);
			if (session === null) return;

			const to = resolve(root, out);
			rmSync(to, { force: true, recursive: true });
			builder.writeClient(to);
			// The shell SvelteKit serves for an address it did not prerender:
			// what the app starts from, whatever its address.
			const page = join(to, 'index.html');
			await builder.generateFallback(page);
			if (builder.prerendered.paths.length > 0) {
				builder.log.warn(`sepal: prerendering is not done yet; ${builder.prerendered.paths.join(', ')} render in the app instead`);
			}

			const html = readFileSync(page, 'utf8');
			const { errors, warnings } = check_page(html);
			for (const file of files_in(to).filter((file) => file.endsWith('.css'))) errors.push(...check_css(readFileSync(join(to, file), 'utf8'), file));
			if (errors.length > 0) throw new Error(`sepal: ${errors.join('\nsepal: ')}`);
			for (const warning of warnings) builder.log.warn(`sepal: ${warning}`);
			writeFileSync(page, html.replace('</head>', `\t${ANNOUNCER}\n\t</head>`));

			const assets = resolve(root, builder.config.files?.assets ?? 'static');
			const written = manifest(session.options, root, existsSync(assets) ? assets : '');
			if (written) writeFileSync(join(to, 'app.json'), written.json);
			if (written?.icon) cpSync(written.icon, join(to, 'icon.png'));
			builder.log.minor(`Wrote the app to ${relative(root, to) || '.'}`);
			session.written(to);
		}
	};
}
