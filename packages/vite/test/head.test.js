// The <svelte:head> rewrite: what comes out compiles for a custom renderer,
// and sets the title.
import assert from 'node:assert/strict';
import { compile, preprocess } from 'svelte/compiler';
import { head } from '../src/head.js';

const warnings = [];
const warn = console.warn;
console.warn = (message) => warnings.push(message);

async function rewrite(source) {
	const { code } = await preprocess(source, [head()], { filename: 'Page.svelte' });
	// The real test: Svelte accepts it for a custom renderer.
	compile(code, { filename: 'Page.svelte', experimental: { customRenderer: 'x' } });
	return code;
}

// Untouched without the block.
assert.equal(await rewrite('<p>hi</p>'), '<p>hi</p>');

// A plain title.
assert.equal(await rewrite('<svelte:head><title>About</title></svelte:head>\n<p>hi</p>'), "{(globalThis.document && (globalThis.document.title = `About`), '')}\n<p>hi</p>");

// Expressions, and characters that mean something in a template literal.
const dynamic = await rewrite('<script>let { data } = $props();</script>\n<svelte:head>\n\t<title>{data.name} - `Shop` $5</title>\n</svelte:head>\n<p>hi</p>');
assert.match(dynamic, /document\.title = `\$\{data\.name\} - \\`Shop\\` \\\$5`/);
assert.equal(dynamic.split('\n').length, 5, 'the same number of lines as before');
assert.equal(warnings.length, 0);

// Other head content goes, with a warning that names it.
const meta = await rewrite('<svelte:head><title>T</title><meta name="description" content="x"><link rel="icon" href="/i.png"></svelte:head>');
assert.equal(meta, "{(globalThis.document && (globalThis.document.title = `T`), '')}");
assert.match(warnings[0], /<meta>, <link> in <svelte:head> is ignored.*Page\.svelte/);

// No title: the block just goes.
assert.equal(await rewrite('<svelte:head><meta charset="utf-8"></svelte:head><p>x</p>'), '<p>x</p>');

// The expression really sets the title when it runs.
const set = new Function('document', 'data', `globalThis.document = document; return ${dynamic.split('\n').find((line) => line.includes('document.title')).slice(1, -1)}`);
const fake = { title: '' };
assert.equal(set(fake, { name: 'Lamp' }), '');
assert.equal(fake.title, 'Lamp - `Shop` $5');

console.warn = warn;
console.log('head: ok');
