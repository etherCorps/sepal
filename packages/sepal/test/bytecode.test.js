// Compiling the app's modules to bytecode after a build: calyx is asked to,
// what it leaves as JavaScript is passed on, and a calyx that cannot is a
// warning and not a failed build. Scripts stand in for calyx.

import assert from 'node:assert/strict';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { compile_bytecode } from '../vite/app.js';

if (process.platform === 'win32') {
	console.log('bytecode: skipped on Windows (the stand-in calyx is a shell script)');
	process.exit(0);
}

const scratch = mkdtempSync(join(tmpdir(), 'sepal-bytecode-'));
const log = join(scratch, 'calyx.log');

/** A stand-in calyx running `body`. @param {string} name @param {string} body */
function host(name, body) {
	const path = join(scratch, name);
	writeFileSync(path, `#!/bin/sh\necho "$*" > "${log}"\n${body}\n`);
	chmodSync(path, 0o755);
	return path;
}

/** @param {Record<string, string>} files */
function folder(files) {
	const dir = mkdtempSync(join(scratch, 'app-'));
	for (const [file, text] of Object.entries(files)) {
		mkdirSync(join(dir, file, '..'), { recursive: true });
		writeFileSync(join(dir, file), text);
	}
	return dir;
}

try {
	/** @type {string[]} */
	let warnings = [];
	const warn = (/** @type {string} */ message) => void warnings.push(message);

	// calyx compiles; a module that stays JavaScript is a warning.
	process.env.SEPAL_HOST = host(
		'calyx',
		'echo "classic.js stays JavaScript: SyntaxError: invalid keyword" >&2\necho "calyx: compiled 2 modules to bytecode in 1 ms (0 up to date, 0 removed)"'
	);
	const app = folder({ 'main.js': '', 'classic.js': '' });
	assert.equal(compile_bytecode(app, warn), 'calyx: compiled 2 modules to bytecode in 1 ms (0 up to date, 0 removed)');
	assert.equal(readFileSync(log, 'utf8').trim(), `--compile ${app}`);
	assert.deepEqual(warnings, ['sepal: classic.js stays JavaScript: SyntaxError: invalid keyword']);

	// A calyx from before --compile takes it for the app folder: said as
	// that, and the app loads its JavaScript.
	warnings = [];
	const old = host('old-calyx', 'echo "--compile is not an app folder: give the directory the build wrote, with index.html in it" >&2\nexit 1');
	process.env.SEPAL_HOST = old;
	assert.equal(compile_bytecode(app, warn), null);
	assert.deepEqual(warnings, [`sepal: ${old} is a calyx from before \`--compile\`, so the app's modules were not compiled to bytecode and it loads their JavaScript. A current calyx compiles them.`]);

	// Any other failure is passed on as calyx said it.
	warnings = [];
	process.env.SEPAL_HOST = host('broken-calyx', 'echo "cannot read the app folder" >&2\nexit 1');
	assert.equal(compile_bytecode(app, warn), null);
	assert.match(warnings[0], /compiling the app's modules to bytecode failed, so it loads their JavaScript: cannot read the app folder/);

	// So does a calyx that is not there.
	warnings = [];
	process.env.SEPAL_HOST = join(scratch, 'missing');
	assert.equal(compile_bytecode(app, warn), null);
	assert.match(warnings[0], /failed, so it loads their JavaScript: .*ENOENT/);
} finally {
	delete process.env.SEPAL_HOST;
	rmSync(scratch, { recursive: true, force: true });
}
console.log('bytecode: ok');
