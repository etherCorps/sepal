// What is left of sepalKit(): a stub that says how to move, for one release.
// The server files the adapter refuses, by name.

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { server_files } from '../vite/adapter.js';
import { sepalKit } from '../vite/sveltekit.js';

assert.throws(() => sepalKit(), /sepalKit\(\) is gone[\s\S]*adapter: adapter\(\)[\s\S]*customRenderer: 'sepal\/renderer'[\s\S]*pollInterval: 0[\s\S]*sepal\(\)[\s\S]*adapter\(\{ out \}\)/);
console.log('sveltekit stub: ok');

// Server code builds and then never runs in an app; the adapter refuses it.
const src = mkdtempSync(join(tmpdir(), 'sepal-server-files-'));
try {
	const server = ['routes/+page.server.ts', 'routes/a/+layout.server.js', 'routes/api/+server.ts', 'hooks.server.ts', 'lib/data.remote.ts'];
	const fine = ['routes/+page.ts', 'routes/+layout.ts', 'routes/+page.svelte', 'hooks.client.ts', 'lib/server.ts', 'lib/remote.ts', 'lib/my+server.md'];
	for (const file of [...server, ...fine]) {
		mkdirSync(dirname(join(src, file)), { recursive: true });
		writeFileSync(join(src, file), '');
	}
	assert.deepEqual(server_files(src).sort(), server.sort());
	assert.deepEqual(server_files(join(src, 'nowhere')), []);
} finally {
	rmSync(src, { recursive: true, force: true });
}
console.log('sveltekit server files: ok');
