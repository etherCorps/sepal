// The app icon the preset writes into the build: the app's own
// `static/icon.png` when it has one, the Sepal icon when it has none.

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import { sepal } from '../vite/vite.js';

/** @param {string} path */
const here = (path) => fileURLToPath(new URL(path, import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), 'sepal-icon-'));

/**
 * Build the custom-router fixture, which has no `static/` of its own, and
 * return the icon and the manifest written.
 * @param {string} name
 * @param {string} [public_dir] a static directory to use instead
 */
async function built(name, public_dir) {
	const out = join(scratch, name);
	await build({
		root: here('./custom-router'),
		...(public_dir ? { publicDir: public_dir } : {}),
		configFile: false,
		logLevel: 'silent',
		plugins: [sepal({ router: false, app: { name: 'Icon' } })],
		build: { outDir: out, emptyOutDir: true }
	});
	return { png: readFileSync(join(out, 'icon.png')), manifest: JSON.parse(readFileSync(join(out, 'app.json'), 'utf8')) };
}

try {
	// The package ships the brand file as it is.
	const sepal_icon = readFileSync(here('../vite/icon.png'));
	assert.ok(sepal_icon.equals(readFileSync(here('../../../assets/brand/sepal-app-icon-1024.png'))), 'vite/icon.png is not assets/brand/sepal-app-icon-1024.png');

	const own = await built('own', here('./custom-icon'));
	assert.ok(own.png.equals(readFileSync(here('./custom-icon/icon.png'))), "the app's own static/icon.png is used");
	assert.equal(own.manifest.icon, 'icon.png');

	const none = await built('none');
	assert.ok(none.png.equals(sepal_icon), 'an app with no icon gets the Sepal icon');
	assert.equal(none.manifest.icon, 'icon.png');

	console.log('icon: ok');
} finally {
	rmSync(scratch, { recursive: true, force: true });
}
