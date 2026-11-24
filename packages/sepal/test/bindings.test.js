// `bind:` on elements is refused by Svelte's custom renderer build. The
// preset rewrites it first; this checks what the rewrite produces, and that
// the result compiles where the original does not.

import assert from 'node:assert/strict';
import { compile, preprocess } from 'svelte/compiler';
import { element_bindings } from '../vite/bindings.js';

const options = { filename: 'T.svelte', generate: 'client', css: 'external', experimental: { customRenderer: 'sepal/renderer' } };

/** @param {string} source */
const rewrite = async (source) => (await preprocess(source, element_bindings(), { filename: 'src/lib/T.svelte' })).code;
/** @param {string} source */
const compiles = (source) => {
	try {
		compile(source, /** @type {any} */ (options));
		return true;
	} catch {
		return false;
	}
};

// bind:this on an element, as every shadcn component has it.
const ref = '<script>let { ref = $bindable(null) } = $props();</script>\n<div\n\tbind:this={ref}\n\tclass="x"\n></div>';
assert.equal(compiles(ref), false, 'Svelte refuses the original');
const ref_out = await rewrite(ref);
assert.ok(!ref_out.includes('bind:this'));
assert.match(ref_out, /\{@attach \(__node\) => \{ ref = __node; return \(\) => \{ if \(ref === __node\) ref = null; \}; \}\}/);
assert.equal(ref_out.split('\n').length, ref.split('\n').length, 'line numbers are unchanged');
assert.ok(compiles(ref_out));

// A member expression is a fine thing to bind to.
assert.ok(compiles(await rewrite('<script>const refs = $state({ el: null });</script><p bind:this={refs.el}></p>')));

// bind:value on form elements becomes a value and an input listener.
const value = '<script>let v = $state("");</script><input class="a" bind:value={v} /><textarea bind:value={v}></textarea>';
const value_out = await rewrite(value);
assert.equal((value_out.match(/value=\{v\}/g) ?? []).length, 2);
assert.equal((value_out.match(/addEventListener\('input'/g) ?? []).length, 2);
assert.ok(compiles(value_out));

// Bindings the host has nothing behind still compile, and say so when the element is created.
const other_out = await rewrite('<script>let f = $state(); let w = $state(0);</script><input type="file" bind:files={f} /><div bind:clientWidth={w}></div>');
assert.match(other_out, /bind:files on <input> is not supported and does nothing \(lib\/T\.svelte\)/);
assert.match(other_out, /bind:clientWidth on <div> is not supported/);
assert.ok(compiles(other_out));

// Components keep their bindings: those are Svelte's own and work.
const component = '<script>import C from "./C.svelte"; let v = $state(""); let c = $state();</script><C bind:value={v} bind:this={c} />';
assert.equal(await rewrite(component), component);

// Nothing to do, nothing changed; and a file that does not parse is left for the compiler.
const plain = '<script>let n = $state(0);</script><button onclick={() => n++}>{n}</button>';
assert.equal(await rewrite(plain), plain);
const broken = '<div bind:this={ref}';
assert.equal(await rewrite(broken), broken);

console.log('bindings: ok');
