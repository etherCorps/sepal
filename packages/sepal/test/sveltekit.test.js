// withSepal(), SvelteKit's options for a Sepal app, and the check of them
// that sepal() and the adapter make. What is left of sepalKit(): a stub that
// says how to move, for one release. The server files the adapter refuses.

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { preprocess } from 'svelte/compiler';
import adapter, { server_files } from '../vite/adapter.js';
import { sepalKit } from '../vite/sveltekit.js';
import { kit_setup_problems, sepalPreprocess, withSepal } from '../vite/vite.js';

// What cannot work in an app throws, saying what to do.
assert.throws(() => withSepal({}), /withSepal\(\) has no adapter[\s\S]*sveltekit\(withSepal\(\{ adapter: adapter\(\) \}\)\)/);
assert.throws(() => withSepal(/** @type {any} */ ({ adapter: { name: '@sveltejs/adapter-static', adapt() {} } })), /was given the adapter @sveltejs\/adapter-static\. A Sepal app's is sepal\/adapter/);
assert.throws(() => withSepal({ adapter: adapter(), compilerOptions: { experimental: { customRenderer: 'other/renderer' } } }), /customRenderer is "other\/renderer"[\s\S]*'sepal\/renderer'/);
assert.throws(() => withSepal({ adapter: adapter(), version: { pollInterval: 60_000 } }), /pollInterval is 60000[\s\S]*sepal\/updater/);

// The app's own options are kept; arrays, functions and class instances as they are.
{
	class Thing {}
	const onwarn = () => {};
	const extensions = ['.svelte', '.svx'];
	const thing = new Thing();
	const own = { name: 'own', markup: () => undefined };
	const app = /** @type {any} */ ({
		adapter: adapter({ out: 'build' }),
		compilerOptions: { runes: true, experimental: { async: true } },
		version: { name: '1.2.3' },
		paths: { base: '/x' },
		preprocess: [own],
		extensions,
		onwarn,
		thing
	});
	const before = JSON.stringify(app);
	const options = /** @type {any} */ (withSepal(app));
	assert.equal(JSON.stringify(app), before, "the app's object is not changed");
	assert.equal(options.adapter, app.adapter);
	assert.deepEqual(options.compilerOptions, { runes: true, experimental: { async: true, customRenderer: 'sepal/renderer' } });
	assert.deepEqual(options.version, { name: '1.2.3', pollInterval: 0 });
	assert.deepEqual(options.paths, { base: '/x' });
	assert.equal(options.extensions, extensions);
	assert.equal(options.onwarn, onwarn);
	assert.equal(options.thing, thing);
	// Sepal's rewrites after the app's own preprocessors, once, however often withSepal() is called.
	assert.deepEqual(options.preprocess.map((/** @type {any} */ group) => group.name), ['own', 'sepal']);
	assert.equal(options.preprocess[0], own);
	const twice = /** @type {any} */ (withSepal(withSepal(app)));
	assert.deepEqual(twice.preprocess.map((/** @type {any} */ group) => group.name), ['own', 'sepal']);
	assert.deepEqual(kit_setup_problems(options), []);
	// A single preprocessor of the app's own, and the values withSepal() sets, given by hand.
	const single = /** @type {any} */ (withSepal({ adapter: adapter(), preprocess: own, compilerOptions: { experimental: { customRenderer: 'sepal/renderer' } }, version: { pollInterval: 0 } }));
	assert.deepEqual(single.preprocess.map((/** @type {any} */ group) => group.name), ['own', 'sepal']);
}

// The rewrites, as one preprocessor for apps that set the options by hand.
{
	const { code } = await preprocess('<input bind:value={text} /><svelte:head><title>Hi</title></svelte:head>', [sepalPreprocess()], { filename: 'Page.svelte' });
	assert.ok(!code.includes('bind:value') && !code.includes('<svelte:head'), code);
}

// Without withSepal(), sepal() and the adapter say what is missing, naming it.
{
	const missing = kit_setup_problems({ adapter: adapter() });
	assert.equal(missing.length, 3);
	for (const [message, what] of [[missing[0], /customRenderer/], [missing[1], /pollInterval: 0/], [missing[2], /sepalPreprocess\(\)/]]) {
		assert.match(message, /withSepal\(\) from sepal\/vite: `sveltekit\(withSepal\(\{ adapter: adapter\(\) \}\)\)`/);
		assert.match(message, what);
	}
}
console.log('withSepal: ok');

assert.throws(() => sepalKit(), /sepalKit\(\) is gone[\s\S]*sveltekit\(withSepal\(\{ adapter: adapter\(\) \}\)\), sepal\(\)[\s\S]*withSepal\(\{ … \}\) as it is[\s\S]*adapter\(\{ out \}\)/);
console.log('sveltekit stub: ok');

// Server code builds and then never runs in an app; the adapter refuses it.
const src = mkdtempSync(join(tmpdir(), 'sepal-server-files-'));
try {
	const server = ['routes/+page.server.ts', 'routes/a/+layout.server.js', 'routes/api/+server.ts', 'hooks.server.ts', 'lib/data.remote.ts'];
	const fine = ['routes/+page.ts', 'routes/+layout.ts', 'routes/+page.svelte', 'hooks.client.ts', 'lib/server.ts', 'lib/remote.ts', 'lib/my+server.md'];
	for (const file of [...server, ...fine]) {
		mkdirSync(dirname(join(src, file)), { recursive: true });
		writeFileSync(join(src, file), '');
	}
	assert.deepEqual(server_files(src).sort(), server.sort());
	assert.deepEqual(server_files(join(src, 'nowhere')), []);
} finally {
	rmSync(src, { recursive: true, force: true });
}
console.log('sveltekit server files: ok');
