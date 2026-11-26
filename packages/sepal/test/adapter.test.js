// sepal/adapter: each thing it checks stops the build with what to add, and
// a good setup is written as SvelteKit writes it, plus app.json. SvelteKit's
// builder is stood in for; the real one runs in the examples' builds.

import assert from 'node:assert/strict';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import adapter from '../vite/adapter.js';
import { SEPAL_ICON, kit } from '../vite/app.js';

const project = mkdtempSync(join(tmpdir(), 'sepal-adapter-'));
const good = { compilerOptions: { experimental: { customRenderer: 'sepal/renderer' } }, version: { pollInterval: 0 }, preprocess: [{ name: 'sepal' }], files: { src: 'src', assets: 'static' } };

/**
 * A builder as SvelteKit hands it over, for a client like the one it writes.
 * @param {any} config
 * @param {string} [shell] the fallback page
 */
const builder = (config, shell = '<!doctype html><html><head><link href="/_app/immutable/assets/0.css" rel="stylesheet"></head><body><div><script>{}</script></div></body></html>') =>
	/** @type {any} */ ({
		config,
		prerendered: { paths: [] },
		log: { warn() {}, minor() {} },
		/** @param {string} to */
		writeClient(to) {
			mkdirSync(join(to, '_app/immutable/assets'), { recursive: true });
			writeFileSync(join(to, '_app/version.json'), '{"version":"1"}');
			writeFileSync(join(to, '_app/immutable/assets/0.css'), 'body { margin: 0 }');
			writeFileSync(join(to, 'logo.svg'), '<svg/>');
		},
		/** @param {string} to */
		async generateFallback(to) {
			writeFileSync(to, shell);
		}
	});

try {
	// Without sepal() in the plugins there is nothing to build the app with.
	kit.session = null;
	await assert.rejects(adapter().adapt(builder(good)), /add `sepal\(\)` from `sepal\/vite` to the plugins in vite\.config\.ts, after `sveltekit\(\)`/);

	let written = 0;
	kit.session = { root: project, options: { app: { name: 'Kit' } }, written: () => void written++ };
	await assert.rejects(adapter().adapt(builder({ ...good, compilerOptions: {} })), /not compiled for the Sepal renderer\. Pass SvelteKit's options through withSepal\(\)/);
	await assert.rejects(adapter().adapt(builder({ ...good, version: { pollInterval: 3_600_000 } })), /new version every hour[\s\S]*withSepal\(\)/);
	await assert.rejects(adapter().adapt(builder({ ...good, preprocess: [] })), /rewrites of `bind:` and `<svelte:head>` are not among the preprocessors\. Pass SvelteKit's options through withSepal\(\)/);
	mkdirSync(join(project, 'src/routes/api'), { recursive: true });
	writeFileSync(join(project, 'src/routes/+page.server.ts'), '');
	writeFileSync(join(project, 'src/routes/api/+server.ts'), '');
	await assert.rejects(adapter().adapt(builder(good)), /remove src\/routes\/\+page\.server\.ts, src\/routes\/api\/\+server\.ts: they are server code/);
	// Everything that is missing, at once.
	await assert.rejects(adapter().adapt(builder({ ...good, compilerOptions: {}, version: {} })), (error) => /** @type {Error} */ (error).message.split('\n').length === 3);
	rmSync(join(project, 'src'), { recursive: true });
	assert.equal(written, 0, 'nothing is written, and the dev window is not told, when a check fails');

	// A shell that would load from the network stops the build too.
	await assert.rejects(adapter().adapt(builder(good, '<html><head><link rel="stylesheet" href="https://cdn.example.com/x.css"></head><body></body></html>')), /loads the stylesheet https:\/\/cdn\.example\.com\/x\.css/);

	// The app is compiled to bytecode once written, every module of it: the
	// last build goes whole, its bytecode too. A script stands in for calyx.
	const dist = join(project, 'dist');
	const unix = process.platform !== 'win32';
	if (unix) {
		const calyx = join(project, 'calyx');
		writeFileSync(calyx, `#!/bin/sh\necho "$*" > "${join(project, 'calyx.log')}"\n`);
		chmodSync(calyx, 0o755);
		process.env.SEPAL_HOST = calyx;
	}
	mkdirSync(join(dist, '_app'), { recursive: true });
	writeFileSync(join(dist, '_app/start.js.qbc'), '');
	writeFileSync(join(dist, 'stale.txt'), '');
	await adapter().adapt(builder(good));
	delete process.env.SEPAL_HOST;
	for (const old of ['_app/start.js.qbc', 'stale.txt']) assert.ok(!existsSync(join(dist, old)), `${old} from the last build was left`);
	if (unix) assert.equal(readFileSync(join(project, 'calyx.log'), 'utf8').trim(), `--compile ${dist}`);
	const html = readFileSync(join(dist, 'index.html'), 'utf8');
	assert.match(html, /<link href="\/_app\/immutable\/assets\/0\.css" rel="stylesheet">\s*<style>#svelte-announcer \{ pointer-events: none \}<\/style>\s*<\/head>/, html);
	for (const file of ['_app/version.json', '_app/immutable/assets/0.css', 'logo.svg']) assert.ok(existsSync(join(dist, file)), `${file} was not written`);
	assert.equal(JSON.parse(readFileSync(join(dist, 'app.json'), 'utf8')).name, 'Kit');
	assert.ok(readFileSync(join(dist, 'icon.png')).equals(readFileSync(SEPAL_ICON)), 'no icon of its own: the Sepal icon');
	assert.equal(written, 1, 'the dev window is told once the app is written');

	// `out` moves it.
	await adapter({ out: 'build' }).adapt(builder(good));
	assert.ok(existsSync(join(project, 'build/index.html')));
} finally {
	kit.session = null;
	rmSync(project, { recursive: true, force: true });
}
console.log('adapter: ok');
