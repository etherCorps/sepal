// The packages are released together under one version, and the starter
// template is written against that version. Fail if they have drifted.

import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const packages = fileURLToPath(new URL('../packages', import.meta.url));
const versions = readdirSync(packages).map((directory) => {
	const { name, version } = JSON.parse(readFileSync(join(packages, directory, 'package.json'), 'utf8'));
	return { name, version };
});

if (new Set(versions.map((entry) => entry.version)).size > 1) {
	console.error('Package versions differ; they must all be the same:');
	for (const { name, version } of versions) console.error(`  ${version}  ${name}`);
	process.exit(1);
}
console.log(`versions: ${versions.length} packages at ${versions[0].version}`);
