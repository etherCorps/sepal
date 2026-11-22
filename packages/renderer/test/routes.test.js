// The route scanner: which files become which routes, in which order, and
// what is refused. Plain Node, no build.

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { generate_routes, route_pattern, scan_routes } from '../src/routes.js';

const scratch = mkdtempSync(join(tmpdir(), 'svelte-blitz-routes-'));

/** Make a routes directory holding these files. @param {string} name @param {string[]} files */
function tree(name, files) {
	const dir = join(scratch, name);
	for (const file of files) {
		mkdirSync(dirname(join(dir, file)), { recursive: true });
		writeFileSync(join(dir, file), '');
	}
	return dir;
}

/** Which route a path lands on, and with which parameters. @param {ReturnType<typeof scan_routes>['routes']} routes @param {string} path */
function match(routes, path) {
	for (const route of routes) {
		const { pattern, names } = route_pattern(route.segments);
		const found = new RegExp(pattern).exec(path);
		if (found) return { id: route.id, params: Object.fromEntries(names.map((name, i) => [name, found[i + 1] ?? ''])) };
	}
	return null;
}

try {
	const dir = tree('app', [
		'+layout.svelte',
		'+layout.ts',
		'+error.svelte',
		'+page.svelte',
		'about/+page.svelte',
		'items/+page.svelte',
		'items/+page.ts',
		'items/new/+page.svelte',
		'items/[id]/+page.svelte',
		'items/[id]/+error.svelte',
		'items/[id]/edit/+page.svelte',
		'docs/[...path]/+page.svelte',
		'(app)/+layout.svelte',
		'(app)/settings/+page.svelte',
		'files/v[major].[minor]/+page.svelte',
		'lib-like/helper.ts'
	]);
	const { routes, fallback, files } = scan_routes(dir);

	assert.deepEqual(match(routes, '/'), { id: '/', params: {} });
	assert.deepEqual(match(routes, '/about'), { id: '/about', params: {} });
	// Fixed text beats a parameter, a parameter beats a rest parameter.
	assert.deepEqual(match(routes, '/items/new'), { id: '/items/new', params: {} });
	assert.deepEqual(match(routes, '/items/42'), { id: '/items/[id]', params: { id: '42' } });
	assert.deepEqual(match(routes, '/items/42/edit'), { id: '/items/[id]/edit', params: { id: '42' } });
	assert.deepEqual(match(routes, '/docs/a/b/c'), { id: '/docs/[...path]', params: { path: 'a/b/c' } });
	assert.deepEqual(match(routes, '/docs'), { id: '/docs/[...path]', params: { path: '' } });
	// A group adds its layout and nothing to the path.
	assert.deepEqual(match(routes, '/settings'), { id: '/(app)/settings', params: {} });
	assert.deepEqual(match(routes, '/files/v2.10'), { id: '/files/v[major].[minor]', params: { major: '2', minor: '10' } });
	assert.equal(match(routes, '/nope'), null);
	assert.equal(match(routes, '/items/42/other'), null);
	assert.equal(match(routes, '/files/v2x10'), null, 'the dot is literal');

	const by_id = Object.fromEntries(routes.map((route) => [route.id, route]));
	assert.equal(by_id['/about'].layouts.length, 1);
	assert.equal(by_id['/(app)/settings'].layouts.length, 2);
	assert.ok(by_id['/'].layouts[0].component?.endsWith('+layout.svelte'));
	assert.ok(by_id['/'].layouts[0].module?.endsWith('+layout.ts'));
	assert.ok(by_id['/items'].page.module?.endsWith('items/+page.ts'));
	// The nearest +error.svelte, and how many layouts stay around it.
	assert.ok(by_id['/about'].error?.endsWith('routes-') === false && by_id['/about'].error?.endsWith('app/+error.svelte'));
	// The root error page renders inside the root layout, and only that one.
	assert.equal(by_id['/about'].error_depth, 1);
	assert.equal(by_id['/(app)/settings'].error_depth, 1);
	assert.ok(by_id['/items/[id]/edit'].error?.endsWith('items/[id]/+error.svelte'));
	assert.equal(by_id['/items/[id]/edit'].error_depth, 1);
	assert.equal(fallback.layouts.length, 1);
	assert.ok(fallback.error?.endsWith('app/+error.svelte'));
	assert.ok(!files.some((file) => file.endsWith('helper.ts')), 'only + files are route files');

	const { code } = generate_routes(dir);
	assert.match(code, /import \* as _\d+ from ".*\+layout\.ts";/);
	assert.match(code, /id: "\/items\/\[id\]", pattern: new RegExp\("\^\/items\/\(\[\^\/\]\+\)\$"\), names: \["id"\]/);

	// Things that cannot work are build errors, not silent omissions.
	const refused = [
		[['+page.server.ts', '+page.svelte'], /needs a server/],
		[['api/+server.ts'], /needs a server/],
		[['+layout.server.js'], /needs a server/],
		[['a/+page.ts'], /has no \+page\.svelte/],
		[['[[lang]]/+page.svelte'], /optional parameters/],
		[['[id=int]/+page.svelte'], /matchers/],
		[['+page.md'], /not a route file/],
		[['a/+page.svelte', '(g)/a/+page.svelte'], /are both the page for \/a/]
	];
	refused.forEach(([paths, message], index) => {
		assert.throws(() => scan_routes(tree(`bad${index}`, /** @type {string[]} */ (paths))), /** @type {RegExp} */ (message));
	});

	console.log('routes: ok');
} finally {
	rmSync(scratch, { recursive: true, force: true });
}
