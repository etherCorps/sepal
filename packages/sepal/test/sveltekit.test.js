// What the SvelteKit adapter does to the stylesheets it merges into app.css,
// and the server files it refuses.

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { rebase_css, server_files } from '../vite/sveltekit.js';

const dir = '_app/immutable/assets';
// A font next to the stylesheet, with or without quotes or `./`.
assert.equal(rebase_css('src:url(./inter.Dx4k.woff2) format("woff2")', dir), `src:url(${dir}/inter.Dx4k.woff2) format("woff2")`);
assert.equal(rebase_css(`url("a.woff2")`, dir), `url("${dir}/a.woff2")`);
assert.equal(rebase_css(`url('../img/b.png')`, dir), `url('_app/immutable/img/b.png')`);
// Left alone: data: URLs, other schemes, root paths and fragments.
for (const kept of ['url(data:font/woff2;base64,AAAA)', 'url(https://example.com/a.png)', 'url(/logo.svg)', 'url(#clip)']) {
	assert.equal(rebase_css(kept, dir), kept);
}

console.log('sveltekit css: ok');

// Server code builds and then never runs in an app; the build refuses it.
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
