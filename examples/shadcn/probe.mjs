// Does each shadcn-svelte component work in svelte-blitz? For every file in
// src/probes: compile it with the preset, run the bundle in the host, and
// report where it fails, if it does.
//
//   node probe.mjs            (needs a host: SVELTE_BLITZ_HOST, or `pnpm run build:host`)

import { spawnSync } from 'node:child_process';
import { mkdirSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import tailwindcss from '@tailwindcss/vite';
import { build } from 'vite';
import { find_host } from '@ethercorps/svelte-blitz-host';
import { svelteBlitz } from '@ethercorps/svelte-blitz-vite';

const root = fileURLToPath(new URL('.', import.meta.url));
const host = find_host();
if (!host) throw new Error('no host binary');
const out = join(root, 'dist-probe');
rmSync(out, { recursive: true, force: true });
mkdirSync(join(root, 'src/probes/.entries'), { recursive: true });

const names = readdirSync(join(root, 'src/probes')).filter((f) => f.endsWith('.svelte')).map((f) => f.replace('.svelte', '')).sort();
const only = process.argv.slice(2);
const results = [];

for (const name of names.filter((n) => only.length === 0 || only.includes(n))) {
	const entry = `src/probes/.entries/${name}.ts`;
	writeFileSync(join(root, entry), `import { mount } from '@ethercorps/svelte-blitz';\nimport '../../app.css';\nimport Probe from '../${name}.svelte';\nmount(Probe);\n`);
	const dir = join(out, name);
	let stage = 'compile';
	let detail = '';
	try {
		await build({
			root, configFile: false, logLevel: 'silent',
			plugins: [tailwindcss(), svelteBlitz({ router: false, entry, app: {} })],
			build: { outDir: dir, emptyOutDir: true }
		});
		stage = 'run';
		const ran = spawnSync(host, ['--screenshot', join(dir, 'shot.png'), dir, '640x360'], { encoding: 'utf8' });
		// The host logs what the app logs: console.warn and console.error, and uncaught errors.
		detail = (ran.stderr + ran.stdout).trim();
		stage = ran.status !== 0 ? 'run' : /\[js error\]/.test(detail) ? 'errors' : /\[js warn\]/.test(detail) ? 'warns' : 'ok';
	} catch (error) {
		detail = String(error.stderr || error.message || error).trim();
	}
	results.push({ name, stage, detail });
	const first = detail.split('\n').find((line) => line.trim() && !line.includes('npm warn')) ?? '';
	if (process.env.VERBOSE) console.log(detail.slice(0, 1800));
	console.log(`${stage === 'ok' ? 'OK  ' : stage === 'warns' ? 'WARN' : 'FAIL'} ${name.padEnd(10)} ${stage.padEnd(8)} ${first.slice(0, 150)}`);
}
rmSync(join(root, 'src/probes/.entries'), { recursive: true, force: true });
mkdirSync(out, { recursive: true });
writeFileSync(join(out, 'results.json'), JSON.stringify(results, null, '\t'));
