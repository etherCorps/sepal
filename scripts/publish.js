// Publish the packed tarballs in a directory to npm, in the order installs
// need them, and set the dist-tags. release.yml runs it when a commit on main
// carries a version that is not on npm yet (with npm trusted publishing, no
// token); by hand it works the same way, with your own login:
//
//   node scripts/publish.js packs            publish
//   node scripts/publish.js packs --dry-run  print what it would run
//
// Order: the four platform packages, then @ethercorps/calyx, sepal,
// create-sepal. Before each stage, the previous stage's tarballs have to
// download from the registry: on 24 Nov calyx's tarball answered 404 for about
// five minutes after its metadata was up, and anything installed in that
// window fails.
//
// A package already at this version is not published again, and its
// dist-tags are set all the same, so a run that failed halfway can be repeated.
//
// Dist-tags: a stable release is `latest`. A prerelease goes out under its
// own tag (`0.1.0-alpha.1` as `alpha`), and, until sepal has a stable release,
// moves `latest` too, on every package, so that `pnpm create sepal` and
// `pnpm add sepal` get it. Once a stable release exists, prereleases leave
// `latest` alone.

import { execFileSync, spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

/** The stage a package is published in. @param {string} name */
export function stage(name) {
	if (name.startsWith('@ethercorps/calyx-')) return 0;
	const order = ['@ethercorps/calyx', 'sepal', 'create-sepal'];
	if (!order.includes(name)) throw new Error(`${name} is not a Sepal package`);
	return order.indexOf(name) + 1;
}

/**
 * The dist-tags a version gets. The first is the one it is published under.
 * @param {string} version
 * @param {boolean} stable_exists whether sepal has a version without a prerelease part
 */
export function dist_tags(version, stable_exists) {
	const prerelease = /^\d+\.\d+\.\d+-([a-z]+)/.exec(version);
	if (!prerelease) return ['latest'];
	return stable_exists ? [prerelease[1]] : [prerelease[1], 'latest'];
}

/** `npm view`, or `null` when the registry has no such package or version. @param {string[]} args */
function view(args) {
	try {
		const out = execFileSync('npm', ['view', ...args, '--json'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] });
		return out.trim() ? JSON.parse(out) : null;
	} catch {
		return null;
	}
}

/** Where npm serves a version's tarball. @param {string} name @param {string} version */
export function tarball_url(name, version) {
	return `https://registry.npmjs.org/${name}/-/${name.split('/').pop()}-${version}.tgz`;
}

/**
 * The tarball's HTTP status, from the registry itself, not from what a CDN edge
 * remembers. Asked for directly, not through the package's listing: on 24 Nov
 * the edge near a runner served @ethercorps/calyx-darwin-x64's listing from
 * before its publish for over 15 minutes. The query keeps the probe out of the
 * CDN's cache both ways: it reads nothing stale, and it leaves nothing behind (a
 * request for the plain URL before the tarball exists caches a 404 there for
 * five minutes, for everyone).
 * @param {string} name @param {string} version
 */
async function tarball_status(name, version) {
	const response = await fetch(`${tarball_url(name, version)}?probe=${Date.now()}`, { method: 'HEAD' }).catch(() => null);
	return response?.status ?? null;
}

/** Wait until each of these tarballs downloads. @param {{ name: string, version: string }[]} packages */
async function downloadable(packages) {
	const deadline = Date.now() + 20 * 60 * 1000;
	for (const { name, version } of packages) {
		while (true) {
			const status = await tarball_status(name, version);
			if (status === 200) break;
			if (Date.now() > deadline) throw new Error(`${name}@${version} still does not download (${tarball_url(name, version)}: ${status})`);
			console.log(`  waiting for ${name}@${version} to download (${status ?? 'no answer'})`);
			await new Promise((done) => setTimeout(done, 15_000));
		}
		console.log(`  ${name}@${version} downloads`);
	}
}

async function main() {
	const [dir, flag] = process.argv.slice(2);
	if (!dir || (flag !== undefined && flag !== '--dry-run')) throw new Error('usage: node scripts/publish.js <packs dir> [--dry-run]');
	const dry = flag === '--dry-run';

	const packages = readdirSync(dir)
		.filter((file) => file.endsWith('.tgz'))
		.map((file) => {
			const tarball = resolve(dir, file);
			const manifest = JSON.parse(execFileSync('tar', ['-xzOf', tarball, 'package/package.json'], { encoding: 'utf8' }));
			return { name: /** @type {string} */ (manifest.name), version: /** @type {string} */ (manifest.version), tarball };
		});
	const versions = new Set(packages.map((pkg) => pkg.version));
	if (versions.size !== 1) throw new Error(`the tarballs are at different versions: ${[...versions].join(', ')}`);
	const [version] = versions;
	if (packages.length !== 7) throw new Error(`expected 7 tarballs (4 platform packages, calyx, sepal, create-sepal), found ${packages.length}`);

	const sepal_versions = view(['sepal', 'versions']) ?? [];
	const stable_exists = [sepal_versions].flat().some((/** @type {string} */ v) => !v.includes('-'));
	const [tag, ...more] = dist_tags(version, stable_exists);
	console.log(`${version}: published as ${tag}${more.length ? `, also ${more.join(', ')}` : ''}${dry ? ' (dry run)' : ''}`);

	/** @param {string[]} args */
	const npm = (args) => {
		console.log(`  npm ${args.join(' ')}`);
		if (!dry) execFileSync('npm', args, { stdio: 'inherit' });
	};

	/**
	 * `npm publish`, where a version npm already has counts as published: what
	 * the registry says about a package can lag behind its tarballs.
	 * @param {string} name @param {string} tarball
	 */
	const publish = (name, tarball) => {
		const args = ['publish', tarball, '--access', 'public', '--tag', tag];
		console.log(`  npm ${args.join(' ')}`);
		if (dry) return;
		const run = spawnSync('npm', args, { stdio: ['inherit', 'inherit', 'pipe'], encoding: 'utf8' });
		process.stderr.write(run.stderr ?? '');
		if (run.status === 0) return;
		if (/previously published versions|EPUBLISHCONFLICT/.test(run.stderr ?? '')) console.log(`  ${name}@${version} was published already`);
		else throw new Error(`npm publish failed for ${name}@${version}`);
	};

	for (let n = 0; n <= 3; n++) {
		const these = packages.filter((pkg) => stage(pkg.name) === n);
		if (n > 0 && !dry) await downloadable(packages.filter((pkg) => stage(pkg.name) === n - 1));
		for (const { name, tarball } of these) {
			if ((await tarball_status(name, version)) === 200) console.log(`  ${name}@${version} is already published`);
			else publish(name, tarball);
			for (const extra of more) npm(['dist-tag', 'add', `${name}@${version}`, extra]);
		}
	}
	if (!dry) await downloadable(packages.filter((pkg) => stage(pkg.name) === 3));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
