// Our URL and URLSearchParams against Node's, on the kinds of URL an app meets.
import assert from 'node:assert/strict';
import { URL as Ours, URLSearchParams as OurParams } from '../src/url.js';
import { cache_sizes, lru } from '../src/lru.js';

const PARTS = ['href', 'origin', 'protocol', 'username', 'password', 'host', 'hostname', 'port', 'pathname', 'search', 'hash'];
const describe = (url) => Object.fromEntries(PARTS.map((part) => [part, url[part]]));

const bases = [undefined, 'http://localhost/', 'https://example.com:8443/a/b/c?x=1#top', 'http://app.localhost/items/42', 'a://', 'custom://host/a/b', 'a:'];
const inputs = [
	'http://localhost/', 'https://Example.COM:443/Path?Q=1#Frag', 'http://localhost:80/a/../b/./c', 'http://user:pw@host.test:8080/x',
	'/', '/a/b', 'a/b', './a', '../a', '../../a/b/', '.', '..', '', '?q=1', '?', '#hash', '#', 'a?b#c', '/a/b/../c/./d/', '//other.test/x',
	'/with space/and"quote', '/ünï/cödé?kéy=välue#frâg', '/a?b=c&d=e f', 'items/7?tab=two#notes', 'http://localhost:3000', 'not a url', 'http://', 'mailto:x@y.z', 'a://', 'custom://host', 'custom://host/p?q#h', 'a:', 'mailto:x@y.z?subject=hi#f'
];
// Where Node's own versions give different answers (22 against 26), there is
// nothing to agree with. Both are corners no app goes into: a relative path
// against a URL that has no path to be relative to.
const unsettled = (input, base) => (base === 'a:' && !/^[a-z]+:|^#/i.test(input)) || (base === 'a://' && input === '..');

let compared = 0;
for (const base of bases) {
	for (const input of inputs) {
		if (unsettled(input, base)) continue;
		let theirs, ours;
		try {
			theirs = describe(new URL(input, base));
		} catch {
			theirs = 'throws';
		}
		try {
			ours = describe(new Ours(input, base));
		} catch {
			ours = 'throws';
		}
		assert.deepEqual(ours, theirs, `new URL(${JSON.stringify(input)}, ${JSON.stringify(base)})`);
		compared++;
		// The same again with the base as a URL, and once more, which comes from the cache of parses.
		if (base === undefined || ours === 'throws') continue;
		for (let again = 0; again < 2; again++) {
			let from_url;
			try {
				from_url = describe(new Ours(input, new Ours(base)));
			} catch {
				from_url = 'throws';
			}
			assert.deepEqual(from_url, theirs, `new URL(${JSON.stringify(input)}, new URL(${JSON.stringify(base)}))`);
		}
	}
}

// Parses are cached by input and base, but every URL is its own object.
{
	const first = new Ours('items/7?tab=two', 'http://app.localhost/a/');
	first.pathname = '/changed';
	first.searchParams.set('tab', 'three');
	const second = new Ours('items/7?tab=two', 'http://app.localhost/a/');
	assert.notEqual(first, second);
	assert.equal(second.href, 'http://app.localhost/a/items/7?tab=two');
	// A base URL that changed in between is a different base.
	const base = new Ours('http://app.localhost/a/');
	assert.equal(new Ours('b', base).href, 'http://app.localhost/a/b');
	base.pathname = '/other/';
	assert.equal(new Ours('b', base).href, 'http://app.localhost/other/b');
	assert.throws(() => new Ours('x', 'not a base'), TypeError);
	assert.throws(() => new Ours('x', 'not a base'), TypeError, 'a failed parse is not cached as a success');
}

// Writing parts.
for (const [Class, Params] of [[URL, URLSearchParams], [Ours, OurParams]]) {
	const url = new Class('http://localhost/a?x=1&y=2&x=3#h');
	assert.deepEqual([...url.searchParams], [['x', '1'], ['y', '2'], ['x', '3']]);
	url.searchParams.set('x', 'one two');
	url.searchParams.append('z', 'a&b=c');
	url.searchParams.delete('y');
	assert.equal(url.href, 'http://localhost/a?x=one+two&z=a%26b%3Dc#h');
	url.search = '?fresh=1';
	assert.equal(url.searchParams.get('fresh'), '1');
	url.hash = '';
	url.pathname = '/b';
	assert.equal(String(url), 'http://localhost/b?fresh=1');
	assert.equal(JSON.stringify({ url }), '{"url":"http://localhost/b?fresh=1"}');

	const params = new Params({ b: '2', a: '1' });
	params.append('a', '0');
	params.sort();
	assert.equal(params.toString(), 'a=1&a=0&b=2');
	assert.deepEqual([params.get('a'), params.getAll('a'), params.has('c'), params.get('c'), params.size], ['1', ['1', '0'], false, null, 3]);
	assert.equal(new Params('?a=1+2&b=%C3%A9&c').toString(), 'a=1+2&b=%C3%A9&c=');
	assert.equal(new Params([['k', 'v']]).get('k'), 'v');
	assert.equal(Class.canParse('nope'), false);
	assert.equal(Class.canParse('/x', 'http://localhost'), true);

	// Ours makes `searchParams` when it is first asked for: whatever came
	// before, it has to start from the query as it is then.
	const later = new Class('http://localhost/?a=1');
	later.search = '?b=2';
	assert.equal(later.searchParams.get('b'), '2');
	const moved = new Class('http://localhost/');
	moved.href = 'http://localhost/x?c=3#h';
	assert.deepEqual([...moved.searchParams], [['c', '3']]);
	moved.search = '';
	assert.equal(moved.searchParams.size, 0);
	moved.searchParams.append('d', '4');
	assert.equal(moved.search, '?d=4');
	assert.ok(!Object.keys(moved).includes('searchParams'), 'searchParams is not an own property, as in Node');
}
// The caches have limits: least recently used goes first, a hit counts as a
// use, and what is stored (a failed parse is `null`) comes back as it was.
const cache = lru('test', 3);
cache.set('a', 1);
cache.set('b', null);
cache.set('c', 3);
assert.equal(cache.get('a'), 1, 'a hit');
cache.set('d', 4);
assert.deepEqual([cache.get('b'), cache.get('a'), cache.get('c'), cache.get('d')], [undefined, 1, 3, 4], 'b was the oldest after a was used');
cache.set('e', null);
assert.equal(cache.get('e'), null);
assert.equal(cache_sizes().test, 3);

// URL's parse cache holds 2,048, and past that still parses right.
for (let i = 0; i < 5000; i++) {
	const url = new Ours(`/items/${i}?n=${i}`, 'http://app.localhost/base/');
	if (i % 997 === 0) assert.equal(url.href, new URL(`/items/${i}?n=${i}`, 'http://app.localhost/base/').href);
}
assert.equal(cache_sizes().urls, 2048);
assert.equal(new Ours('/items/0?n=0', 'http://app.localhost/base/').href, 'http://app.localhost/items/0?n=0', 'one that was dropped');

console.log(`url: ok (${compared} URLs compared with Node's)`);
