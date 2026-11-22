#!/usr/bin/env node
// svelte-blitz: run, bundle and screenshot an app, and start a new project.
// Building is Vite's job (`vite build`, and `vite build --watch` for dev).

import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { find_host } from './host.js';

const USAGE = `usage:
  svelte-blitz create <directory>                  start a new project
  svelte-blitz run [dist]                          open the built app in a window
  svelte-blitz bundle [dist] [out-dir] [--sign <identity>] [--notarize <keychain-profile>]
                                                   build a macOS .app
  svelte-blitz screenshot <out.png> [dist] [WIDTHxHEIGHT]

Build first with \`vite build\`. For dev, \`vite build --watch\` opens the app
and reloads it on every save.`;

/** @param {string} message @returns {never} */
function fail(message) {
	console.error(message);
	process.exit(1);
}

/** Run the host with these arguments and exit with its status. @param {string[]} args @returns {never} */
function host(args) {
	const bin = find_host();
	if (bin === null) {
		fail(
			`svelte-blitz: no host binary for ${process.platform}-${process.arch}.\n` +
				'Build one from crates/host and point SVELTE_BLITZ_HOST at it.'
		);
	}
	const { status, error } = spawnSync(bin, args, { stdio: 'inherit' });
	if (error) fail(`svelte-blitz: cannot start ${bin}: ${error.message}`);
	process.exit(status ?? 1);
}

/** @param {string | undefined} directory */
function create(directory) {
	if (!directory) fail(USAGE);
	const target = resolve(directory);
	if (existsSync(target) && readdirSync(target).length > 0) {
		fail(`svelte-blitz: ${directory} exists and is not empty`);
	}

	const name = basename(target);
	const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'app';
	const package_root = fileURLToPath(new URL('..', import.meta.url));
	const own = JSON.parse(readFileSync(join(package_root, 'package.json'), 'utf8'));

	cpSync(join(package_root, 'template'), target, { recursive: true });
	// npm leaves `.gitignore` out of published packages, so it ships under another name.
	renameSync(join(target, 'gitignore'), join(target, '.gitignore'));

	const pkg_path = join(target, 'package.json');
	const pkg = JSON.parse(readFileSync(pkg_path, 'utf8'));
	pkg.name = slug;
	pkg.dependencies[own.name] = `^${own.version}`;
	writeFileSync(pkg_path, JSON.stringify(pkg, null, '\t') + '\n');

	const config_path = join(target, 'vite.config.ts');
	const config = readFileSync(config_path, 'utf8')
		.replace("'__NAME__'", JSON.stringify(name))
		.replace("'__ID__'", JSON.stringify(`com.example.${slug}`));
	writeFileSync(config_path, config);

	console.log(`Created ${directory}.\n\n  cd ${directory}\n  pnpm install\n  pnpm dev\n`);
}

const [command, ...args] = process.argv.slice(2);
// `bundle --sign x` with no directory still means `dist`.
const dist = args[0] !== undefined && !args[0].startsWith('--') ? args : ['dist', ...args];

switch (command) {
	case 'create':
		create(args[0]);
		break;
	case 'run':
		host(dist);
	case 'bundle':
		host(['--bundle', ...dist]);
	case 'screenshot':
		if (!args[0]) fail(USAGE);
		host(['--screenshot', args[0], ...(args.length > 1 ? args.slice(1) : ['dist'])]);
	default:
		fail(USAGE);
}
