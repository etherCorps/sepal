// `bun build.ts` (npm run build): the Tracker as dist/Tracker.app, with gpuix-svelte's build() and
// its defaults (no --aot / --trim). `--bench` makes the bench build (src/build-flag.ts), as
// VITE_BENCH=1 does for Sepal and Tauri; the flag file is put back afterwards.
import { readFileSync, writeFileSync } from 'node:fs';
import { build } from 'gpuix-svelte/build';

const FLAG = 'src/build-flag.ts';
const original = readFileSync(FLAG, 'utf8');
const bench = process.argv.includes('--bench');
writeFileSync(FLAG, original.replace(/BENCH_BUILD: boolean = (true|false)/, `BENCH_BUILD: boolean = ${bench}`));
try {
	await build({ entry: './standalone.ts' });
} finally {
	writeFileSync(FLAG, original);
}
