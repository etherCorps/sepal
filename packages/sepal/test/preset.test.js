// sepal(), for apps without SvelteKit: a web build from index.html, and
// the checks on what the page may load.

import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import { sepal } from '../vite/vite.js';
import { check_css, check_page } from '../vite/page.js';

// Next to the fixtures, where `sepal` resolves, as it would in an app.
const scratch = mkdtempSync(fileURLToPath(new URL('./.scratch-', import.meta.url)));

/**
 * A project with these files, built with sepal(). Returns where it was
 * built, and what the build warned.
 * @param {string} name
 * @param {Record<string, string>} files
 * @param {import('../vite/vite.js').Config} [config]
 */
async function built(name, files, config = {}) {
	const project = join(scratch, name);
	for (const [file, text] of Object.entries(files)) {
		mkdirSync(join(project, file, '..'), { recursive: true });
		writeFileSync(join(project, file), text);
	}
	/** @type {string[]} */
	const warnings = [];
	await build({
		root: project,
		configFile: false,
		logLevel: 'silent',
		customLogger: /** @type {any} */ ({ info() {}, warn: (/** @type {string} */ text) => warnings.push(text), warnOnce() {}, error() {}, clearScreen() {}, hasErrorLogged: () => false, hasWarned: false }),
		plugins: [sepal({ app: { name: name }, ...config })],
		build: { outDir: join(project, 'dist'), emptyOutDir: true }
	});
	return { dist: join(project, 'dist'), warnings };
}

/** @param {string} body @param {string} [head] */
const page = (body, head = '') => `<!doctype html><html><head><title>t</title>${head}</head><body>${body}<script type="module" src="/src/main.js"></script></body></html>`;

const main = { 'src/main.js': "import './app.css';\nglobalThis.started = true;\nimport('./later.js');\n", 'src/later.js': 'export const later = 1;\n', 'src/app.css': 'body { margin: 0 }\n' };

try {
	// A web build: index.html, its chunks and one stylesheet, and app.json. No app.js.
	{
		const { dist } = await built('plain', { 'index.html': page(''), ...main });
		const html = readFileSync(join(dist, 'index.html'), 'utf8');
		assert.match(html, /<script type="module" crossorigin src="\/assets\/index-[\w-]+\.js"><\/script>/);
		assert.match(html, /<link rel="stylesheet" crossorigin href="\/assets\/[\w-]+\.css">/);
		const assets = readdirSync(join(dist, 'assets'));
		assert.equal(assets.filter((file) => file.endsWith('.js')).length, 2, `one chunk per module here: ${assets}`);
		assert.equal(assets.filter((file) => file.endsWith('.css')).length, 1, 'one stylesheet');
		assert.ok(existsSync(join(dist, 'app.json')) && !existsSync(join(dist, 'app.js')));
		// No modulepreload polyfill: it reaches for `document`, which an app without the DOM layer has not got.
		const entry = readFileSync(join(dist, 'assets', /** @type {string} */ (assets.find((file) => file.startsWith('index-')))), 'utf8');
		assert.ok(!entry.includes('MutationObserver'), 'the modulepreload polyfill is in the build');
	}

	// With `dom: true`, the DOM layer comes first in the modules the page loads.
	{
		const { dist } = await built('dom', { 'index.html': page(''), ...main }, { dom: true });
		const entry = readdirSync(join(dist, 'assets')).find((file) => file.startsWith('index-'));
		assert.match(readFileSync(join(dist, 'assets', /** @type {string} */ (entry)), 'utf8'), /app\.localhost/, 'the DOM layer is not in the entry chunk');
	}

	// What the host leaves out is warned about.
	{
		const { warnings } = await built('unsupported', { 'index.html': page('<button onclick="go()">go</button><iframe src="/x.html"></iframe>'), ...main });
		assert.ok(warnings.some((text) => text.includes('event handler attributes')), warnings.join('\n'));
		assert.ok(warnings.some((text) => text.includes('<iframe>')), warnings.join('\n'));
	}

	// Nothing loads from the network: a remote script, stylesheet or font stops the build.
	await assert.rejects(built('remote-script', { 'index.html': page('', '<script src="https://cdn.example.com/x.js"></script>'), ...main }), /loads the script https:\/\/cdn\.example\.com\/x\.js\. Scripts load only from the app folder/);
	await assert.rejects(built('remote-sheet', { 'index.html': page('', '<link rel="stylesheet" href="//cdn.example.com/x.css">'), ...main }), /loads the stylesheet \/\/cdn\.example\.com\/x\.css/);
	await assert.rejects(
		built('remote-font', { 'index.html': page(''), ...main, 'src/app.css': "@font-face { font-family: Inter; src: url('https://fonts.example.com/inter.woff2') }\n" }),
		/loads the font https:\/\/fonts\.example\.com\/inter\.woff2\. Fonts load only from the app folder: self-host them, for example with Fontsource/
	);
	await assert.rejects(built('remote-import', { 'index.html': page(''), ...main, 'src/app.css': "@import url('https://fonts.googleapis.com/css2?family=Inter');\n" }), /imports the stylesheet https:\/\/fonts\.googleapis\.com/);

	// A project without a page is told what to write.
	await assert.rejects(built('no-page', main), /has no index\.html[\s\S]*<script type="module" src="\/src\/main\.ts"><\/script>/);
	// Pages in src/routes are SvelteKit's.
	await assert.rejects(built('routes', { 'src/routes/+page.svelte': '<h1>Home</h1>\n' }), /pages in src\/routes\. Pages are SvelteKit's[\s\S]*adapter: adapter\(\)[\s\S]*sepal\(\)/);
	// A SvelteKit app (it has src/app.html) without SvelteKit's plugin is told how to set it up.
	{
		const project = join(scratch, 'kit-without-plugin');
		mkdirSync(join(project, 'src'), { recursive: true });
		writeFileSync(join(project, 'src/app.html'), '<html><head>%sveltekit.head%</head><body>%sveltekit.body%</body></html>');
		const here = process.cwd();
		process.chdir(project);
		try {
			const plugins = sepal();
			await assert.rejects(build({ root: project, configFile: false, logLevel: 'silent', plugins }), /has src\/app\.html, so it is a SvelteKit app, but vite\.config\.ts has no `sveltekit\(\)`[\s\S]*sveltekit\(withSepal\(\{ adapter: adapter\(\) \}\)\), sepal\(\)/);
		} finally {
			process.chdir(here);
		}
	}

	// SvelteKit's options without withSepal() stop the build before anything
	// is compiled, naming it; and sepal() does not rewrite components itself
	// in a SvelteKit app: the rewrites are SvelteKit's options'.
	{
		const project = join(scratch, 'kit-without-withsepal');
		mkdirSync(join(project, 'src'), { recursive: true });
		writeFileSync(join(project, 'src/app.html'), '<html><head>%sveltekit.head%</head><body>%sveltekit.body%</body></html>');
		const here = process.cwd();
		process.chdir(project);
		try {
			const plugins = /** @type {any[]} */ (sepal());
			const setup = { name: 'vite-plugin-sveltekit-setup', api: { options: { compilerOptions: {}, version: {}, preprocess: [] } } };
			await assert.rejects(build({ root: project, configFile: false, logLevel: 'silent', plugins: [setup, ...plugins] }), /not compiled for the Sepal renderer\. Pass SvelteKit's options through withSepal\(\)[\s\S]*pollInterval[\s\S]*sepalPreprocess/);
			const edits = plugins.find((plugin) => plugin.name === 'sepal:sveltekit');
			assert.equal(edits.transform('<input bind:value={text} />', join(project, 'src/routes/+page.svelte')), undefined);
		} finally {
			process.chdir(here);
		}
	}

	// The checks themselves: images and local files are fine.
	assert.deepEqual(check_page(page('<img src="https://example.com/a.png">', '<link rel="stylesheet" href="/a.css"><link rel="icon" href="https://example.com/i.png">')), { errors: [], warnings: [] });
	assert.deepEqual(check_css("body { background: url('https://example.com/a.png') } @font-face { src: url('/inter.woff2') }", 'x.css'), []);
	assert.equal(check_page(page('', '<link rel="modulepreload" href="https://x.example/m.js">')).errors.length, 1);
	assert.equal(check_page(page('', '<!-- <script src="https://x.example/m.js"></script> -->')).errors.length, 0, 'comments are not the page');
	assert.ok(check_page(page('<script src="/legacy.js"></script>')).warnings[0].includes('without type="module"'));
} finally {
	rmSync(scratch, { recursive: true, force: true });
}
console.log('preset: ok');
