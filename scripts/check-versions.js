// The packages are released together under one version, and the starter
// template is written against that version. The host crate carries it too,
// in Cargo.toml and Cargo.lock, so that `calyx --version` names the release
// a binary shipped in. Fail if any of them have drifted.

import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const packages = fileURLToPath(new URL('../packages', import.meta.url));
const versions = readdirSync(packages).map((directory) => {
	const { name, version } = JSON.parse(readFileSync(join(packages, directory, 'package.json'), 'utf8'));
	return { name, version };
});
const host = fileURLToPath(new URL('../crates/host/', import.meta.url));
const crate = /^\[package\]\nname = "sepal"\nversion = "([^"]+)"/m;
const locked = /^\[\[package\]\]\nname = "sepal"\nversion = "([^"]+)"/m;
versions.push(
	{ name: 'crates/host/Cargo.toml', version: crate.exec(readFileSync(join(host, 'Cargo.toml'), 'utf8'))?.[1] ?? '(no version found)' },
	{ name: 'crates/host/Cargo.lock', version: locked.exec(readFileSync(join(host, 'Cargo.lock'), 'utf8'))?.[1] ?? '(no version found)' }
);

if (new Set(versions.map((entry) => entry.version)).size > 1) {
	console.error('Package versions differ; they must all be the same:');
	for (const { name, version } of versions) console.error(`  ${version}  ${name}`);
	process.exit(1);
}
console.log(`versions: ${versions.length - 2} packages and the host crate at ${versions[0].version}`);
