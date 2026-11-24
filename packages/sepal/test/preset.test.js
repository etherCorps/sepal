// sepal() refuses an app with pages in src/routes and no src/main: pages are
// SvelteKit's, and the preset that builds them is sepalKit().

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { build } from 'vite';
import { sepal } from '../vite/vite.js';

const project = mkdtempSync(join(tmpdir(), 'sepal-preset-'));
try {
	mkdirSync(join(project, 'src/routes'), { recursive: true });
	writeFileSync(join(project, 'src/routes/+page.svelte'), '<h1>Home</h1>\n');
	const built = () =>
		build({
			root: project,
			configFile: false,
			logLevel: 'silent',
			plugins: [sepal({ app: { name: 'Routes' } })],
			build: { outDir: join(project, 'dist'), emptyOutDir: true }
		});
	await assert.rejects(built, /src\/routes and no src\/main.*sepalKit\(\)/s);
} finally {
	rmSync(project, { recursive: true, force: true });
}
console.log('preset: ok');
