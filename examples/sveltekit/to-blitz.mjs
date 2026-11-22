// Turn SvelteKit's build (an HTML shell that imports one ES module) into what
// the svelte-blitz host loads: dist/app.js, a plain script, and dist/app.css.
//
//   vite build && node to-blitz.mjs

import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';

const html = readFileSync('build/index.html', 'utf8');
// `__sveltekit_abc123 = { base: "", version: "…" };`, which SvelteKit's client reads.
const payload = /__sveltekit_\w+ = \{[^}]*\};/.exec(html)?.[0];
if (!payload) throw new Error('no SvelteKit payload in build/index.html');

const directory = 'build/_app/immutable';
const name = readdirSync(directory).find((file) => /^bundle\..*\.js$/.test(file));
let code = readFileSync(`${directory}/${name}`, 'utf8');

// The module's one export statement, `export{a as app,b as start}`.
const exported = /export\s*\{([^}]*)\};?\s*$/.exec(code);
if (!exported) throw new Error('the bundle does not end with its exports');
// `local as exported`, or just `name` when they are the same.
const start = exported[1].split(',').map((pair) => pair.trim().split(/\s+as\s+/)).find(([local, as = local]) => as === 'start')?.[0];
if (!start) throw new Error('the bundle does not export `start`');
code = code.slice(0, exported.index);

// A script has no `import.meta`. The module's address only matters for
// resolving paths next to it.
const url = `http://app.localhost/_app/immutable/${name}`;
code = code.replaceAll('import.meta.resolve', '((path) => new URL(path, __module_url).href)').replaceAll('import.meta.url', '__module_url');

const css = readdirSync(directory, { recursive: true }).filter((file) => String(file).endsWith('.css')).map((file) => readFileSync(`${directory}/${file}`, 'utf8'));

mkdirSync('dist', { recursive: true });
writeFileSync(
	'dist/app.js',
	`(() => {
const __module_url = ${JSON.stringify(url)};
${code}
globalThis.${payload}
// Where SvelteKit puts the app: the element its start script sat in.
const element = document.createElement('div');
document.body.appendChild(element);
${start}(element);
})();
`
);
writeFileSync('dist/app.css', css.join('\n'));
console.log(`dist/app.js ${(code.length / 1024).toFixed(0)} kB, dist/app.css ${css.length} file(s)`);
