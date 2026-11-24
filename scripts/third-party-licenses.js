// Write the licences of everything compiled into the host binary to
// packages/host/THIRD-PARTY-LICENSES.txt, which ships next to it.
//
// The binary is our code plus several hundred Rust crates (and QuickJS, which
// comes inside one). Their licences ask for their notices to travel with the
// binary; Stylo's (MPL-2.0) also asks that people can find its source, so each
// crate is listed with where it comes from.
//
// The texts are the crates' own licence files, read from Cargo's registry, so
// the copyright lines are theirs. Crates whose files are identical are listed
// together above one copy of the text.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const manifest = join(root, 'crates/host/Cargo.toml');
const out = join(root, 'packages/host/THIRD-PARTY-LICENSES.txt');

// The features the shipped binary is built with (see build-host-package.js).
// All platforms at once: a superset is fine for a list of notices.
const metadata = JSON.parse(
	execFileSync('cargo', ['metadata', '--format-version', '1', '--features', 'screenshot', '--manifest-path', manifest], {
		encoding: 'utf8',
		maxBuffer: 256 * 1024 * 1024
	})
);

/** @type {Map<string, any>} */
const packages = new Map(metadata.packages.map((/** @type {any} */ pkg) => [pkg.id, pkg]));
/** @type {Map<string, any>} */
const nodes = new Map(metadata.resolve.nodes.map((/** @type {any} */ node) => [node.id, node]));

// What ends up in the binary: reachable from the host through ordinary
// dependencies. Build scripts and tests are not linked in.
const linked = new Set();
const queue = [metadata.resolve.root];
while (queue.length > 0) {
	const id = /** @type {string} */ (queue.pop());
	if (linked.has(id)) continue;
	linked.add(id);
	for (const dep of nodes.get(id).deps) {
		if (dep.dep_kinds.some((/** @type {any} */ kind) => kind.kind === null)) queue.push(dep.pkg);
	}
}
linked.delete(metadata.resolve.root);

const notice = /^(licen[sc]e|copying|notice|unlicense)/i;

/** @type {Map<string, { text: string, crates: string[] }>} licence text by its hash */
const texts = new Map();
/** @type {string[]} */
const index = [];
/** @type {string[]} */
const without_text = [];

for (const id of [...linked].sort((a, b) => packages.get(a).name.localeCompare(packages.get(b).name))) {
	const pkg = packages.get(id);
	const label = `${pkg.name} ${pkg.version}`;
	// Our own patched copies are listed too: they are still their authors' work.
	const source = pkg.repository ?? pkg.homepage ?? `https://crates.io/crates/${pkg.name}`;
	index.push(`${label}  (${pkg.license ?? 'see its licence file'})  ${source}`);

	const directory = dirname(pkg.manifest_path);
	// One level down as well: a crate that wraps a C library keeps the
	// library, and its licence, in a subdirectory (QuickJS in rquickjs-sys),
	// and some crates keep theirs in a `LICENSES` directory.
	const files = existsSync(directory)
		? readdirSync(directory, { withFileTypes: true })
				.flatMap((entry) =>
					entry.isDirectory() && entry.name !== 'target'
						? readdirSync(join(directory, entry.name), { withFileTypes: true })
								.filter((inner) => inner.isFile() && (notice.test(inner.name) || notice.test(entry.name)))
								.map((inner) => join(entry.name, inner.name))
						: entry.isFile() && notice.test(entry.name)
							? [entry.name]
							: []
				)
				.sort()
		: [];
	if (files.length === 0) without_text.push(label);
	for (const file of files) {
		// Line endings and trailing space differ between otherwise identical copies.
		const text = readFileSync(join(directory, file), 'utf8').replace(/\r\n/g, '\n').replace(/[ \t]+$/gm, '').trim();
		const key = createHash('sha256').update(text).digest('hex');
		if (!texts.has(key)) texts.set(key, { text, crates: [] });
		/** @type {{ crates: string[] }} */ (texts.get(key)).crates.push(label);
	}
}

const rule = '='.repeat(78);
const sections = [...texts.values()]
	.sort((a, b) => b.crates.length - a.crates.length)
	.map(({ text, crates }) => `${rule}\n${[...new Set(crates)].join(', ')}\n${rule}\n\n${text}\n`);

writeFileSync(
	out,
	`Third-party software in the Sepal host binary

The host is built from the crates listed below. Each is the work of its
authors and is used under the licence named beside it; the source of each is
at the address given. The licence texts follow the list.

${index.join('\n')}
${without_text.length > 0 ? `\nPublished without a licence file of their own. The licence named above applies;\nthe texts of those licences (MIT, Apache-2.0, MPL-2.0) are among those below:\n${without_text.join(', ')}\n` : ''}
${sections.join('\n')}`
);
console.log(`third-party licences: ${linked.size} crates, ${texts.size} distinct texts, ${(readFileSync(out).length / 1024).toFixed(0)} kB`);
