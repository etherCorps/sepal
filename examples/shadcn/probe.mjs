// Does each shadcn-svelte component work in Sepal? For every file in
// src/probes: compile it with the preset, run the bundle in the host, and
// report where it fails, if it does.
//
//   node probe.mjs            (needs a host: SEPAL_HOST, or `pnpm run build:host`)

import { spawnSync } from 'node:child_process';
import { mkdirSync, readdirSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import tailwindcss from '@tailwindcss/vite';
import { build } from 'vite';
import { find_host } from '@ethercorps/calyx';
import { sepal } from 'sepal/vite';

const root = fileURLToPath(new URL('.', import.meta.url));
const host = find_host().path;
const out = join(root, 'dist-probe');
rmSync(out, { recursive: true, force: true });
mkdirSync(join(root, 'src/probes/.entries'), { recursive: true });

const names = readdirSync(join(root, 'src/probes')).filter((f) => f.endsWith('.svelte')).map((f) => f.replace('.svelte', '')).sort();
const only = process.argv.slice(2);
const results = [];

for (const name of names.filter((n) => only.length === 0 || only.includes(n))) {
	// A page per probe, which the build takes in place of index.html.
	const entry = `src/probes/.entries/${name}`;
	writeFileSync(join(root, `${entry}.ts`), `import { mount } from 'sepal';\nimport '../../app.css';\nimport Probe from '../${name}.svelte';\nmount(Probe);\n`);
	writeFileSync(join(root, `${entry}.html`), `<!doctype html><html><head><title>${name}</title></head><body><script type="module" src="./${name}.ts"></script></body></html>\n`);
	const dir = join(out, name);
	let stage = 'compile';
	let detail = '';
	try {
		await build({
			root, configFile: false, logLevel: 'silent',
			plugins: [tailwindcss(), sepal({ app: {}, dom: true })],
			// Not minified: a stack trace from a probe should be readable.
			build: { outDir: dir, emptyOutDir: true, minify: false, rolldownOptions: { input: join(root, `${entry}.html`) } }
		});
		// The page lands where it was in the project; the app's is at the top.
		renameSync(join(dir, `${entry}.html`), join(dir, 'index.html'));
		stage = 'run';
		const ran = spawnSync(host, ['--screenshot', join(dir, 'shot.png'), dir, '640x360'], { encoding: 'utf8', timeout: 15000 });
		// The host logs what the app logs: console.warn and console.error, and uncaught errors.
		detail = (ran.stderr + ran.stdout).trim();
		if (ran.error) detail = `the host did not finish: ${ran.error.message}`;
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
