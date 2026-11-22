// The starter template, the way a user gets it: pack the package, run
// `svelte-blitz create`, install from the tarball, type-check and build.
//
// When this platform's host package has been built (`pnpm run build:host`),
// it is packed and installed too, and the app is rendered through the CLI
// with no `SVELTE_BLITZ_HOST`: the whole path a user without Rust takes.
//
// Needs the npm registry, or a warm pnpm store, for the template's dependencies.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { host_package } from '../src/host.js';

const package_root = fileURLToPath(new URL('..', import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), 'svelte-blitz-template-'));
const app = join(scratch, 'My App');

// A host from the environment would hide a broken platform package.
const env = { ...process.env };
delete env.SVELTE_BLITZ_HOST;

/** @param {string} command @param {string[]} args @param {string} cwd */
const run = (command, args, cwd) =>
	execFileSync(command, args, { cwd, env, stdio: ['ignore', 'pipe', 'inherit'], encoding: 'utf8' });
/** @param {unknown} condition @param {string} message */
const assert = (condition, message) => {
	if (!condition) throw new Error(message);
};

try {
	run('pnpm', ['pack', '--pack-destination', scratch], package_root);
	const tarball = readdirSync(scratch).find((file) => file.endsWith('.tgz'));
	assert(tarball, 'pnpm pack wrote no tarball');

	const host = host_package();
	const host_root = join(package_root, '..', host.name.replace('@ethercorps/svelte-blitz', 'host'));
	const has_host = existsSync(join(host_root, host.file));
	let overrides = '';
	if (has_host) {
		run('pnpm', ['pack', '--pack-destination', scratch], host_root);
		const host_tarball = readdirSync(scratch).find((file) => file.endsWith('.tgz') && file !== tarball);
		assert(host_tarball, 'pnpm pack wrote no host tarball');
		overrides = `overrides:\n  "${host.name}": "file:${join(scratch, /** @type {string} */ (host_tarball))}"\n`;
	}

	run('node', [join(package_root, 'src/cli.js'), 'create', app], scratch);
	assert(existsSync(join(app, '.gitignore')), '.gitignore was not restored');

	const pkg_path = join(app, 'package.json');
	const pkg = JSON.parse(readFileSync(pkg_path, 'utf8'));
	assert(pkg.name === 'my-app', `package name is ${pkg.name}`);
	assert(/^\^\d/.test(pkg.dependencies['@ethercorps/svelte-blitz']), 'the package is not a versioned dependency');
	assert(readFileSync(join(app, 'vite.config.ts'), 'utf8').includes('name: "My App"'), 'app name not filled in');

	// Not published yet: install the tarball where a user would get the registry version.
	pkg.dependencies['@ethercorps/svelte-blitz'] = `file:${join(scratch, /** @type {string} */ (tarball))}`;
	writeFileSync(pkg_path, JSON.stringify(pkg, null, '\t'));
	// Outside the workspace on purpose: this is a project of its own.
	// The override stands in for the registry, where the host package is not published yet.
	writeFileSync(join(app, 'pnpm-workspace.yaml'), 'packages: []\n' + overrides);
	run('pnpm', ['install', '--prefer-offline'], app);

	run('pnpm', ['run', 'check'], app);
	run('pnpm', ['run', 'build'], app);
	const manifest = JSON.parse(readFileSync(join(app, 'dist/app.json'), 'utf8'));
	assert(manifest.name === 'My App' && manifest.id === 'com.example.my-app', 'manifest not written from the template');
	assert(existsSync(join(app, 'dist/app.js')) && existsSync(join(app, 'dist/icon.png')), 'bundle or icon missing');

	if (has_host) {
		run('pnpm', ['exec', 'svelte-blitz', 'screenshot', join(scratch, 'shot.png')], app);
		assert(existsSync(join(scratch, 'shot.png')), 'the CLI did not render a screenshot');
		console.log(`rendered with the host from ${host.name}`);
	} else {
		console.log(`no built host package for this platform (${host.name}); skipped the render`);
	}
	console.log('ok');
} finally {
	rmSync(scratch, { recursive: true, force: true });
}
