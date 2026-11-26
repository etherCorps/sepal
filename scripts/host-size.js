// The host's size against its budget: prints it, adds it to the job summary
// on GitHub Actions, and fails above 30,000,000 bytes. Every platform's
// binary has the same budget; release.yml checks the one it ships, and
// host-size.yml the one a pull request that changes dependencies would ship.
//
//   node scripts/host-size.js <binary> [label]

import { appendFileSync, statSync } from 'node:fs';

const BUDGET = 30_000_000;

const [file, label = file] = process.argv.slice(2);
if (!file) throw new Error('usage: node scripts/host-size.js <binary> [label]');
const bytes = statSync(file).size;
const fits = bytes <= BUDGET;
const line = `${label}: ${bytes.toLocaleString('en')} bytes, ${fits ? `${(BUDGET - bytes).toLocaleString('en')} under` : `${(bytes - BUDGET).toLocaleString('en')} over`} the ${BUDGET.toLocaleString('en')}-byte budget`;
console.log(line);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${fits ? '' : ':x: '}${line}\n`);
if (!fits) {
	console.error(`::error::${line}`);
	process.exitCode = 1;
}
