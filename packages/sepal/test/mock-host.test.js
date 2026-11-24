import fs from 'node:fs';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { MockHost } from './mock-host.js';

const bundle = fileURLToPath(new URL('../../../examples/counter/dist/app.js', import.meta.url));
const code = fs.readFileSync(bundle, 'utf8');
const squash = (s) => s.replace(/\s+/g, ' ').trim();

const host = new MockHost();
host.eval(code);

const errors = () => host.logs.filter(([level]) => level === 2);
assert.deepEqual(errors(), [], 'no errors logged during mount');

// --- initial render ---------------------------------------------------------
const inc = host.by_id('inc');
assert.ok(inc, 'button rendered');
assert.equal(squash(host.text(inc)), 'count is 0');
assert.equal(squash(host.text(host.by_id('doubled'))), 'doubled: 0');
assert.equal(host.by_id('badge'), null);
const rows = host.by_id('rows');
const lis = () => rows.children.filter((n) => n.kind === 'element');
assert.equal(lis().length, 1000);
assert.match(inc.attrs.class, /bg-indigo-600/);
assert.equal(host.stats.batches, 1, 'the whole mount is one batch');
console.log(`mount: ${host.stats.ops} ops in ${host.stats.batches} batch`);

// --- click -> reactive text update ------------------------------------------
assert.ok(host.listening.has('click'));
host.dispatch(inc, 'click');
await host.flush_microtasks();
assert.equal(squash(host.text(inc)), 'count is 1');
assert.equal(squash(host.text(host.by_id('doubled'))), 'doubled: 2');
console.log(`one click: ${host.stats.last_ops} ops`);
assert.ok(host.stats.last_ops <= 4, 'a counter click touches only the text nodes that changed');

// --- {#if} ------------------------------------------------------------------
host.dispatch(inc, 'click');
await host.flush_microtasks();
host.dispatch(inc, 'click');
await host.flush_microtasks();
assert.equal(squash(host.text(host.by_id('badge'))), 'three or more');

// --- bubbling: click a <span> inside a row, handler is on the <li> -----------
const third = lis()[2];
const span = third.children.find((n) => n.kind === 'element');
host.dispatch(span, 'click');
await host.flush_microtasks();
assert.match(third.attrs.class, /selected/);
assert.equal(squash(host.text(third)), 'Row 3 1');
console.log(`row select: ${host.stats.last_ops} ops`);
assert.ok(host.stats.last_ops <= 4, 'selecting a row does not touch the other 999');

// --- keyed {#each} reorder ---------------------------------------------------
const before = lis().map((n) => n.id);
host.dispatch(host.by_id('shuffle'), 'click');
await host.flush_microtasks();
const after = lis().map((n) => n.id);
assert.equal(after.length, 1000);
assert.notDeepEqual(after, before, 'rows moved');
assert.deepEqual([...after].sort((a, b) => a - b), [...before].sort((a, b) => a - b), 'rows were moved, not recreated');
console.log(`shuffle 1000 rows: ${host.stats.last_ops} ops`);

// --- timers ------------------------------------------------------------------
assert.equal(host.wake, -1, 'no timer pending while idle');
host.dispatch(host.by_id('tick'), 'click');
await host.flush_microtasks();
assert.notEqual(host.wake, -1, 'starting the timer asks the host for a wake-up');
host.advance(1000);
await host.flush_microtasks();
host.advance(1000);
await host.flush_microtasks();
assert.equal(squash(host.text(host.by_id('seconds'))), '2s');
host.dispatch(host.by_id('tick'), 'click');
await host.flush_microtasks();
host.advance(5000);
assert.equal(host.wake, -1, 'stopping the timer leaves nothing to wake up for');
assert.equal(squash(host.text(host.by_id('seconds'))), '2s');

// --- scrolling: the host reports it, onscroll hears it ---------------------------
assert.ok(host.listening.has('scroll'), 'onscroll registers like any other event');
assert.equal(squash(host.text(host.by_id('scrolled'))), 'scrolled 0px');
host.scroll(rows, [0, 120, 400, 37000, 400, 300]);
await host.flush_microtasks();
assert.equal(squash(host.text(host.by_id('scrolled'))), 'scrolled 120px', 'the handler read scrollTop from the node');
assert.equal(host.scrolls.length, 0, 'being told about a scroll writes nothing back');

assert.deepEqual(errors(), [], 'no errors logged');
console.log('ok');
