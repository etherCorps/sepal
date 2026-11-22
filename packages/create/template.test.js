// The starter template, the way a user gets it: pack every package, run the
// packed `create-svelte-blitz`, install from the tarballs, type-check and build.
//
// When this platform's host binary has been built (`pnpm run build:host`), the
// app is also rendered through the CLI with no `SVELTE_BLITZ_HOST`: the whole
// path a user without Rust takes.
//
// Needs the npm registry, or a warm pnpm store, for the template's dependencies.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { bundled_host, platform } from '../host/index.js';

const packages = fileURLToPath(new URL('..', import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), 'svelte-blitz-template-'));
const app = join(scratch, 'My App');

// A host from the environment would hide a broken host package.
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
	// Pack everything: name -> tarball.
	/** @type {Record<string, string>} */
	const tarballs = {};
	for (const directory of readdirSync(packages)) {
		const before = new Set(readdirSync(scratch));
		run('pnpm', ['pack', '--pack-destination', scratch], join(packages, directory));
		const tarball = readdirSync(scratch).find((file) => file.endsWith('.tgz') && !before.has(file));
		assert(tarball, `pnpm pack wrote no tarball for packages/${directory}`);
		const { name } = JSON.parse(readFileSync(join(packages, directory, 'package.json'), 'utf8'));
		tarballs[name] = join(scratch, /** @type {string} */ (tarball));
	}

	// Run the create script out of its tarball, so a file missing from the package shows up here.
	const create = join(scratch, 'create');
	mkdirSync(create);
	run('tar', ['-xzf', tarballs['@ethercorps/create-svelte-blitz'], '-C', create], scratch);
	run('node', [join(create, 'package/index.js'), app], scratch);
	assert(existsSync(join(app, '.gitignore')), '.gitignore was not restored');

	const pkg = JSON.parse(readFileSync(join(app, 'package.json'), 'utf8'));
	assert(pkg.name === 'my-app', `package name is ${pkg.name}`);
	for (const [group, name] of [
		['dependencies', '@ethercorps/svelte-blitz'],
		['devDependencies', '@ethercorps/svelte-blitz-vite'],
		['devDependencies', '@ethercorps/svelte-blitz-cli']
	]) {
		assert(/^\^\d/.test(pkg[group][name]), `${name} is not a versioned entry in ${group}`);
	}
	assert(readFileSync(join(app, 'vite.config.ts'), 'utf8').includes('name: "My App"'), 'app name not filled in');

	// Nothing is published yet: the overrides stand in for the registry. Its own
	// workspace file also keeps the project out of this repository's workspace.
	const overrides = Object.entries(tarballs)
		.filter(([name]) => name !== '@ethercorps/create-svelte-blitz')
		.map(([name, tarball]) => `  "${name}": "file:${tarball}"\n`);
	writeFileSync(join(app, 'pnpm-workspace.yaml'), `packages: []\noverrides:\n${overrides.join('')}`);
	run('pnpm', ['install', '--prefer-offline'], app);

	run('pnpm', ['run', 'check'], app);
	run('pnpm', ['run', 'build'], app);
	const manifest = JSON.parse(readFileSync(join(app, 'dist/app.json'), 'utf8'));
	assert(manifest.name === 'My App' && manifest.id === 'com.example.my-app', 'manifest not written from the template');
	assert(existsSync(join(app, 'dist/app.js')) && existsSync(join(app, 'dist/icon.png')), 'bundle or icon missing');
	assert(existsSync(join(app, 'dist/logo.svg')), 'static/ was not copied into the build');
	// The template is file-routed: the About page only exists in the bundle if src/routes was read.
	assert(readFileSync(join(app, 'dist/app.js'), 'utf8').includes('back home'), 'routes were not bundled');

	if (existsSync(bundled_host())) {
		run('pnpm', ['exec', 'svelte-blitz', 'screenshot', join(scratch, 'shot.png')], app);
		assert(existsSync(join(scratch, 'shot.png')), 'the CLI did not render a screenshot');
		console.log(`template: rendered with the packaged ${platform()} host`);
	} else {
		console.log(`template: no built host for ${platform()}; skipped the render`);
	}
	console.log('template: ok');
} finally {
	rmSync(scratch, { recursive: true, force: true });
}
