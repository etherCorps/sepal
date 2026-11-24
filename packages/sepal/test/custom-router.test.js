// The `router` option of the preset: a router of the app's own as the entry,
// and routing switched off. Each variant is built with Vite and run in the
// JS mock host.

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import { MockHost } from './mock-host.js';
import { sepal } from '../vite/vite.js';

const project = fileURLToPath(new URL('./custom-router', import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), 'sepal-router-'));

/**
 * Build the fixture with these preset options and return what it renders.
 * @param {string} name
 * @param {Parameters<typeof sepal>[0]} options
 */
async function render(name, options) {
	const out = join(scratch, name);
	await build({
		root: project,
		configFile: false,
		logLevel: 'silent',
		plugins: [sepal(options)],
		build: { outDir: out, emptyOutDir: true }
	});
	const code = readFileSync(join(out, 'app.js'), 'utf8');
	const host = new MockHost();
	host.eval(code);
	assert.deepEqual(host.logs.filter(([level]) => level === 2), [], `${name}: no errors logged`);
	const items = host.by_id('routes').children.filter((node) => node.kind === 'element');
	return { code, ids: items.map((item) => host.text(item).trim()), fallback: host.text(host.by_id('fallback')).trim() };
}

try {
	// A router of the app's own gets the generated route table and is the entry.
	const custom = await render('custom', { router: { entry: 'src/my-router.js' } });
	// The table is in matching order (most specific first), which is not alphabetical.
	assert.deepEqual(custom.ids.toSorted(), ['/', '/about']);
	assert.equal(custom.fallback, 'false');
	assert.ok(!custom.code.includes('Not found:'), 'the built-in router is not in the bundle');

	// It may keep its route files somewhere else, or have none at all.
	const elsewhere = await render('elsewhere', { router: { entry: 'src/my-router.js', dir: 'src/routes/about' } });
	assert.deepEqual(elsewhere.ids, ['/']);
	const none = await render('none', { router: { entry: 'src/my-router.js', dir: 'src/no-such-directory' } });
	assert.deepEqual(none.ids, []);

	// Routing off: `src/routes` is ignored and `src/main.js` is the entry.
	const off = await render('off', { router: false });
	assert.deepEqual(off.ids, ['plain entry']);

	console.log('custom router: ok');
} finally {
	rmSync(scratch, { recursive: true, force: true });
}
