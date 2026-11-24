// The `sepal` command and the calyx it runs: a binary from a platform package
// of another version is refused, SEPAL_HOST is taken as it is, a missing
// platform package is named, and on macOS an app with a name runs from a
// bundle, so that it is not called "calyx" in the Dock. Runs the real cli.js and calyx index.js out of a
// node_modules laid out as an install would, with a script standing in for the
// binary.

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { platform_package } from '../../host/index.js';

if (process.platform === 'win32') {
	console.log('cli: skipped on Windows (the stand-in binary is a shell script)');
	process.exit(0);
}

/** @param {string} path */
const here = (path) => fileURLToPath(new URL(path, import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), 'sepal-cli-'));
const modules = join(scratch, 'node_modules');
const platform_dir = join(modules, platform_package());

/** @param {string} dir @param {object} manifest */
function write_package(dir, manifest) {
	mkdirSync(dir, { recursive: true });
	writeFileSync(join(dir, 'package.json'), JSON.stringify(manifest));
}

/** Lay out sepal 1.0.0, its calyx, and this machine's platform package at `version` (none for `null`). @param {string | null} version */
function install(version) {
	rmSync(modules, { recursive: true, force: true });
	write_package(join(modules, 'sepal'), { name: 'sepal', version: '1.0.0', type: 'module' });
	copyFileSync(here('../cli.js'), join(modules, 'sepal/cli.js'));
	const calyx = JSON.parse(readFileSync(here('../../host/package.json'), 'utf8'));
	write_package(join(modules, '@ethercorps/calyx'), { ...calyx, version: '1.0.0' });
	copyFileSync(here('../../host/index.js'), join(modules, '@ethercorps/calyx/index.js'));
	if (version === null) return;
	write_package(platform_dir, { name: platform_package(), version });
	mkdirSync(join(platform_dir, 'bin'));
	// `--bundle <dist> <out>` makes "<out>/App.app", whose binary says it ran; anything else just says so.
	writeFileSync(
		join(platform_dir, 'bin/calyx'),
		'#!/bin/sh\n' +
			'if [ "$1" = "--bundle" ]; then\n' +
			'  mkdir -p "$3/App.app/Contents/MacOS"\n' +
			'  printf \'#!/bin/sh\\necho "bundled ran: $*"\\n\' > "$3/App.app/Contents/MacOS/App"\n' +
			'  chmod +x "$3/App.app/Contents/MacOS/App"\n' +
			'  echo "$3/App.app"\n' +
			'  exit 0\n' +
			'fi\n' +
			'echo "calyx ran: $*"\n'
	);
	chmodSync(join(platform_dir, 'bin/calyx'), 0o755);
}

/** @param {Record<string, string>} [extra] */
function sepal(extra = {}) {
	const env = { ...process.env, ...extra };
	if (!('SEPAL_HOST' in extra)) delete env.SEPAL_HOST;
	return spawnSync(process.execPath, [join(modules, 'sepal/cli.js'), 'run', 'dist'], { cwd: scratch, env, encoding: 'utf8' });
}

try {
	// Same version: the binary runs.
	install('1.0.0');
	let ran = sepal();
	assert.equal(ran.status, 0, ran.stderr);
	assert.equal(ran.stdout.trim(), 'calyx ran: dist');

	// An app with a name: on macOS through a throwaway bundle in node_modules/.sepal, elsewhere as it is.
	mkdirSync(join(scratch, 'dist'), { recursive: true });
	writeFileSync(join(scratch, 'dist/app.json'), JSON.stringify({ name: 'App' }));
	ran = sepal();
	assert.equal(ran.status, 0, ran.stderr);
	assert.equal(ran.stdout.trim(), process.platform === 'darwin' ? 'bundled ran: dist' : 'calyx ran: dist');
	rmSync(join(scratch, 'dist'), { recursive: true });

	// Another version: refused, saying which and what to reinstall, and nothing runs.
	install('0.9.0');
	ran = sepal();
	assert.equal(ran.status, 1);
	assert.equal(ran.stdout, '');
	assert.match(ran.stderr, new RegExp(`this is sepal 1\\.0\\.0, but its calyx binary \\(${platform_package()}\\) is 0\\.9\\.0`));
	assert.match(ran.stderr, /Reinstall sepal@1\.0\.0/);

	// SEPAL_HOST is the user's own calyx: no version to compare, it runs.
	ran = sepal({ SEPAL_HOST: join(platform_dir, 'bin/calyx') });
	assert.equal(ran.status, 0, ran.stderr);
	assert.equal(ran.stdout.trim(), 'calyx ran: dist');

	// No platform package at all: the package this machine needs, and why it may be missing.
	install(null);
	ran = sepal();
	assert.equal(ran.status, 1);
	assert.match(ran.stderr, new RegExp(`this machine needs ${platform_package()}, and it is not installed`));
	assert.match(ran.stderr, /optional dependencies were turned off/);
	assert.match(ran.stderr, /copied from another OS or CPU/);

	console.log('cli: ok');
} finally {
	rmSync(scratch, { recursive: true, force: true });
}
