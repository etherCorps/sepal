// The entry `bun build.ts` compiles, and what `npm start` runs. start.ts first: the bench build's
// `MARK start` comes before any other module is evaluated.
import './src/start.ts';
import { readFileSync } from 'node:fs';
import { render, register_font } from 'gpuix-svelte';
import App from './src/App.svelte';
// The Tracker's font files (examples/tracker's @fontsource packages) as TrueType: scripts/fonts.ts.
import inter from './fonts/inter-latin-wght-normal.ttf' with { type: 'file' };
import grotesk_500 from './fonts/space-grotesk-latin-500-normal.ttf' with { type: 'file' };
import grotesk_600 from './fonts/space-grotesk-latin-600-normal.ttf' with { type: 'file' };
import mono_400 from './fonts/jetbrains-mono-latin-400-normal.ttf' with { type: 'file' };
import mono_500 from './fonts/jetbrains-mono-latin-500-normal.ttf' with { type: 'file' };

render(App, {
	title: 'Tracker',
	width: 1200,
	height: 800,
	app: {
		name: 'Tracker',
		identifier: 'com.compare.tracker.gpuix'
	}
});

// After render(): register_font needs the window, and nothing has been laid out until the frame loop's first tick.
for (const file of [inter, grotesk_500, grotesk_600, mono_400, mono_500]) register_font(readFileSync(file));
