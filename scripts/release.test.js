// scripts/release.js: the next version, the changelog, and a release
// committed in a throwaway repository laid out like this one.

import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { day, file_unreleased, next, section } from './release.js';

// Versions.
assert.equal(next('0.1.0-alpha.1', 'prerelease'), '0.1.0-alpha.2');
assert.equal(next('0.1.0-alpha.9', 'prerelease'), '0.1.0-alpha.10');
assert.equal(next('0.1.0-beta', 'prerelease'), '0.1.0-beta.0');
assert.equal(next('0.1.0', 'prerelease'), '0.1.1-alpha.0');
assert.equal(next('0.1.0-alpha.2', 'patch'), '0.1.0');
assert.equal(next('0.1.0', 'patch'), '0.1.1');
assert.equal(next('0.1.0-alpha.2', 'minor'), '0.1.0');
assert.equal(next('0.1.3', 'minor'), '0.2.0');
assert.equal(next('0.1.0-alpha.1', '0.2.0-beta.0'), '0.2.0-beta.0');
assert.throws(() => next('0.1.0', '0.1.0'), /already/);
assert.throws(() => next('0.1.0', 'major'), /prerelease, patch, minor/);

// The changelog.
const changelog = `# Changelog

## Unreleased

- Something new.

## 0.1.0-alpha.1 (24 Nov 2026)

- The split.

## 0.1.0-alpha.0

- First.
`;
assert.equal(section(changelog, '0.1.0-alpha.1'), '- The split.');
assert.equal(section(changelog, '0.1.0-alpha.0'), '- First.');
assert.equal(section(changelog, '0.1.0-alpha'), null, 'a version is not a prefix of another');
const filed = file_unreleased(changelog, '0.1.0-alpha.2', '25 Nov 2026');
assert.ok(filed.includes('## 0.1.0-alpha.2 (25 Nov 2026)\n\n- Something new.'));
assert.ok(!filed.includes('Unreleased'));
assert.throws(() => file_unreleased('# Changelog\n\n## 0.1.0\n\n- x\n', '0.1.1', 'today'), /no "## Unreleased"/);
assert.throws(() => file_unreleased('# Changelog\n\n## Unreleased\n\n## 0.1.0\n', '0.1.1', 'today'), /empty/);
assert.equal(day(new Date(2026, 10, 5)), '5 Nov 2026');

// End to end: a repository with two packages, the crate, its lock and a changelog.
const script = fileURLToPath(new URL('./release.js', import.meta.url));
const root = mkdtempSync(join(tmpdir(), 'sepal-release-'));
try {
	const write = (/** @type {string} */ file, /** @type {string} */ text) => {
		mkdirSync(join(root, file, '..'), { recursive: true });
		writeFileSync(join(root, file), text);
	};
	for (const name of ['sepal', 'host']) write(`packages/${name}/package.json`, `{\n\t"name": "${name}",\n\t"version": "0.1.0-alpha.1",\n\t"dependencies": { "x": "1.0.0" }\n}\n`);
	write('crates/host/Cargo.toml', '[package]\nname = "sepal"\nversion = "0.1.0-alpha.1"\nedition = "2024"\n\n[dependencies]\nserde = { version = "1" }\n');
	write('crates/host/Cargo.lock', '[[package]]\nname = "serde"\nversion = "1.0.0"\n\n[[package]]\nname = "sepal"\nversion = "0.1.0-alpha.1"\n');
	write('CHANGELOG.md', changelog);
	const git = (/** @type {string[]} */ ...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
	git('init', '--quiet');
	git('config', 'user.name', 'Test');
	git('config', 'user.email', 'test@example.com');
	git('config', 'commit.gpgsign', 'false');
	git('add', '.');
	git('commit', '--quiet', '-m', 'Start');

	const run = (/** @type {string[]} */ ...args) => spawnSync(process.execPath, [script, ...args], { cwd: root, encoding: 'utf8' });
	const released = run('prerelease');
	assert.equal(released.status, 0, released.stderr);
	assert.equal(git('log', '-1', '--format=%s'), 'Release 0.1.0-alpha.2');
	assert.equal(git('status', '--porcelain'), '', 'everything it changed is in the commit');
	for (const name of ['sepal', 'host']) {
		const pkg = JSON.parse(readFileSync(join(root, `packages/${name}/package.json`), 'utf8'));
		assert.equal(pkg.version, '0.1.0-alpha.2');
		assert.equal(pkg.dependencies.x, '1.0.0', 'only the version changes');
	}
	assert.match(readFileSync(join(root, 'crates/host/Cargo.toml'), 'utf8'), /^version = "0\.1\.0-alpha\.2"$/m);
	assert.match(readFileSync(join(root, 'crates/host/Cargo.toml'), 'utf8'), /serde = \{ version = "1" \}/);
	const lock = readFileSync(join(root, 'crates/host/Cargo.lock'), 'utf8');
	assert.ok(lock.includes('name = "sepal"\nversion = "0.1.0-alpha.2"') && lock.includes('name = "serde"\nversion = "1.0.0"'));
	assert.ok(readFileSync(join(root, 'CHANGELOG.md'), 'utf8').includes(`## 0.1.0-alpha.2 (${day(new Date())})`));

	// The release's notes, as the workflow reads them.
	const notes = run('notes', '0.1.0-alpha.2');
	assert.equal(notes.stdout.trim(), '- Something new.');
	assert.equal(run('notes', '9.9.9').status, 1);

	// Nothing to say about the next one yet: refused, and nothing is touched.
	const empty = run('prerelease');
	assert.equal(empty.status, 1);
	assert.match(empty.stderr, /no "## Unreleased"/);
	assert.equal(git('log', '-1', '--format=%s'), 'Release 0.1.0-alpha.2');
	assert.equal(git('status', '--porcelain'), '');

	console.log('release: ok');
} finally {
	rmSync(root, { recursive: true, force: true });
}
