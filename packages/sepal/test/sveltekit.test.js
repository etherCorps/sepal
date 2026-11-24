// What the SvelteKit adapter does to the stylesheets it merges into app.css.

import assert from 'node:assert/strict';
import { rebase_css } from '../vite/sveltekit.js';

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
