// The issue fixtures for compare/ (PROTOCOL.md): seed(n) from the Tracker, as
// `{ n, issues }`. Run from the repository root:
//   node --experimental-strip-types compare/shared/make-fixtures.ts
import { writeFileSync } from 'node:fs';
import { seed } from '../../examples/tracker/src/lib/seed.ts';

for (const n of [1000, 10000]) {
	const issues = seed(n);
	if (issues.length !== n) throw new Error(`seed(${n}) gave ${issues.length}`);
	writeFileSync(new URL(`fixtures/issues-${n}.json`, import.meta.url), JSON.stringify({ n, issues }));
}
