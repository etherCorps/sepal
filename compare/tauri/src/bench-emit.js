// The Tauri way out for bench marks (compare/shared/PROTOCOL.md): each line goes
// to the `mark` command, which writes it to stderr. After every mark but a
// frame mark, two animation frames later, `frame:<name>`: WebKit's nearest
// signal for "the change is on screen".
import { invoke } from '@tauri-apps/api/core';

globalThis.__bench_emit = (line) => {
	invoke('mark', { line });
	// Names may hold spaces (`search:deploy cac`); the time is the last field.
	const name = /^MARK (.+) \S+$/.exec(line)?.[1];
	// `read` and `parsed` (fixture read, JSON.parse done) are start-up steps with no frame, as in gpuix.
	if (name === undefined || name.startsWith('frame:') || name === 'read' || name === 'parsed') return;
	requestAnimationFrame(() =>
		requestAnimationFrame(() => {
			invoke('mark', { line: `MARK frame:${name} ${(performance.timeOrigin + performance.now()).toFixed(1)}` });
		})
	);
};

// The Sepal host prints uncaught errors to stderr; so does this.
const error = (what) => invoke('mark', { line: `ERROR ${what}` });
addEventListener('error', (event) => error(event.error?.stack ?? event.message));
addEventListener('unhandledrejection', (event) => error(event.reason?.stack ?? event.reason));
