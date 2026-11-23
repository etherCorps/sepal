// The data and the views over it, shared by every screen.
//
// `issues` is `$state.raw`: issue objects are plain and never mutated; an
// edit replaces the object (and the array). Filtering 20k proxies on every
// keystroke would cost more than the filter itself, and a keyed {#each}
// still updates exactly the rows whose object changed.

import {
	DEFAULT_N,
	EPOCH,
	ME,
	PEOPLE,
	PRIORITIES,
	PROJECTS,
	STATUSES,
	WEEK,
	details,
	initials,
	label_by_id,
	now,
	person_by_id,
	priority_by_id,
	project_by_key,
	seed,
	status_by_id,
	type Activity,
	type Comment,
	type Details,
	type Issue,
	type Person,
	type PriorityId,
	type StatusId
} from './seed';
import {
	DEFAULT_SETTINGS,
	clear_edits,
	edit_count,
	empty_overlay,
	flush,
	load,
	persist_stats,
	save_settings,
	schedule_save,
	type Overlay,
	type Settings
} from './persist';
import { goto } from '$app/navigation';
import { MAX_STEPS, apply_steps, apply_steps_grouped, comparators, kept, step_count, type Compare, type SortBy, type Step } from './order';

export type GroupBy = Settings['defaultGroupBy'];
export type { SortBy };
/** The fields an edit may change. */
export type Patch = Partial<Pick<Issue, 'title' | 'status' | 'priority' | 'assignee' | 'project' | 'labelIds' | 'estimate'>>;

export const PAGE_SIZE = 200;
export const BOARD_LIMIT = 50;

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

let n = DEFAULT_N;
/** The seed as generated: what the overlay is a difference from. */
let base: Issue[] = seed(n);
let base_by_key = new Map(base.map((issue) => [issue.key, issue]));
let overlay: Overlay = empty_overlay();

// An edit of one issue out of 20,000 must not cost a pass over all of them
// (in QuickJS a pass is ~5-15 ms, a sort ~30). So `issues` only changes
// through `set_issues`, which says what changed when it knows (a Step), and
// everything kept over all issues follows the steps: the key map and the
// open counts here, the sorted, filtered and grouped lists below (order.ts).

let issues = $state.raw<Issue[]>(base);
/** Not reactive: read `issues` to depend on it (`get_issue` does). */
let by_key = new Map(base.map((issue) => [issue.key, issue]));
let open_by_project = $state.raw(count_open(base));

/** What changed in `issues` since `ordered` last ran, step by step; null when unknown (sort again). */
let pending: Step[] | null = null;

function count_open(list: Issue[]): Record<string, number> {
	const counts: Record<string, number> = Object.fromEntries(PROJECTS.map((p) => [p.key, 0]));
	for (const issue of list) if (issue.closedAt === null) counts[issue.project]++;
	return counts;
}

/** Replace `issues`. Pass what changed when it is known: then nothing goes over every issue again. */
function set_issues(next: Issue[], step?: Step) {
	if (step) {
		if (pending !== null) pending.push(step);
		const counts = { ...open_by_project };
		for (const issue of step.removed) {
			by_key.delete(issue.key);
			if (issue.closedAt === null) counts[issue.project]--;
		}
		for (const issue of step.added) {
			by_key.set(issue.key, issue);
			if (issue.closedAt === null) counts[issue.project]++;
		}
		open_by_project = counts;
	} else {
		pending = null;
		by_key = new Map(next.map((issue) => [issue.key, issue]));
		open_by_project = count_open(next);
	}
	issues = next;
}

/** Bumped when comments or activity change, which live outside `issues`. */
let rev = $state(0);
/** Activity from this session's edits, by key. Not saved: only edits are. */
let session_activity: Record<string, Activity[]> = {};

export const settings = $state<Settings>({ ...DEFAULT_SETTINGS });

/** Startup: `loaded` turns true once edits.json and settings.json were read (or found missing). */
export const boot = $state({ loaded: false });

// ---------------------------------------------------------------------------
// The shared filter and view state (list, board)
// ---------------------------------------------------------------------------

export const view = $state({
	search: '',
	/** '' means all */
	status: '' as StatusId | '',
	priority: '' as PriorityId | '',
	/** a person id, 'none' for unassigned, '' for all */
	assignee: '',
	project: '',
	/** `/issues?week=N`: created in the week N weeks before today (0 = the last 7 days) */
	week: null as number | null,
	/** what the route fixes: `/my` sets `{ assignee: ME }`, `/projects/ENG` `{ project: 'ENG' }` */
	scope: {} as { assignee?: string; project?: string },
	groupBy: DEFAULT_SETTINGS.defaultGroupBy as GroupBy,
	sortBy: 'updated' as SortBy,
	/** how many rows are mounted; `Infinity` after Show all */
	visible: PAGE_SIZE,
	/** group ids that are collapsed */
	collapsed: {} as Record<string, boolean>
});

/** Set by each list route when it mounts. Resets paging. */
export function set_scope(scope: { assignee?: string; project?: string }): void {
	view.scope = scope;
	view.visible = PAGE_SIZE;
}

/** Clear search and the four filter Selects (not the route's scope). */
export function clear_filters(): void {
	view.search = '';
	view.status = '';
	view.priority = '';
	view.assignee = '';
	view.project = '';
	view.week = null;
	view.visible = PAGE_SIZE;
}

export function show_more(): void {
	view.visible += PAGE_SIZE;
}
export function show_all(): void {
	view.visible = Infinity;
}
export function toggle_group(id: string): void {
	view.collapsed[id] = !view.collapsed[id];
}

/** [start, end) of week N, counting back from today. Week 0 includes everything made since. */
export function week_range(week: number): [number, number] {
	return [EPOCH - (week + 1) * WEEK, week === 0 ? Infinity : EPOCH - week * WEEK];
}

/** Which of the last `range` weeks a time falls in, as an index into `Insights.weeks` (oldest first). */
function week_slot(at: number, range: number): number {
	return range - 1 - Math.max(0, Math.min(range - 1, Math.floor((EPOCH - at) / WEEK)));
}

/** Lower-cased "key title", made once per issue object. */
const haystacks = new WeakMap<Issue, string>();
function haystack(issue: Issue): string {
	let text = haystacks.get(issue);
	if (text === undefined) haystacks.set(issue, (text = `${issue.key} ${issue.title}`.toLowerCase()));
	return text;
}

let filter_ms = 0;

/**
 * A list kept over all issues, with the steps that took it from the run
 * before (null: rebuilt from scratch). `seq` counts runs, so what is built
 * on a list can tell whether it saw the run just before this one.
 */
interface Run {
	list: Issue[];
	steps: Step[] | null;
	seq: number;
}

let sorted: { by: SortBy; run: Run } | null = null;

/**
 * All issues in sort order: sorted once per sort change, not per keystroke.
 * An edit of a few issues splices them out and back in (order.ts), ~30 ms
 * less than sorting 20,000 again in QuickJS.
 */
const ordered = $derived.by((): Run => {
	const by = view.sortBy;
	const all = issues;
	const steps = pending;
	pending = [];
	const compare = comparators[by];
	const seq = (sorted?.run.seq ?? 0) + 1;
	let list: Issue[] | null = null;
	if (sorted?.by === by && steps !== null && step_count(steps) <= MAX_STEPS) list = apply_steps(sorted.run.list, steps, compare);
	const run: Run = list === null ? { list: all.slice().sort(compare), steps: null, seq } : { list, steps, seq };
	sorted = { by, run };
	return run;
});

/** The filters and search as a test, and a key that changes when they do. */
function matcher(): { key: string; test: (issue: Issue) => boolean } {
	const query = view.search.trim().toLowerCase();
	const { status, priority, assignee, project, week, scope } = view;
	const [from, to] = week === null ? [0, 0] : week_range(week);
	const who = assignee === 'none' ? null : assignee;
	return {
		key: JSON.stringify([query, status, priority, assignee, project, week, scope.project, scope.assignee]),
		test: (issue) =>
			!(
				(scope.project && issue.project !== scope.project) ||
				(scope.assignee && issue.assignee !== scope.assignee) ||
				(status && issue.status !== status) ||
				(priority && issue.priority !== priority) ||
				(assignee && issue.assignee !== who) ||
				(project && issue.project !== project) ||
				(week !== null && (issue.createdAt < from || issue.createdAt >= to)) ||
				(query && !haystack(issue).includes(query))
			)
	};
}

let filtered_last: { from: number; key: string; run: Run } | null = null;

/** Scope, filters and search applied, in sort order. */
const filtered_run = $derived.by((): Run => {
	const start = performance.now();
	const source = ordered;
	const { key, test } = matcher();
	const last = filtered_last;
	const seq = (last?.run.seq ?? 0) + 1;
	let run: Run | null = null;
	if (last && source.steps !== null && source.seq === last.from + 1 && key === last.key) {
		const steps = kept(source.steps, test);
		const list = apply_steps(last.run.list, steps, comparators[view.sortBy]);
		if (list !== null) run = { list, steps, seq };
	}
	if (run === null) {
		const list: Issue[] = [];
		const all = source.list;
		for (let i = 0; i < all.length; i++) if (test(all[i])) list.push(all[i]);
		run = { list, steps: null, seq };
	}
	filtered_last = { from: source.seq, key, run };
	filter_ms = performance.now() - start;
	return run;
});
const filtered = $derived(filtered_run.list);

export interface Group {
	/** a status, priority, person or project id; 'none' for unassigned; 'all' when not grouped */
	id: string;
	name: string;
	issues: Issue[];
}

/** The order groups appear in, and their names. */
function group_order(by: GroupBy): { id: string; name: string }[] {
	switch (by) {
		case 'status':
			return STATUSES;
		case 'priority':
			return PRIORITIES;
		case 'project':
			return PROJECTS.map((p) => ({ id: p.key, name: p.name }));
		case 'assignee':
			return [...PEOPLE.map((p) => ({ id: p.id, name: person(p.id)!.name })), { id: 'none', name: 'Unassigned' }];
		case 'none':
			return [{ id: 'all', name: 'All issues' }];
	}
}
function group_of(issue: Issue, by: GroupBy): string {
	switch (by) {
		case 'status':
			return issue.status;
		case 'priority':
			return issue.priority;
		case 'project':
			return issue.project;
		case 'assignee':
			return issue.assignee ?? 'none';
		case 'none':
			return 'all';
	}
}

/**
 * The filtered list in buckets by `by`, kept through the filter's steps. One
 * per user (the list's groups, the board's columns): each remembers the run
 * it last saw.
 */
function grouper() {
	let last: { seq: number; by: GroupBy; buckets: Map<string, Issue[]> } | null = null;
	return (run: Run, by: GroupBy, compare: Compare): Map<string, Issue[]> => {
		if (last && run.seq === last.seq && by === last.by) return last.buckets;
		let buckets: Map<string, Issue[]> | null = null;
		if (last && run.steps !== null && run.seq === last.seq + 1 && by === last.by) {
			buckets = apply_steps_grouped(last.buckets, run.steps, compare, (issue) => group_of(issue, by));
		}
		if (buckets === null) {
			buckets = new Map();
			for (const { id } of group_order(by)) buckets.set(id, []);
			const all = run.list;
			for (let i = 0; i < all.length; i++) buckets.get(group_of(all[i], by))?.push(all[i]);
		}
		last = { seq: run.seq, by, buckets };
		return buckets;
	};
}
const list_groups = grouper();
const board_columns = grouper();

/** Sorted issues in groups; empty groups left out. */
const grouped = $derived.by((): Group[] => {
	const by = view.groupBy;
	const buckets = list_groups(filtered_run, by, comparators[view.sortBy]);
	return group_order(by)
		.map(({ id, name }) => ({ id, name, issues: buckets.get(id) ?? [] }))
		.filter((group) => group.issues.length > 0);
});

export interface PagedGroup extends Group {
	/** only the rows that are mounted */
	issues: Issue[];
	/** every issue in the group */
	total: number;
	collapsed: boolean;
}
export interface Paged {
	groups: PagedGroup[];
	/** the mounted rows in screen order: what j/k and Shift+click walk */
	rows: Issue[];
	shown: number;
	/** issues left for Show more, not counting collapsed groups */
	hidden: number;
}

/** What the list mounts: the first `view.visible` rows, across groups. */
const paged = $derived.by((): Paged => {
	let budget = view.visible;
	const groups: PagedGroup[] = [];
	const rows: Issue[] = [];
	let open_total = 0;
	for (const group of grouped) {
		const collapsed = view.collapsed[group.id] === true;
		if (!collapsed) open_total += group.issues.length;
		if (budget <= 0) continue;
		const shown = collapsed ? [] : group.issues.length <= budget ? group.issues : group.issues.slice(0, budget);
		budget -= shown.length;
		for (const issue of shown) rows.push(issue);
		groups.push({ ...group, issues: shown, total: group.issues.length, collapsed });
	}
	return { groups, rows, shown: rows.length, hidden: open_total - rows.length };
});

export interface Column {
	status: (typeof STATUSES)[number];
	/** at most BOARD_LIMIT */
	issues: Issue[];
	total: number;
}

/** The board: one column per status, the list's filters and sort. */
const board = $derived.by((): Column[] => {
	const buckets = board_columns(filtered_run, 'status', comparators[view.sortBy]);
	return STATUSES.map((status) => {
		const all = buckets.get(status.id) ?? [];
		return { status, issues: all.length <= BOARD_LIMIT ? all : all.slice(0, BOARD_LIMIT), total: all.length };
	});
});

// ---------------------------------------------------------------------------
// Insights: aggregates over every issue (not the list's filters)
// ---------------------------------------------------------------------------

export const insights_view = $state({ range: 12 as 4 | 12 | 26, project: '' });

export interface Week {
	/** weeks before today: 0 is the last 7 days. `/issues?week=N` uses it. */
	index: number;
	start: number;
	created: number;
	closed: number;
}
export interface Insights {
	open: number;
	created: number;
	closed: number;
	median_age_days: number;
	/** change against the start of the range (open, median age) or the range before it (created, closed) */
	delta: { open: number; created: number; closed: number; median_age_days: number };
	/** oldest first */
	weeks: Week[];
	/** open issues, top 10 assignees; person null is unassigned */
	by_assignee: { person: string | null; count: number }[];
	/** open issues, in PRIORITIES order */
	by_priority: { priority: PriorityId; count: number }[];
	/** the 10 oldest open issues */
	oldest: Issue[];
}

function median(values: number[]): number {
	if (values.length === 0) return 0;
	values.sort((a, b) => a - b);
	const mid = values.length >> 1;
	return values.length % 2 ? values[mid] : Math.round((values[mid - 1] + values[mid]) / 2);
}

const insights = $derived.by((): Insights => {
	const { range, project } = insights_view;
	const start = EPOCH - range * WEEK;
	const previous = EPOCH - 2 * range * WEEK;
	const weeks: Week[] = [];
	for (let w = range - 1; w >= 0; w--) weeks.push({ index: w, start: week_range(w)[0], created: 0, closed: 0 });
	let open = 0;
	let open_then = 0;
	let created = 0;
	let created_before = 0;
	let closed = 0;
	let closed_before = 0;
	const ages: number[] = [];
	const ages_then: number[] = [];
	const assignees = new Map<string | null, number>();
	const priorities = Object.fromEntries(PRIORITIES.map((p) => [p.id, 0])) as Record<PriorityId, number>;
	const open_issues: Issue[] = [];
	for (const issue of issues) {
		if (project && issue.project !== project) continue;
		const { createdAt, closedAt } = issue;
		if (closedAt === null) {
			open++;
			ages.push(Math.max(0, EPOCH - createdAt));
			assignees.set(issue.assignee, (assignees.get(issue.assignee) ?? 0) + 1);
			priorities[issue.priority]++;
			open_issues.push(issue);
		}
		if (createdAt <= start && (closedAt === null || closedAt > start)) {
			open_then++;
			ages_then.push(start - createdAt);
		}
		if (createdAt >= start) {
			created++;
			weeks[week_slot(createdAt, range)].created++;
		} else if (createdAt >= previous) created_before++;
		if (closedAt !== null) {
			if (closedAt >= start) {
				closed++;
				weeks[week_slot(closedAt, range)].closed++;
			} else if (closedAt >= previous) closed_before++;
		}
	}
	const median_age_days = Math.round(median(ages) / 86_400_000);
	open_issues.sort((a, b) => a.createdAt - b.createdAt);
	return {
		open,
		created,
		closed,
		median_age_days,
		delta: {
			open: open - open_then,
			created: created - created_before,
			closed: closed - closed_before,
			median_age_days: median_age_days - Math.round(median(ages_then) / 86_400_000)
		},
		weeks,
		by_assignee: [...assignees]
			.map(([person, count]) => ({ person, count }))
			.sort((a, b) => b.count - a.count)
			.slice(0, 10),
		by_priority: PRIORITIES.map((p) => ({ priority: p.id, count: priorities[p.id] })),
		oldest: open_issues.slice(0, 10)
	};
});

/** Everything derived, read through getters so it stays reactive. */
export const data = {
	/** every issue, seed order with created ones first */
	get issues() {
		return issues;
	},
	get total() {
		return issues.length;
	},
	get filtered() {
		return filtered;
	},
	get sorted() {
		return filtered;
	},
	get groups() {
		return grouped;
	},
	get paged() {
		return paged;
	},
	get board() {
		return board;
	},
	get insights() {
		return insights;
	},
	/** open issues per project key, for the sidebar */
	get open_by_project() {
		return open_by_project;
	},
	/** edits held in the overlay (Settings' "37 local edits") */
	get edits() {
		rev;
		return edit_count(overlay);
	}
};

export function get_issue(key: string): Issue | undefined {
	issues; // by_key is a plain Map: depend on what it follows
	return by_key.get(key);
}

/** The key `delta` rows after `key` among the mounted rows (j/k), or null at the ends. */
export function neighbor(key: string | null, delta: number): string | null {
	const rows = paged.rows;
	if (rows.length === 0) return null;
	const at = key === null ? -1 : rows.findIndex((issue) => issue.key === key);
	if (at === -1) return rows[delta > 0 ? 0 : rows.length - 1].key;
	return rows[at + delta]?.key ?? null;
}

/** A person, with the user's display name from settings. Reactive. */
export function person(id: string | null): Person | null {
	if (id === null) return null;
	if (id === ME) {
		const name = settings.displayName.trim() || DEFAULT_SETTINGS.displayName;
		return { ...person_by_id[ME], name, initials: initials(name) };
	}
	return person_by_id[id] ?? null;
}

/** Description, comments (seed then the user's) and activity (seed then this session's), oldest first. */
export function thread(key: string): Details {
	rev;
	const seeded = base_by_key.get(key);
	const issue = seeded ?? get_issue(key);
	const generated: Details = seeded
		? details(seeded)
		: { description: '', comments: [], activity: issue ? [{ author: ME, text: 'created the issue', at: issue.createdAt }] : [] };
	return {
		description: generated.description,
		comments: [...generated.comments, ...(overlay.comments[key] ?? [])],
		activity: [...generated.activity, ...(session_activity[key] ?? [])]
	};
}

// ---------------------------------------------------------------------------
// Edits
// ---------------------------------------------------------------------------

const FIELDS = ['project', 'title', 'status', 'priority', 'assignee', 'labelIds', 'estimate', 'updatedAt', 'closedAt'] as const;
const same = (a: unknown, b: unknown) => (Array.isArray(a) && Array.isArray(b) ? a.join() === b.join() : a === b);

/** Record an issue's current state in the overlay: a diff for seed issues, the whole issue for created ones. */
function put_overlay(issue: Issue) {
	const original = base_by_key.get(issue.key);
	if (original === undefined) {
		const at = overlay.created.findIndex((c) => c.key === issue.key);
		if (at !== -1) overlay.created[at] = issue;
		return;
	}
	const diff: Partial<Issue> = {};
	let any = false;
	for (const field of FIELDS) {
		if (!same(issue[field], original[field])) {
			(diff as any)[field] = issue[field];
			any = true;
		}
	}
	if (any) overlay.changed[issue.key] = diff;
	else delete overlay.changed[issue.key];
}

function save() {
	rev++;
	schedule_save(() => overlay);
}

/** Put changed issue objects in place of the ones with their keys. */
function replace(changed: Issue[]) {
	// slice and indexOf run in C; a map() over 20,000 issues calls back 20,000 times
	const next = issues.slice();
	const step: Step = { removed: [], added: [] };
	for (const issue of changed) {
		const old = by_key.get(issue.key);
		const at = old === undefined ? -1 : next.indexOf(old);
		if (at === -1) continue;
		next[at] = issue;
		step.removed.push(old!);
		step.added.push(issue);
	}
	set_issues(next, step);
	for (const issue of changed) put_overlay(issue);
	save();
}

const labels_text = (ids: string[]) => ids.map((id) => label_by_id[id]?.name ?? id).join(', ') || 'none';
function describe(patch: Patch): { text: string; status?: StatusId }[] {
	const out: { text: string; status?: StatusId }[] = [];
	if (patch.title !== undefined) out.push({ text: 'changed the title' });
	if (patch.status !== undefined) out.push({ text: `changed status to ${status_by_id[patch.status].name}`, status: patch.status });
	if (patch.priority !== undefined) out.push({ text: `set priority to ${priority_by_id[patch.priority].name}` });
	if (patch.assignee !== undefined) out.push({ text: patch.assignee ? `assigned ${person(patch.assignee)?.name}` : 'removed the assignee' });
	if (patch.project !== undefined) out.push({ text: `moved it to ${project_by_key[patch.project]?.name}` });
	if (patch.labelIds !== undefined) out.push({ text: `set labels to ${labels_text(patch.labelIds)}` });
	if (patch.estimate !== undefined) out.push({ text: `set the estimate to ${patch.estimate ?? 'none'}` });
	return out;
}

function changed_issue(issue: Issue, patch: Patch, at: number): Issue {
	const next = { ...issue, ...patch, updatedAt: at };
	if (patch.status !== undefined) {
		const open = status_by_id[patch.status].open;
		next.closedAt = open ? null : (issue.closedAt ?? at);
	}
	return next;
}

interface Undo {
	/** what the toast says: `Changed status of ENG-12` */
	label: string;
	revert: () => void;
}
const undo_stack: Undo[] = [];
/** For an Undo button that should only show when there is something to undo. */
export const history = $state({ size: 0, last: '' });

function push_undo(label: string, revert: () => void) {
	undo_stack.push({ label, revert });
	if (undo_stack.length > 50) undo_stack.shift();
	history.size = undo_stack.length;
	history.last = label;
}

function revert(entry: Undo | undefined): string | null {
	const at = entry ? undo_stack.lastIndexOf(entry) : -1;
	if (!entry || at === -1) return null;
	undo_stack.splice(at, 1);
	history.size = undo_stack.length;
	history.last = undo_stack[undo_stack.length - 1]?.label ?? '';
	entry.revert();
	return entry.label;
}

/** Revert the last edit made through this module. Returns its label, or null if there was none. */
export function undo(): string | null {
	return revert(undo_stack[undo_stack.length - 1]);
}

/** A toast's Undo for the edit just made: it reverts that edit, not whatever is newest when clicked. */
export function undo_action(): { label: string; run: () => void } {
	const entry = undo_stack[undo_stack.length - 1];
	return { label: 'Undo', run: () => void revert(entry) };
}

/** Returns the entries it added, so an undo removes exactly those. */
function add_activity(keys: string[], patch: Patch, author: string, at: number): Activity[] {
	const added: Activity[] = [];
	for (const entry of describe(patch)) {
		for (const key of keys) {
			const item = { author, at, ...entry };
			(session_activity[key] ??= []).push(item);
			added.push(item);
		}
	}
	return added;
}
function drop_activity(keys: string[], added: Activity[]) {
	const gone = new Set(added);
	for (const key of keys) if (session_activity[key]) session_activity[key] = session_activity[key].filter((a) => !gone.has(a));
}

/**
 * Change fields of some issues as one edit (one Undo). Unknown keys are skipped.
 * Returns the issues as they are now.
 */
export function bulk_update(keys: string[], patch: Patch, author: string = ME): Issue[] {
	const at = now();
	const before: Issue[] = [];
	const after: Issue[] = [];
	for (const key of keys) {
		const issue = by_key.get(key);
		if (!issue) continue;
		before.push(issue);
		after.push(changed_issue(issue, patch, at));
	}
	if (after.length === 0) return [];
	const done = after.map((issue) => issue.key);
	const added = add_activity(done, patch, author, at);
	replace(after);
	const label = done.length === 1 ? `Updated ${done[0]}` : `Updated ${done.length} issues`;
	if (author === ME) {
		const fields = Object.keys(patch) as (keyof Patch)[];
		push_undo(label, () => {
			drop_activity(done, added);
			// Put back only what this edit changed: a teammate may have edited the issue since.
			const reverted: Issue[] = [];
			for (const old of before) {
				const current = by_key.get(old.key);
				if (!current) continue;
				const next: any = { ...current };
				for (const field of fields) next[field] = old[field];
				if (patch.status !== undefined) next.closedAt = old.closedAt;
				if (current.updatedAt === at) next.updatedAt = old.updatedAt;
				reverted.push(next);
			}
			if (reverted.length > 0) replace(reverted);
		});
	}
	return after;
}

/** Change fields of one issue. Returns it as it is now, or undefined for an unknown key. */
export function update(key: string, patch: Patch): Issue | undefined {
	return bulk_update([key], patch)[0];
}

export interface NewIssue {
	title: string;
	project: string;
	status?: StatusId;
	priority?: PriorityId;
	assignee?: string | null;
	labelIds?: string[];
	estimate?: number | null;
}

/** Add an issue at the top of the list. Returns it (its key is `PROJECT-next number`). */
export function create(fields: NewIssue): Issue {
	// By key, not `issue.project`: a key stays when its issue moves to another project.
	// Keys that only live on in comments or activity (deleted, undone) are not handed out again.
	const prefix = `${fields.project}-`;
	let number = 0;
	const take = (key: string) => {
		if (key.startsWith(prefix)) number = Math.max(number, Number(key.slice(prefix.length)) || 0);
	};
	for (const issue of issues) take(issue.key);
	for (const issue of base) take(issue.key);
	for (const key in overlay.comments) take(key);
	for (const key in session_activity) take(key);
	const at = now();
	const status = fields.status ?? 'todo';
	const issue: Issue = {
		key: `${fields.project}-${number + 1}`,
		project: fields.project,
		number: number + 1,
		title: fields.title.trim(),
		status,
		priority: fields.priority ?? 'none',
		assignee: fields.assignee ?? null,
		labelIds: fields.labelIds ?? [],
		estimate: fields.estimate ?? null,
		createdAt: at,
		updatedAt: at,
		closedAt: status_by_id[status].open ? null : at
	};
	set_issues([issue, ...issues], { removed: [], added: [issue] });
	overlay.created.unshift(issue);
	save();
	push_undo(`Created ${issue.key}`, () => {
		set_issues(
			issues.filter((i) => i.key !== issue.key),
			{ removed: issues.filter((i) => i.key === issue.key), added: [] }
		);
		overlay.created = overlay.created.filter((i) => i.key !== issue.key);
		save();
	});
	return issue;
}

/** Delete issues as one edit. Returns how many went. */
export function delete_issues(keys: string[]): number {
	const gone = new Set(keys);
	const removed: { index: number; issue: Issue }[] = [];
	issues.forEach((issue, index) => {
		if (gone.has(issue.key)) removed.push({ index, issue });
	});
	if (removed.length === 0) return 0;
	const created = overlay.created.filter((issue) => gone.has(issue.key));
	set_issues(
		issues.filter((issue) => !gone.has(issue.key)),
		{ removed: removed.map((r) => r.issue), added: [] }
	);
	overlay.created = overlay.created.filter((issue) => !gone.has(issue.key));
	const seeded = removed.map((r) => r.issue.key).filter((key) => base_by_key.has(key));
	overlay.deleted.push(...seeded);
	save();
	push_undo(removed.length === 1 ? `Deleted ${removed[0].issue.key}` : `Deleted ${removed.length} issues`, () => {
		const next = issues.slice();
		for (const { index, issue } of removed) next.splice(index, 0, issue);
		set_issues(next, { removed: [], added: removed.map((r) => r.issue) });
		overlay.created = [...created, ...overlay.created];
		const back = new Set(seeded);
		overlay.deleted = overlay.deleted.filter((key) => !back.has(key));
		save();
	});
	return removed.length;
}

/** Post a comment as the user. */
export function add_comment(key: string, body: string): Comment | undefined {
	const text = body.trim();
	if (!text || !by_key.has(key)) return undefined;
	const comment: Comment = { author: ME, body: text, at: now() };
	(overlay.comments[key] ??= []).push(comment);
	save();
	push_undo(`Commented on ${key}`, () => {
		const list = overlay.comments[key];
		const at = list?.lastIndexOf(comment) ?? -1;
		if (at !== -1) list.splice(at, 1);
		if (list?.length === 0) delete overlay.comments[key];
		save();
	});
	return comment;
}

/** The seed with an overlay applied: deleted dropped, changes merged, created first. */
function apply(overlay: Overlay): Issue[] {
	const deleted = new Set(overlay.deleted);
	const out: Issue[] = [...overlay.created];
	for (const issue of base) {
		if (deleted.has(issue.key)) continue;
		const changed = overlay.changed[issue.key];
		out.push(changed ? { ...issue, ...changed } : issue);
	}
	return out;
}

/** Settings' "Reset to sample data": drops every edit, on disk too. */
export function reset_edits(): Promise<void> {
	overlay = empty_overlay();
	session_activity = {};
	undo_stack.length = 0;
	history.size = 0;
	history.last = '';
	set_issues(base);
	rev++;
	return clear_edits();
}

/** Save settings.json now. Call after changing `settings`. */
export function store_settings(): Promise<void> {
	return save_settings($state.snapshot(settings));
}

/** Write a pending edits.json now instead of in 500 ms. */
export const save_now = flush;

/** CSV of the given issues (default: the list's current filter, sorted). */
export function to_csv(list: Issue[] = filtered): string {
	const cell = (value: unknown) => {
		const text = value === null || value === undefined ? '' : String(value);
		return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
	};
	const rows = [['key', 'title', 'status', 'priority', 'assignee', 'project', 'labels', 'estimate', 'created', 'updated'].join(',')];
	for (const issue of list) {
		rows.push(
			[
				issue.key,
				issue.title,
				status_by_id[issue.status].name,
				priority_by_id[issue.priority].name,
				person(issue.assignee)?.name ?? '',
				issue.project,
				issue.labelIds.map((id) => label_by_id[id]?.name).join('; '),
				issue.estimate,
				new Date(issue.createdAt).toISOString(),
				new Date(issue.updatedAt).toISOString()
			]
				.map(cell)
				.join(',')
		);
	}
	return rows.join('\n');
}

// ---------------------------------------------------------------------------
// Simulated teammates: one 2 s interval, only while the setting is on
// ---------------------------------------------------------------------------

let teammates: ReturnType<typeof setInterval> | null = null;
let teammate_tick = 0;

/** One edit by a random teammate: what the interval does, callable by tests. */
export function simulate_once(): Issue | undefined {
	const tick = teammate_tick++;
	const list = issues;
	if (list.length === 0) return undefined;
	// Deterministic: the n-th edit always hits the same issue with the same change.
	const issue = list[(tick * 7919) % list.length];
	const author = PEOPLE[1 + (tick % (PEOPLE.length - 1))].id;
	const next = STATUSES[(STATUSES.findIndex((s) => s.id === issue.status) + 1) % STATUSES.length].id;
	return bulk_update([issue.key], { status: next }, author)[0];
}

/** Start or stop the teammates. Sets `settings.simulate`; call `store_settings()` to keep it. */
export function set_simulate(on: boolean): void {
	settings.simulate = on;
	if (on && teammates === null) teammates = setInterval(simulate_once, 2000);
	if (!on && teammates !== null) {
		clearInterval(teammates);
		teammates = null;
	}
}

// ---------------------------------------------------------------------------
// Startup and the harness hook
// ---------------------------------------------------------------------------

/** Replace the seed with N issues and apply the overlay again. For the bench. */
function reseed(count: number) {
	n = count;
	base = seed(n);
	base_by_key = new Map(base.map((issue) => [issue.key, issue]));
	set_issues(apply(overlay));
	undo_stack.length = 0;
	history.size = 0;
	view.visible = PAGE_SIZE;
	rev++;
	return stats();
}

function stats() {
	return {
		n,
		issues: issues.length,
		filtered: filtered.length,
		filter_ms,
		stringify_ms: persist_stats.stringify_ms,
		saves: persist_stats.saves,
		edits: edit_count(overlay),
		loaded: boot.loaded,
		persist: persist_stats.available
	};
}

// Pinned on globalThis: QuickJS frees module values nothing live refers to,
// and this keeps the store reachable for the harness whatever is mounted.
// `update` and `save_now` let the bench make overlay edits without clicking (P10);
// `goto` reaches URLs nothing links to (a 404) and navigates from a timer.
(globalThis as any).__tracker = { reseed, stats, simulate_once, update, bulk_update, create, undo, thread, save_now: flush, issue: get_issue, delete: delete_issues, goto };

load().then(({ overlay: saved, settings: loaded }) => {
	Object.assign(settings, loaded);
	view.groupBy = loaded.defaultGroupBy;
	if (saved.created.length || saved.deleted.length || Object.keys(saved.changed).length || Object.keys(saved.comments).length) {
		overlay = saved;
		set_issues(apply(overlay));
		rev++;
	}
	if (loaded.simulate) set_simulate(true);
	boot.loaded = true;
});
