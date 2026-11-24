#!/usr/bin/env node
// Start a Sepal project: `pnpm create sepal my-app`.
// Copies the starter in ./template and fills in the project's name.
//
// The starter comes with one of two routers, both with pages as files in
// `src/routes`: the one built into Sepal, or SvelteKit's. Asked for
// when run in a terminal; `--router built-in` or `--router sveltekit` (or
// `--sveltekit`) answers ahead of time.

import { cpSync, existsSync, readFileSync, readdirSync, renameSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { createInterface } from 'node:readline/promises';
import { fileURLToPath } from 'node:url';

const ROUTERS = /** @type {const} */ ({
	'built-in': 'Sepal\'s own: small, nothing of a browser needed',
	sveltekit: 'SvelteKit\'s, as a single-page app: its full client API, more to load'
});

/** @param {string} message @returns {never} */
function fail(message) {
	console.error(message);
	process.exit(1);
}

const USAGE = 'usage: create-sepal <directory> [--router built-in|sveltekit]';

/** @type {string | undefined} */
let directory;
/** @type {string | undefined} */
let router;
const args = process.argv.slice(2);
for (let i = 0; i < args.length; i++) {
	const arg = args[i];
	if (arg === '--sveltekit') router = 'sveltekit';
	else if (arg === '--router') router = args[++i];
	else if (arg.startsWith('--router=')) router = arg.slice('--router='.length);
	else if (arg.startsWith('-')) fail(`unknown option ${arg}\n${USAGE}`);
	else if (directory === undefined) directory = arg;
	else fail(USAGE);
}
if (!directory) fail(USAGE);
if (router !== undefined && !(router in ROUTERS)) fail(`unknown router ${router}: choose built-in or sveltekit`);

const target = resolve(directory);
if (existsSync(target) && readdirSync(target).length > 0) {
	fail(`${directory} exists and is not empty`);
}

// Asked only of a person: a script gets the default unless it says otherwise.
if (router === undefined && process.stdin.isTTY && process.stdout.isTTY) {
	const names = Object.keys(ROUTERS);
	console.log('Which router?\n');
	names.forEach((key, index) => console.log(`  ${index + 1}) ${key.padEnd(10)} ${ROUTERS[/** @type {keyof typeof ROUTERS} */ (key)]}`));
	const prompt = createInterface({ input: process.stdin, output: process.stdout });
	// Ctrl+D or Ctrl+C at the question: leave quietly, having created nothing.
	const answer = (await prompt.question('\n1 or 2 [1]: ').catch(() => fail('\nCancelled.'))).trim().toLowerCase();
	prompt.close();
	router = names[Number(answer) - 1] ?? names.find((key) => key === answer) ?? (answer === '' ? 'built-in' : undefined);
	if (router === undefined) fail(`"${answer}" is not one of the choices`);
}
router ??= 'built-in';

const name = basename(target);
const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'app';
const package_root = fileURLToPath(new URL('.', import.meta.url));
const own = JSON.parse(readFileSync(join(package_root, 'package.json'), 'utf8'));

cpSync(join(package_root, 'template'), target, { recursive: true });
if (router === 'sveltekit') {
	// The same starter, with the files that differ laid over it. The pages
	// themselves do not: `$app/state` and `$app/navigation` are SvelteKit's
	// names, and the built-in router answers to them too.
	cpSync(join(package_root, 'template-sveltekit'), target, { recursive: true });
}
// npm leaves `.gitignore` out of published packages, so it ships under another name.
renameSync(join(target, 'gitignore'), join(target, '.gitignore'));

const pkg_path = join(target, 'package.json');
const pkg = JSON.parse(readFileSync(pkg_path, 'utf8'));
pkg.name = slug;
// The sepal package is released together with this one, under one version.
for (const group of [pkg.dependencies, pkg.devDependencies]) {
	for (const dependency of Object.keys(group)) {
		if (dependency === 'sepal') group[dependency] = `^${own.version}`;
	}
}
writeFileSync(pkg_path, JSON.stringify(pkg, null, '\t') + '\n');

const config_path = join(target, 'sepal.config.ts');
const config = readFileSync(config_path, 'utf8')
	.replace("'__NAME__'", JSON.stringify(name))
	.replace("'__ID__'", JSON.stringify(`com.example.${slug}`));
writeFileSync(config_path, config);

console.log(`Created ${directory} with the ${router} router.\n\n  cd ${directory}\n  pnpm install\n  pnpm dev\n`);
