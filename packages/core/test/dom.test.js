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

// --- layout: the host reports boxes, nothing asks for them ---------------------------
const count = host.by_id('count');
assert.ok(host.measured.has(host.root.id), '<body> is measured from the start, for the viewport size');
run(`
	globalThis.sizes = [];
	globalThis.observer = new ResizeObserver((entries) => {
		sizes.push(entries.map((entry) => entry.target.id + ' ' + entry.contentRect.width + 'x' + entry.contentRect.height).join());
	});
	observer.observe(document.getElementById('count'));
	globalThis.resizes = 0;
	window.addEventListener('resize', () => resizes++);
`);
assert.equal(run(`document.getElementById('count').getBoundingClientRect().width`), 0, 'not laid out yet');
await host.flush_microtasks();
assert.ok(host.measured.has(count.id), 'observing a node asks the host to report it');
assert.equal(run('sizes.length'), 0, 'nothing to report before the first layout');

host.layout(900, 700, [[count, 32, 90, 84, 36]]);
assert.equal(run('sizes.join("|")'), 'count 84x36');
assert.equal(run(`JSON.stringify(document.getElementById('count').getBoundingClientRect())`), '{"x":32,"y":90,"width":84,"height":36,"top":90,"left":32,"right":116,"bottom":126}');
assert.equal(run(`document.getElementById('count').offsetWidth`), 84);
assert.equal(run('[innerWidth, innerHeight, document.documentElement.clientWidth, resizes].join()'), '900,700,900,1');

// Moved, same size: the box is new, the observer has nothing to say.
host.layout(900, 700, [[count, 40, 90, 84, 36]]);
assert.equal(run(`document.getElementById('count').getBoundingClientRect().left`), 40);
assert.equal(run('sizes.length + "," + resizes'), '1,1');
host.layout(800, 600, [[count, 40, 90, 60, 36]]);
assert.equal(run('sizes.join("|") + "," + resizes'), 'count 84x36|count 60x36,2');
run('observer.disconnect()');

// --- an event that hit nothing the app made belongs to <html> -------------------------
run(`globalThis.outside = null; document.addEventListener('pointerdown', (event) => (outside = event.target === document.documentElement));`);
host.settle();
host.dispatch({ id: 0 }, 'pointerdown');
assert.equal(run('outside'), true);

// --- focus: JavaScript moves it with an op, and hears its own events at once ----------
run(`
	globalThis.focus_log = [];
	for (const type of ['focusin', 'focusout']) document.addEventListener(type, (event) => focus_log.push(type + ' ' + event.target.id));
`);
host.settle();
assert.equal(run(`document.activeElement === document.body`), true);
assert.equal(run(`document.getElementById('count').focus(), document.activeElement.id`), 'count');
assert.equal(host.focused, null, 'the host is told at the end of the turn');
host.settle();
assert.equal(host.focused, host.by_id('count'));
// Focus does not land on something that cannot take it.
assert.equal(run(`document.getElementById('greeting').focus(), document.activeElement.id`), 'count');
assert.equal(run(`document.getElementById('name').focus(), focus_log.join()`), 'focusin count,focusout count,focusin name');
// When the host moves the focus (a click, Tab), it says so with events.
host.dispatch(host.by_id('count'), 'focusin');
assert.equal(run(`document.activeElement.id`), 'count');
run(`document.activeElement.blur()`);
host.settle();
assert.equal(host.focused, null);
assert.equal(run(`document.activeElement === document.body`), true);

// --- timers are tasks: the host runs them one at a time --------------------------------
run(`globalThis.ran = []; setTimeout(() => ran.push('a'), 5); setTimeout(() => ran.push('b'), 5); setTimeout(() => { ran.push('c'); setTimeout(() => ran.push('late'), 0); }, 1);`);
host.time += 10;
assert.equal(run('__sb.timers()'), true);
assert.equal(run('ran.join()'), 'c', 'one per call, soonest first');
assert.equal(run('[__sb.timers(), __sb.timers(), __sb.timers()].join() + " " + ran.join()'), 'true,true,false c,a,b');
assert.equal(run('__sb.timers() + " " + ran.join()'), 'true c,a,b,late', 'a timer set during a round waits for the next');
run('__sb.timers()');

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
