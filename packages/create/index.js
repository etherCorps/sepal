#!/usr/bin/env node
// Start a svelte-blitz project: `pnpm create @ethercorps/svelte-blitz my-app`.
// Copies the starter in ./template and fills in the project's name.

import { cpSync, existsSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** @param {string} message @returns {never} */
function fail(message) {
	console.error(message);
	process.exit(1);
}

const directory = process.argv[2];
if (!directory) fail('usage: create-svelte-blitz <directory>');

const target = resolve(directory);
if (existsSync(target) && readdirSync(target).length > 0) {
	fail(`${directory} exists and is not empty`);
}

const name = basename(target);
const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'app';
const package_root = fileURLToPath(new URL('.', import.meta.url));
const own = JSON.parse(readFileSync(join(package_root, 'package.json'), 'utf8'));

cpSync(join(package_root, 'template'), target, { recursive: true });
// npm leaves `.gitignore` out of published packages, so it ships under another name.
renameSync(join(target, 'gitignore'), join(target, '.gitignore'));

const pkg_path = join(target, 'package.json');
const pkg = JSON.parse(readFileSync(pkg_path, 'utf8'));
pkg.name = slug;
// Every svelte-blitz package is released together under one version.
for (const group of [pkg.dependencies, pkg.devDependencies]) {
	for (const dependency of Object.keys(group)) {
		if (dependency.startsWith('@ethercorps/svelte-blitz')) group[dependency] = `^${own.version}`;
	}
}
writeFileSync(pkg_path, JSON.stringify(pkg, null, '\t') + '\n');

const config_path = join(target, 'svelte-blitz.config.ts');
const config = readFileSync(config_path, 'utf8')
	.replace("'__NAME__'", JSON.stringify(name))
	.replace("'__ID__'", JSON.stringify(`com.example.${slug}`));
writeFileSync(config_path, config);

console.log(`Created ${directory}.\n\n  cd ${directory}\n  pnpm install\n  pnpm dev\n`);
