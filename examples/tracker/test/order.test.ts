// Splicing edits into a sorted list (order.ts) gives what sorting again gives.
// Runs in Node with its type stripping: `pnpm --filter tracker test`.

import assert from 'node:assert/strict';
import { MAX_STEPS, apply_steps, apply_steps_grouped, comparators, kept, type SortBy, type Step } from '../src/lib/order.ts';
import { PRIORITIES, STATUSES, seed, type Issue } from '../src/lib/seed.ts';

let issues: Issue[] = seed(3000);
const orders = Object.keys(comparators) as SortBy[];
const sorted = Object.fromEntries(orders.map((by) => [by, issues.slice().sort(comparators[by])])) as Record<SortBy, Issue[]>;

// As the store chains them: sorted, then filtered (`kept`), then cut into groups.
const keep = (issue: Issue) => issue.project !== 'OPS' && issue.priority !== 'low';
const status_of = (issue: Issue) => issue.status;
const buckets_of = (list: Issue[]) => {
	const out = new Map<string, Issue[]>();
	for (const issue of list) out.set(issue.status, [...(out.get(issue.status) ?? []), issue]);
	return out;
};
let filtered = sorted.title.filter(keep);
let buckets = buckets_of(filtered);

// Seeded titles repeat, so the title order has ties for the tie-breaker to settle.
assert.ok(new Set(issues.map((i) => i.title)).size < issues.length, 'some titles repeat');

let state = 1;
const random = (n: number) => ((state = (state * 48271) % 2147483647) % n);
let at = Date.UTC(2026, 10, 24);

for (let round = 0; round < 300; round++) {
	const steps: Step[] = [];
	const edits = 1 + random(3);
	for (let e = 0; e < edits; e++) {
		const kind = random(10);
		if (kind < 7) {
			// an edit: status, priority or title, and a new updatedAt (sometimes the same one, a tie)
			const old = issues[random(issues.length)];
			const next: Issue = { ...old, updatedAt: kind === 0 ? old.updatedAt : (at += 1000) };
			if (kind < 3) next.status = STATUSES[random(STATUSES.length)].id;
			else if (kind < 5) next.priority = PRIORITIES[random(PRIORITIES.length)].id;
			else next.title = issues[random(issues.length)].title;
			issues = issues.map((i) => (i === old ? next : i));
			steps.push({ removed: [old], added: [next] });
		} else if (kind < 9) {
			const gone = issues[random(issues.length)];
			issues = issues.filter((i) => i !== gone);
			steps.push({ removed: [gone], added: [] });
		} else {
			const made: Issue = { ...issues[random(issues.length)], key: `NEW-${round}-${e}`, createdAt: at, updatedAt: at };
			issues = [made, ...issues];
			steps.push({ removed: [], added: [made] });
		}
	}
	assert.ok(steps.length <= MAX_STEPS);

	const seen = kept(steps, keep);
	const next_filtered = apply_steps(filtered, seen, comparators.title);
	const next_buckets = apply_steps_grouped(buckets, seen, comparators.title, status_of);
	assert.ok(next_filtered !== null && next_buckets !== null);
	filtered = next_filtered;
	buckets = next_buckets;
	const resorted = issues.slice().sort(comparators.title).filter(keep);
	assert.deepEqual(filtered.map((i) => i.key), resorted.map((i) => i.key), `round ${round}: filtered`);
	const full = buckets_of(resorted);
	for (const status of STATUSES) {
		assert.deepEqual((buckets.get(status.id) ?? []).map((i) => i.key), (full.get(status.id) ?? []).map((i) => i.key), `round ${round}: ${status.id} bucket`);
	}

	for (const by of orders) {
		const spliced = apply_steps(sorted[by], steps, comparators[by]);
		const full = issues.slice().sort(comparators[by]);
		assert.ok(spliced !== null, `round ${round}, ${by}: applied`);
		assert.deepEqual(
			spliced.map((i) => i.key),
			full.map((i) => i.key),
			`round ${round}, ${by}: spliced = sorted`
		);
		sorted[by] = spliced;
	}
}

// An issue that is not where the order says gives up instead of guessing.
const stray = { ...issues[0], key: 'NOT-THERE' };
assert.equal(apply_steps(sorted.updated, [{ removed: [stray], added: [] }], comparators.updated), null);

console.log('order: ok');
