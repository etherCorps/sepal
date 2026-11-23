// One-off: the Tracker's font files (woff2/woff from examples/tracker/node_modules/@fontsource*)
// decompressed to TrueType, because GPUI's register_font takes TTF/OTF only (woff2 registers
// without an error and is then never used). Same glyph data; run with `bun scripts-fonts.ts`.
import { readFileSync, writeFileSync } from 'node:fs';
// @ts-ignore no types
import woff2 from 'wawoff2';

const src = '../../examples/tracker/node_modules';
const files: [string, string][] = [
	[`${src}/@fontsource-variable/inter/files/inter-latin-wght-normal.woff2`, 'fonts/inter-latin-wght-normal.ttf'],
	[`${src}/@fontsource/space-grotesk/files/space-grotesk-latin-500-normal.woff2`, 'fonts/space-grotesk-latin-500-normal.ttf'],
	[`${src}/@fontsource/space-grotesk/files/space-grotesk-latin-600-normal.woff2`, 'fonts/space-grotesk-latin-600-normal.ttf'],
	[`${src}/@fontsource/jetbrains-mono/files/jetbrains-mono-latin-400-normal.woff2`, 'fonts/jetbrains-mono-latin-400-normal.ttf'],
	[`${src}/@fontsource/jetbrains-mono/files/jetbrains-mono-latin-500-normal.woff2`, 'fonts/jetbrains-mono-latin-500-normal.ttf']
];
for (const [from, to] of files) {
	const ttf = await woff2.decompress(readFileSync(from));
	writeFileSync(to, ttf);
	console.log(to, ttf.length);
}
