// `fetch` and the Fetch API classes, against a stand-in host: the request the
// host is asked to make, the response it reads back, and the `instanceof`
// checks libraries make (SvelteKit's client does `input instanceof Request` on
// every fetch).

import assert from 'node:assert/strict';

/** @type {{ name: string, args: any }[]} */
const calls = [];
/** @type {any} */ (globalThis).__sb_host = {
	/** @param {number} id @param {string} name @param {string} args */
	call(id, name, args) {
		calls.push({ name, args: JSON.parse(args) });
		const answer =
			name === 'http.fetch'
				? { id: 7, status: 201, status_text: 'Created', url: 'https://api.example.com/items', redirected: false, headers: [['Content-Type', 'application/json']] }
				: '{"made":true}';
		queueMicrotask(() => settle_call(id, true, JSON.stringify(answer)));
	}
};
// host-call.js reads __sb_host when it loads, so after the stand-in is there.
const { settle_call } = await import('../src/host-call.js');
const { fetch, Headers, Request, Response } = await import('../src/fetch.js');

// Headers: one name whatever its case, values appended, names kept in lower case.
const headers = new Headers({ 'X-Token': 'a' });
headers.append('x-token', 'b');
headers.set('Accept', 'application/json');
assert.equal(headers.get('X-TOKEN'), 'a, b');
assert.deepEqual([...headers], [['x-token', 'a, b'], ['accept', 'application/json']]);
assert.deepEqual([...new Headers(headers)], [...headers]);
assert.deepEqual([...new Headers([['A', '1']])], [['a', '1']]);

// Request: from a string, a URL-like object or another Request, init on top.
const request = new Request('https://api.example.com/items', { method: 'post', headers: { 'X-Key': 'k' }, body: '{"a":1}' });
assert.equal(request.method, 'POST');
assert.equal(new Request(request, { method: 'propfind' }).method, 'propfind');
assert.equal(new Request(request).body, '{"a":1}');
assert.equal(new Request({ href: 'https://api.example.com/x' }).url, 'https://api.example.com/x');
assert.ok(request instanceof Request && !(/** @type {unknown} */ ('a string') instanceof Request));

// fetch(Request): what the host is asked for, and what comes back.
const response = await fetch(request);
assert.deepEqual(calls[0], {
	name: 'http.fetch',
	args: { url: 'https://api.example.com/items', method: 'POST', headers: [['x-key', 'k']], body: '{"a":1}' }
});
assert.ok(response instanceof Response);
assert.equal(response.status, 201);
assert.equal(response.ok, true);
assert.equal(response.headers.get('content-type'), 'application/json');
assert.deepEqual(await response.json(), { made: true });
assert.deepEqual(calls[1], { name: 'http.body', args: { id: 7, as: 'text' } });
await assert.rejects(() => response.text(), /already read/);

// fetch(string) as before.
await fetch('https://api.example.com/items');
assert.deepEqual(calls[2].args, { url: 'https://api.example.com/items', method: 'GET', headers: [] });

// new Response(): a body in hand, as libraries build them.
const made = new Response('{"x":1}', { status: 404, headers: { 'Content-Type': 'application/json' } });
assert.equal(made.ok, false);
assert.equal(made.headers.get('content-type'), 'application/json');
assert.deepEqual(await made.json(), { x: 1 });

console.log('fetch: ok');
