// fetch.js against a stand-in for the host: what fetch asks for, what comes
// back, and the classes around it, as the Fetch API defines them.
import assert from 'node:assert/strict';

/** @type {{ name: string, args: any }[]} */
const calls = [];
/** Body chunks the stand-in hands out for `http.read`, per response id. @type {Map<number, string[]>} */
const chunks = new Map([[8, ['aGVs', 'bG8g', 'd29ybGQ=']]]);
/** @type {any} */ (globalThis).__sb_host = {
	call(/** @type {number} */ id, /** @type {string} */ name, /** @type {string} */ args) {
		const parsed = JSON.parse(args);
		calls.push({ name, args: parsed });
		let answer;
		if (name === 'http.fetch') {
			const streamed = parsed.url.endsWith('/stream');
			answer = { id: streamed ? 8 : 7, status: 201, status_text: 'Created', url: parsed.url, redirected: false, headers: [['Content-Type', 'application/json'], ['Set-Cookie', 'a=1'], ['Set-Cookie', 'b=2']] };
		} else if (name === 'http.read') {
			const left = chunks.get(parsed.id) ?? [];
			const chunk = left.shift();
			answer = chunk === undefined ? { chunk: '', done: true } : { chunk, done: false };
		} else answer = '{"made":true}';
		queueMicrotask(() => settle_call(id, true, JSON.stringify(answer)));
	}
};
// host-call.js reads __sb_host when it loads, so after the stand-in is there.
const { settle_call } = await import('../src/host-call.js');
const { fetch, Headers, Request, Response, FormData, File, Blob, ReadableStream, AbortController, URLSearchParams } = await import('../src/web.js');

// Headers: one name whatever its case, values appended, names kept in lower
// case, iterated in name order as the Fetch API does; Set-Cookie lines kept apart.
const headers = new Headers({ 'X-Token': 'a' });
headers.append('x-token', 'b');
headers.set('Accept', 'application/json');
assert.equal(headers.get('X-TOKEN'), 'a, b');
assert.deepEqual([...headers], [['accept', 'application/json'], ['x-token', 'a, b']]);
assert.deepEqual([...new Headers(headers)], [...headers]);
assert.deepEqual([...new Headers([['A', '1']])], [['a', '1']]);
const cookies = new Headers();
cookies.append('Set-Cookie', 'a=1; Path=/');
cookies.append('set-cookie', 'b=2, with a comma');
assert.deepEqual(cookies.getSetCookie(), ['a=1; Path=/', 'b=2, with a comma']);
assert.deepEqual([...cookies], [['set-cookie', 'a=1; Path=/'], ['set-cookie', 'b=2, with a comma']]);

// Request: from a string, a URL-like object or another Request, init on top.
const request = new Request('https://api.example.com/items', { method: 'post', headers: { 'X-Key': 'k' }, body: '{"a":1}' });
assert.equal(request.method, 'POST');
assert.equal(request.headers.get('content-type'), 'text/plain;charset=UTF-8', 'a string body implies its type');
assert.equal(new Request(request, { method: 'propfind' }).method, 'propfind');
assert.ok(new Request(request).body instanceof ReadableStream);
assert.equal(await new Request(request).text(), '{"a":1}', 'a copy reads the same body');
assert.equal(new Request({ href: 'https://api.example.com/x' }).url, 'https://api.example.com/x');
assert.ok(request instanceof Request && !(/** @type {unknown} */ ('a string') instanceof Request));
assert.equal(new Request('/x').body, null);
assert.equal(new Request('/x').signal.aborted, false);

// fetch(Request): what the host is asked for, and what comes back. A text
// body goes to the host as text. (The copies above split `request`'s body
// into a stream, which would go as base64.)
const response = await fetch(new Request('https://api.example.com/items', { method: 'post', headers: { 'X-Key': 'k' }, body: '{"a":1}' }));
assert.deepEqual(calls[0], {
	name: 'http.fetch',
	args: { url: 'https://api.example.com/items', method: 'POST', headers: [['content-type', 'text/plain;charset=UTF-8'], ['x-key', 'k']], body: '{"a":1}' }
});
assert.ok(response instanceof Response);
assert.equal(response.status, 201);
assert.equal(response.ok, true);
assert.equal(response.headers.get('content-type'), 'application/json');
assert.deepEqual(response.headers.getSetCookie(), ['a=1', 'b=2']);
assert.deepEqual(await response.json(), { made: true });
assert.deepEqual(calls[1], { name: 'http.body', args: { id: 7, as: 'text' } });
assert.equal(response.bodyUsed, true);
await assert.rejects(() => response.text(), /already read/);

// fetch(string) as before.
await fetch('https://api.example.com/items');
assert.deepEqual(calls[2].args, { url: 'https://api.example.com/items', method: 'GET', headers: [] });

// response.body: the host's body a chunk at a time, as it arrives.
const streamed = await fetch('https://api.example.com/stream');
const reader = streamed.body.getReader();
const parts = [];
for (;;) {
	const { done, value } = await reader.read();
	if (done) break;
	parts.push(new TextDecoder().decode(value));
}
assert.deepEqual(parts, ['hel', 'lo ', 'world']);
assert.deepEqual(calls.filter((call) => call.name === 'http.read').map((call) => call.args), [{ id: 8 }, { id: 8 }, { id: 8 }, { id: 8 }]);

// Bytes, Blobs and URLSearchParams as request bodies, sent as base64 or text.
await fetch('https://api.example.com/items', { method: 'POST', body: new Uint8Array([1, 2, 255]) });
assert.equal(calls.at(-1).args.body_base64, 'AQL/');
await fetch('https://api.example.com/items', { method: 'POST', body: new URLSearchParams({ a: '1', b: 'two words' }) });
assert.deepEqual(calls.at(-1).args.headers, [['content-type', 'application/x-www-form-urlencoded;charset=UTF-8']]);
assert.equal(calls.at(-1).args.body, 'a=1&b=two+words');
await fetch('https://api.example.com/items', { method: 'POST', body: new Blob(['blob'], { type: 'text/x-test' }) });
assert.deepEqual(calls.at(-1).args.headers, [['content-type', 'text/x-test']]);

// An aborted signal: fetch rejects with its reason, before or during.
const controller = new AbortController();
controller.abort();
await assert.rejects(() => fetch('https://api.example.com/items', { signal: controller.signal }), { name: 'AbortError' });
const later = new AbortController();
const pending = fetch('https://api.example.com/items', { signal: later.signal });
later.abort(new Error('changed my mind'));
await assert.rejects(() => pending, /changed my mind/);

// new Response(): a body in hand, as libraries build them.
const made = new Response('{"x":1}', { status: 404, headers: { 'Content-Type': 'application/json' } });
assert.equal(made.ok, false);
assert.equal(made.headers.get('content-type'), 'application/json');
assert.deepEqual(await made.json(), { x: 1 });
assert.equal(await new Response(new Uint8Array([104, 105])).text(), 'hi', 'text() of a byte body');
assert.deepEqual([...(await new Response('hi').bytes())], [104, 105], 'bytes() of a text body');
assert.equal(await new Response(null).text(), '');
const stream = new ReadableStream({
	start(controller) {
		controller.enqueue(new TextEncoder().encode('from '));
		controller.enqueue(new TextEncoder().encode('a stream'));
		controller.close();
	}
});
assert.equal(await new Response(stream).text(), 'from a stream');

// clone(): both copies read the whole body.
const original = new Response('twice');
const copy = original.clone();
assert.equal(await original.text(), 'twice');
assert.equal(await copy.text(), 'twice');

// The statics.
const json = Response.json({ ok: true }, { status: 201 });
assert.equal(json.status, 201);
assert.equal(json.headers.get('content-type'), 'application/json');
assert.deepEqual(await json.json(), { ok: true });
const redirect = Response.redirect('https://example.com/next', 303);
assert.equal(redirect.status, 303);
assert.equal(redirect.headers.get('location'), 'https://example.com/next');
assert.throws(() => Response.redirect('/x', 200), RangeError);
assert.equal(Response.error().type, 'error');
assert.throws(() => new Response('x', { status: 100 }), RangeError);

// FormData as a body, both ways: multipart with a file, and urlencoded.
const form = new FormData();
form.append('title', 'A "quoted"\nname');
form.append('upload', new File(['file contents'], 'notes.txt', { type: 'text/plain' }));
const posted = new Request('https://api.example.com/form', { method: 'POST', body: form });
assert.match(posted.headers.get('content-type') ?? '', /^multipart\/form-data; boundary=/);
const parsed = await new Response(await posted.arrayBuffer(), { headers: { 'content-type': posted.headers.get('content-type') ?? '' } }).formData();
assert.equal(parsed.get('title'), 'A "quoted"\r\nname', 'line breaks normalised to CRLF, as browsers send them');
const file = /** @type {any} */ (parsed.get('upload'));
assert.ok(file instanceof File);
assert.equal(file.name, 'notes.txt');
assert.equal(file.type, 'text/plain');
assert.equal(await file.text(), 'file contents');
const urlencoded = await new Response('a=1&b=two+words', { headers: { 'content-type': 'application/x-www-form-urlencoded' } }).formData();
assert.deepEqual([...urlencoded], [['a', '1'], ['b', 'two words']]);
await assert.rejects(() => new Response('{}', { headers: { 'content-type': 'application/json' } }).formData(), /not a form/);

console.log('fetch: ok');
