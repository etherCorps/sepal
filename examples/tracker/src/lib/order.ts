// The list's sort orders, and keeping a sorted array sorted through a few
// edits without sorting it again (a teammate's edit, a card move: one issue
// out of 20,000). Plain TypeScript: test/order.test.ts runs it in Node.

import { PRIORITIES, type Issue, type PriorityId } from './seed.ts';

export type SortBy = 'updated' | 'created' | 'priority' | 'title';
export type Compare = (a: Issue, b: Issue) => number;

const priority_rank = Object.fromEntries(PRIORITIES.map((p, i) => [p.id, i])) as Record<PriorityId, number>;

/**
 * Every order ends on newest created, then key, so no two issues tie: a
 * binary search then finds the one place an issue goes, and an incremental
 * update gives exactly what a full sort would.
 */
const last = (a: Issue, b: Issue) => b.createdAt - a.createdAt || (a.key < b.key ? -1 : a.key > b.key ? 1 : 0);

export const comparators: Record<SortBy, Compare> = {
	updated: (a, b) => b.updatedAt - a.updatedAt || last(a, b),
	created: (a, b) => last(a, b),
	priority: (a, b) => priority_rank[a.priority] - priority_rank[b.priority] || b.updatedAt - a.updatedAt || last(a, b),
	title: (a, b) => (a.title < b.title ? -1 : a.title > b.title ? 1 : last(a, b))
};

/** Where `issue` is, or would go, in `sorted`. */
function search(sorted: Issue[], issue: Issue, compare: Compare): number {
	let lo = 0;
	let hi = sorted.length;
	while (lo < hi) {
		const mid = (lo + hi) >> 1;
		if (compare(sorted[mid], issue) < 0) lo = mid + 1;
		else hi = mid;
	}
	return lo;
}

/** Some issues left `issues` and some came in (an edit is both: the old object out, the new one in). */
export interface Step {
	removed: Issue[];
	added: Issue[];
}

/** Past this many changed issues, sorting again is as cheap as splicing each one. */
export const MAX_STEPS = 32;

export const step_count = (steps: Step[]) => steps.reduce((n, s) => n + s.removed.length + s.added.length, 0);

/** The steps as a sub-list sees them: a list filtered by `keep` only ever held, or gets, the issues that pass. */
export const kept = (steps: Step[], keep: (issue: Issue) => boolean): Step[] =>
	steps.map((s) => ({ removed: s.removed.filter(keep), added: s.added.filter(keep) }));

/**
 * `sorted` with the steps applied in order, as a new array; or null when it
 * cannot be done exactly (an issue to remove is not there) and the caller
 * should sort from scratch.
 */
export function apply_steps(sorted: Issue[], steps: Step[], compare: Compare): Issue[] | null {
	const out = sorted.slice();
	for (const { removed, added } of steps) {
		for (const issue of removed) {
			const at = search(out, issue, compare);
			if (out[at] !== issue) return null;
			out.splice(at, 1);
		}
		for (const issue of added) out.splice(search(out, issue, compare), 0, issue);
	}
	return out;
}

/**
 * A sorted list cut into buckets (one per group), with the steps applied.
 * Only the buckets the steps touch are copied. Null as for `apply_steps`.
 */
export function apply_steps_grouped(
	buckets: Map<string, Issue[]>,
	steps: Step[],
	compare: Compare,
	group: (issue: Issue) => string
): Map<string, Issue[]> | null {
	const out = new Map(buckets);
	const touched = new Set<string>();
	for (const { removed, added } of steps) for (const issue of [...removed, ...added]) touched.add(group(issue));
	for (const id of touched) {
		const next = apply_steps(out.get(id) ?? [], kept(steps, (issue) => group(issue) === id), compare);
		if (next === null) return null;
		out.set(id, next);
	}
	return out;
}
