// Ask for a release:
//
//   pnpm release prerelease        0.1.0-alpha.1 -> 0.1.0-alpha.2
//   pnpm release patch | minor     0.1.0-alpha.2 -> 0.1.0, 0.1.0 -> 0.1.1 | 0.2.0
//   pnpm release 0.2.0-beta.0      exactly that
//
// Sets every package in packages/, the host crate and its Cargo.lock entry to
// the new version, moves CHANGELOG.md's "Unreleased" section under it with
// today's date, and commits "Release <version>". It does not push: that
// commit reaching main is what publishes it (.github/workflows/release.yml).
//
//   node scripts/release.js notes <version>
//
// prints that version's CHANGELOG section, the GitHub Release's notes, and
// fails when there is none.
//
// Run from the repository's root, as `pnpm release` does.

import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const SEMVER = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/**
 * The version after `current`.
 * @param {string} current
 * @param {string} how `prerelease`, `patch`, `minor`, or a version to use as it is
 */
export function next(current, how) {
	const parts = SEMVER.exec(current);
	if (!parts) throw new Error(`${current} is not a version`);
	if (SEMVER.test(how)) {
		if (how === current) throw new Error(`${current} is the version already`);
		return how;
	}
	const [major, minor, patch] = parts.slice(1, 4).map(Number);
	const pre = parts[4];
	if (how === 'prerelease') {
		if (!pre) return `${major}.${minor}.${patch + 1}-alpha.0`;
		const ids = pre.split('.');
		const last = ids[ids.length - 1];
		if (/^\d+$/.test(last)) ids[ids.length - 1] = String(Number(last) + 1);
		else ids.push('0');
		return `${major}.${minor}.${patch}-${ids.join('.')}`;
	}
	// A prerelease of x.y.z is released as x.y.z, as npm's semver does.
	if (how === 'patch') return pre ? `${major}.${minor}.${patch}` : `${major}.${minor}.${patch + 1}`;
	if (how === 'minor') return pre && patch === 0 ? `${major}.${minor}.0` : `${major}.${minor + 1}.0`;
	throw new Error(`release takes prerelease, patch, minor or a version, not ${JSON.stringify(how)}`);
}

/**
 * A version's section of the changelog, without its heading; `null` when there
 * is none. The heading is `## <version>`, maybe followed by more (a date).
 * @param {string} changelog
 * @param {string} version
 */
export function section(changelog, version) {
	const lines = changelog.split('\n');
	const start = lines.findIndex((line) => line === `## ${version}` || line.startsWith(`## ${version} `));
	if (start < 0) return null;
	const after = lines.slice(start + 1);
	const end = after.findIndex((line) => line.startsWith('## '));
	return (end < 0 ? after : after.slice(0, end)).join('\n').trim();
}

/** `24 Nov 2026` @param {Date} date */
export const day = (date) => `${date.getDate()} ${MONTHS[date.getMonth()]} ${date.getFullYear()}`;

/**
 * The changelog with its "Unreleased" section filed under the version.
 * @param {string} changelog @param {string} version @param {string} date
 */
export function file_unreleased(changelog, version, date) {
	const notes = section(changelog, 'Unreleased');
	if (notes === null) throw new Error('CHANGELOG.md has no "## Unreleased" section: say what this release changes there first');
	if (notes === '') throw new Error('the "## Unreleased" section of CHANGELOG.md is empty: say what this release changes there first');
	if (section(changelog, version) !== null) throw new Error(`CHANGELOG.md already has a section for ${version}`);
	return changelog.replace(/^## Unreleased.*$/m, `## ${version} (${date})`);
}

/** @param {string} text @param {RegExp} pattern a version in its second group @param {string} version @param {string} what */
function set(text, pattern, version, what) {
	if (!pattern.test(text)) throw new Error(`no version found in ${what}`);
	return text.replace(pattern, (_, before, _old, after) => before + version + after);
}

/** The host crate's version, in Cargo.toml or Cargo.lock. */
export const CRATE = /^(\[package\]\nname = "sepal"\nversion = ")([^"]+)(")/m;
export const LOCKED = /^(\[\[package\]\]\nname = "sepal"\nversion = ")([^"]+)(")/m;
const PACKAGE = /^(\s*"version": ")([^"]+)(")/m;

/** @param {string} root @param {string[]} args */
export function main(root, args) {
	const [how, version] = args;
	const read = (/** @type {string} */ file) => readFileSync(join(root, file), 'utf8');

	if (how === 'notes') {
		const notes = version ? section(read('CHANGELOG.md'), version) : null;
		if (!notes) throw new Error(`CHANGELOG.md has no section for ${version}`);
		return notes;
	}
	if (!how) throw new Error('usage: pnpm release <prerelease|patch|minor|version>');

	const current = JSON.parse(read('packages/sepal/package.json')).version;
	const target = next(current, how);
	/** @type {Map<string, string>} */
	const files = new Map();
	for (const directory of readdirSync(join(root, 'packages'))) {
		const file = `packages/${directory}/package.json`;
		files.set(file, set(read(file), PACKAGE, target, file));
	}
	files.set('crates/host/Cargo.toml', set(read('crates/host/Cargo.toml'), CRATE, target, 'crates/host/Cargo.toml'));
	files.set('crates/host/Cargo.lock', set(read('crates/host/Cargo.lock'), LOCKED, target, 'crates/host/Cargo.lock'));
	files.set('CHANGELOG.md', file_unreleased(read('CHANGELOG.md'), target, day(new Date())));

	// Everything is worked out before anything is written.
	for (const [file, text] of files) writeFileSync(join(root, file), text);
	const git = (/** @type {string[]} */ ...rest) => execFileSync('git', rest, { cwd: root, stdio: 'inherit' });
	git('add', '--', ...files.keys());
	git('commit', '--quiet', '-m', `Release ${target}`, '--', ...files.keys());
	return `Committed "Release ${target}" (from ${current}). Push it to main to publish it.`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
	try {
		console.log(main(process.cwd(), process.argv.slice(2)));
	} catch (error) {
		console.error(`release: ${/** @type {Error} */ (error).message}`);
		process.exit(1);
	}
}
