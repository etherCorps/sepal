// The DOM layer (`@ethercorps/svelte-blitz/dom`), as examples/shadcn loads it
// with `dom: true`. The mock host's context has ECMAScript built-ins only, so
// everything a library finds here was put there by the layer.

import fs from 'node:fs';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { MockHost } from './mock-host.js';

const bundle = fileURLToPath(new URL('../../../examples/shadcn/dist/app.js', import.meta.url));
const host = new MockHost();
host.eval(fs.readFileSync(bundle, 'utf8'));
// The router shows the page once its module has loaded.
await host.flush_microtasks();
assert.deepEqual(host.logs.filter(([level]) => level === 2), [], 'no errors logged during mount');

/** Run source in the app's context. Values come back from another realm: compare primitives. */
const run = (source) => vm.runInContext(source, host.context);
const squash = (s) => s.replace(/\s+/g, ' ').trim();

// --- the globals a browser library reaches for --------------------------------
assert.equal(run('window === globalThis && document.body.tagName'), 'BODY');
assert.equal(run('document.body instanceof HTMLElement && !(document.body instanceof SVGElement)'), true);

// --- selectors ------------------------------------------------------------------
assert.equal(run(`document.querySelector('#count').tagName`), 'BUTTON');
assert.equal(run(`document.querySelectorAll('main > div [data-slot=button]').length`), 8);
assert.equal(run(`document.getElementById('count').closest('main') === document.querySelector('main')`), true);
// Libraries borrow methods from the class, not from the node.
assert.equal(run(`Element.prototype.matches.call(document.getElementById('count'), 'button[data-slot^=but]:not([disabled])')`), true);

// --- events: capture on the way down, then the target, then up -------------------
run(`
	globalThis.order = [];
	const button = document.getElementById('count');
	const note = (what) => (event) => order.push(what + (event.target === button ? '' : '?'));
	window.addEventListener('click', note('window'));
	document.addEventListener('click', note('document'));
	document.addEventListener('click', note('document capture'), true);
	document.body.addEventListener('click', note('body capture, once'), { capture: true, once: true });
	button.addEventListener('click', note('button'));
`);
host.dispatch(host.by_id('count'), 'click');
assert.equal(run('order.join(" > ")'), 'document capture > body capture, once > button > document > window');
host.dispatch(host.by_id('count'), 'click');
assert.equal(run('order.slice(5).join(" > ")'), 'document capture > button > document > window');
await host.flush_microtasks();
assert.equal(squash(host.text(host.by_id('count'))), 'Clicked 2', "Svelte's own handler ran too");

// --- writes go through the op buffer, like Svelte's --------------------------------
run(`
	const made = document.createElement('div');
	made.id = 'made';
	made.className = 'a b';
	made.classList.add('c');
	made.style.setProperty('color', 'red');
	made.textContent = 'hello';
	document.body.appendChild(made);
`);
assert.equal(host.by_id('made'), null, 'nothing reaches the host before the turn ends');
host.settle();
assert.equal(host.by_id('made').attrs.class, 'a b c');
assert.match(host.by_id('made').attrs.style, /color:\s*red/);
assert.equal(host.text(host.by_id('made')), 'hello');
run(`document.getElementById('made').remove()`);
host.settle();
assert.equal(host.by_id('made'), null);

// --- a node cannot go inside itself ------------------------------------------------
// The host would loop forever on a tree with a cycle in it.
assert.throws(() => run(`document.querySelector('main').firstElementChild.appendChild(document.querySelector('main'))`), /descendant/);

// --- a Bits UI component, end to end -------------------------------------------------
assert.equal(squash(host.text(host.by_id('switch-state'))), 'Switch is off');
host.dispatch(host.by_id('switch'), 'click');
await host.flush_microtasks();
assert.equal(squash(host.text(host.by_id('switch-state'))), 'Switch is on');
assert.equal(host.by_id('switch').attrs['data-state'], 'checked');

assert.deepEqual(host.logs.filter(([level]) => level === 2), [], 'no errors logged');
console.log('dom layer: ok');
