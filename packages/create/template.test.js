// The starter template, the way a user gets it: pack every package, run the
// packed `create-sepal`, install from the tarballs, type-check and build.
// Once for each router it offers.
//
// When this platform's host binary has been built (`pnpm run build:host`), the
// app is also rendered through the CLI with no `SEPAL_HOST`: the whole
// path a user without Rust takes.
//
// Needs the npm registry, or a warm pnpm store, for the template's dependencies.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { platform } from '../host/index.js';

const packages = fileURLToPath(new URL('..', import.meta.url));
const scratch = mkdtempSync(join(tmpdir(), 'sepal-template-'));

// A host from the environment would hide a broken host package.
const env = { ...process.env };
delete env.SEPAL_HOST;

/** @param {string} command @param {string[]} args @param {string} cwd */
const run = (command, args, cwd) =>
	execFileSync(command, args, {
		cwd,
		env,
		stdio: ['ignore', 'pipe', 'inherit'],
		encoding: 'utf8',
		// On Windows pnpm is a .cmd file, which only a shell can start. Nothing
		// passed to it here has a space in it.
		shell: process.platform === 'win32' && command === 'pnpm'
	});
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
	run('tar', ['-xzf', tarballs['create-sepal'], '-C', create], scratch);
	// No terminal here, so nothing is asked: no flag means the built-in router.
	for (const [router, flags] of /** @type {[string, string[]][]} */ ([
		['built-in', []],
		['sveltekit', ['--router', 'sveltekit']]
	])) {
		const app = join(scratch, router, 'My App');
		mkdirSync(join(scratch, router));
		run('node', [join(create, 'package/index.js'), app, ...flags], scratch);
		assert(existsSync(join(app, '.gitignore')), '.gitignore was not restored');

		const pkg = JSON.parse(readFileSync(join(app, 'package.json'), 'utf8'));
		assert(pkg.name === 'my-app', `package name is ${pkg.name}`);
		assert(('@sveltejs/kit' in pkg.devDependencies) === (router === 'sveltekit'), `wrong starter for the ${router} router`);
		assert(/^\^\d/.test(pkg.dependencies.sepal), 'sepal is not a versioned entry in dependencies');
		// SvelteKit 3 has no `$lib`: the starter declares `#lib` itself.
		if (router === 'sveltekit') assert(pkg.imports?.['#lib/*'] === './src/lib/*', 'the SvelteKit starter does not declare #lib');
		assert(readFileSync(join(app, 'sepal.config.ts'), 'utf8').includes('name: "My App"'), 'app name not filled in');

		// What is tested is this checkout, not what is published: the overrides stand in for the registry. Its own
		// workspace file also keeps the project out of this repository's workspace.
		const overrides = Object.entries(tarballs)
			.filter(([name]) => name !== 'create-sepal')
			// `/` on Windows too: a backslash in a quoted YAML string starts an escape.
			.map(([name, tarball]) => `  "${name}": "file:${tarball.replaceAll('\\', '/')}"\n`);
		writeFileSync(join(app, 'pnpm-workspace.yaml'), `packages: []\noverrides:\n${overrides.join('')}`);
		run('pnpm', ['install', '--prefer-offline'], app);

		// Every Sepal package came from the tarballs, none from the registry, and of
		// the four platform packages only this machine's was installed.
		const ours = readdirSync(join(app, 'node_modules/.pnpm')).filter((entry) => /^(sepal|@ethercorps\+calyx[a-z0-9-]*)@/.test(entry));
		for (const entry of ours) assert(entry.includes('@file+'), `${entry} was not installed from a local tarball`);
		const platforms = ours.filter((entry) => entry.startsWith('@ethercorps+calyx-')).map((entry) => entry.split('@')[1]);
		assert(platforms.length === 1 && platforms[0] === `ethercorps+calyx-${platform()}`, `platform packages installed: ${platforms.join(', ') || 'none'}`);

		run('pnpm', ['run', 'check'], app);
		run('pnpm', ['run', 'build'], app);
		const manifest = JSON.parse(readFileSync(join(app, 'dist/app.json'), 'utf8'));
		assert(manifest.name === 'My App' && manifest.id === 'com.example.my-app', 'manifest not written from the template');
		assert(existsSync(join(app, 'dist/app.js')), 'bundle missing');
		// The starter has no icon of its own, so the packed package has to supply one.
		const sepal_icon = readFileSync(join(packages, 'sepal/vite/icon.png'));
		assert(readFileSync(join(app, 'dist/icon.png')).equals(sepal_icon), 'the build does not have the Sepal icon');
		assert(existsSync(join(app, 'dist/logo.svg')), 'static/ was not copied into the build');
		// The template is file-routed: the About page only exists in the bundle if src/routes was read.
		const bundle = readFileSync(join(app, 'dist/app.js'), 'utf8');
		assert(bundle.includes('back home'), 'routes were not bundled');
		// SvelteKit's router brings a word of its own that the built-in one never says.
		assert(bundle.includes('__sveltekit') === (router === 'sveltekit'), `not built with the ${router} router`);
		assert(readFileSync(join(app, 'dist/app.css'), 'utf8').includes('.bg-indigo-600'), 'Tailwind classes are missing from app.css');

		if (existsSync(join(packages, `calyx-${platform()}`, 'bin'))) {
			const shot = join(scratch, `${router}.png`);
			run('pnpm', ['exec', 'sepal', 'screenshot', shot], app);
			assert(existsSync(shot), 'the CLI did not render a screenshot');
			console.log(`template (${router}): rendered with the packaged ${platform()} host`);
		} else {
			console.log(`template (${router}): no built host for ${platform()}; skipped the render`);
		}
	}
	console.log('template: ok');
} finally {
	rmSync(scratch, { recursive: true, force: true });
}
