// The seed is deterministic and has the counts docs/tracker-plan.md promises.
// Runs in Node with its type stripping: `pnpm --filter tracker test`.

import assert from 'node:assert/strict';
import { DAY, EPOCH, LABELS, PEOPLE, PRIORITIES, PROJECTS, STATUSES, WEEK, details, seed } from '../src/lib/seed.ts';
import { count, long_date, short_date } from '../src/lib/format.ts';

assert.equal(PEOPLE.length, 25);
assert.equal(PROJECTS.length, 5);
assert.equal(LABELS.length, 12);
assert.equal(STATUSES.length, 6);
assert.equal(PRIORITIES.length, 5);

const a = seed(5000);
const b = seed(5000);
assert.equal(a.length, 5000);
assert.deepEqual(a, b, 'the same N gives the same issues');
assert.equal(seed().length, 5000, 'N defaults to 5000');
assert.equal(seed(1000).length, 1000);

assert.equal(new Set(a.map((issue) => issue.key)).size, a.length, 'keys are unique');
for (const issue of a) {
	assert.ok(issue.createdAt <= EPOCH && issue.createdAt > EPOCH - 26 * WEEK, `${issue.key} falls in the 26 weeks`);
	assert.ok(issue.updatedAt >= issue.createdAt && issue.updatedAt <= EPOCH);
	assert.equal(issue.closedAt === null, issue.status !== 'done' && issue.status !== 'canceled');
	assert.equal(issue.key, `${issue.project}-${issue.number}`);
}
for (let i = 1; i < a.length; i++) assert.ok(a[i - 1].createdAt >= a[i].createdAt, 'newest first');
for (const status of STATUSES) assert.ok(a.some((issue) => issue.status === status.id), `some issues are ${status.id}`);
assert.ok(a.some((issue) => issue.title.includes('deploy')), 'the search benchmark types "deploy"');

// Pin a few values, so a change to the generator is a decision, not an accident.
assert.deepEqual(
	a.slice(0, 3).map((issue) => `${issue.key} ${issue.status} ${issue.priority} ${issue.title}`),
	['ENG-1092 todo low Deprecate session tokens under load', 'OPS-331 review low Split release notes', 'API-175 backlog high Migrate error boundary after timeout']
);

const first = details(a[0]);
assert.equal(details(a[0]), first, 'details are cached');
assert.ok(first.activity.length >= 5 && first.activity.length <= 40);
assert.ok(first.description.length > 0);

assert.equal(count(5000), '5,000');
assert.equal(count(214), '214');
assert.equal(count(1234567), '1,234,567');
assert.equal(short_date(EPOCH - 2 * DAY), 'Nov 21');
assert.equal(long_date(Date.UTC(2026, 10, 21, 14, 5)), '21 Nov 2026, 14:05');

console.log('seed: ok');
