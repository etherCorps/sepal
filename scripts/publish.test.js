// The rules scripts/publish.js publishes by: which dist-tags a version gets,
// and the order packages go out in.

import assert from 'node:assert/strict';
import { dist_tags, stage, tarball_url } from './publish.js';

// No stable release yet: a prerelease is its own tag and latest.
assert.deepEqual(dist_tags('0.1.0-alpha.1', false), ['alpha', 'latest']);
assert.deepEqual(dist_tags('0.2.0-beta.3', false), ['beta', 'latest']);
// Once one exists, prereleases leave latest alone.
assert.deepEqual(dist_tags('1.1.0-alpha.0', true), ['alpha']);
// A stable release is latest.
assert.deepEqual(dist_tags('1.0.0', false), ['latest']);
assert.deepEqual(dist_tags('1.0.0', true), ['latest']);

// Platform packages first, then calyx, sepal, create-sepal.
assert.equal(stage('@ethercorps/calyx-darwin-arm64'), 0);
assert.equal(stage('@ethercorps/calyx-linux-arm64-gnu'), 0);
assert.deepEqual(['@ethercorps/calyx', 'sepal', 'create-sepal'].map(stage), [1, 2, 3]);
assert.throws(() => stage('something-else'));

// Tarballs are asked for directly, scoped or not.
assert.equal(tarball_url('sepal', '0.1.0-alpha.2'), 'https://registry.npmjs.org/sepal/-/sepal-0.1.0-alpha.2.tgz');
assert.equal(
	tarball_url('@ethercorps/calyx-darwin-x64', '0.1.0-alpha.2'),
	'https://registry.npmjs.org/@ethercorps/calyx-darwin-x64/-/calyx-darwin-x64-0.1.0-alpha.2.tgz'
);

console.log('publish rules: ok');
