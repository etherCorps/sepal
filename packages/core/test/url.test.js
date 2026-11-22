// Our URL and URLSearchParams against Node's, on the kinds of URL an app meets.
import assert from 'node:assert/strict';
import { URL as Ours, URLSearchParams as OurParams } from '../src/url.js';

const PARTS = ['href', 'origin', 'protocol', 'username', 'password', 'host', 'hostname', 'port', 'pathname', 'search', 'hash'];
const describe = (url) => Object.fromEntries(PARTS.map((part) => [part, url[part]]));

const bases = [undefined, 'http://localhost/', 'https://example.com:8443/a/b/c?x=1#top', 'http://app.localhost/items/42', 'a://', 'custom://host/a/b', 'a:'];
const inputs = [
	'http://localhost/', 'https://Example.COM:443/Path?Q=1#Frag', 'http://localhost:80/a/../b/./c', 'http://user:pw@host.test:8080/x',
	'/', '/a/b', 'a/b', './a', '../a', '../../a/b/', '.', '..', '', '?q=1', '?', '#hash', '#', 'a?b#c', '/a/b/../c/./d/', '//other.test/x',
	'/with space/and"quote', '/ünï/cödé?kéy=välue#frâg', '/a?b=c&d=e f', 'items/7?tab=two#notes', 'http://localhost:3000', 'not a url', 'http://', 'mailto:x@y.z', 'a://', 'custom://host', 'custom://host/p?q#h', 'a:', 'mailto:x@y.z?subject=hi#f'
];
let compared = 0;
for (const base of bases) {
	for (const input of inputs) {
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
	}
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
}
console.log(`url: ok (${compared} URLs compared with Node's)`);
