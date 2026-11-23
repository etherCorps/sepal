// The sample data: every issue comes from mulberry32(42) and word lists, so a
// run is the same headless or in a window, on either engine, at any N.
// Plain TypeScript with no imports: test/seed.test.ts runs it in Node.

export type StatusId = 'backlog' | 'todo' | 'progress' | 'review' | 'done' | 'canceled';
export type PriorityId = 'urgent' | 'high' | 'medium' | 'low' | 'none';

export interface Status {
	id: StatusId;
	name: string;
	/** done and canceled are closed */
	open: boolean;
}
export interface Priority {
	id: PriorityId;
	name: string;
}
export interface Person {
	id: string;
	name: string;
	initials: string;
	/** 1 to 12: which `--hue-N` the avatar uses */
	hue: number;
}
export interface Project {
	key: string;
	name: string;
	hue: number;
}
export interface Label {
	id: string;
	name: string;
	hue: number;
}
export interface Issue {
	/** `ENG-123`; never changes, even when the project does */
	key: string;
	project: string;
	number: number;
	title: string;
	status: StatusId;
	priority: PriorityId;
	/** a person id, or null when unassigned */
	assignee: string | null;
	labelIds: string[];
	estimate: number | null;
	/** ms since 1970, UTC */
	createdAt: number;
	updatedAt: number;
	/** when it became done or canceled; null while open */
	closedAt: number | null;
}
export interface Comment {
	author: string;
	body: string;
	at: number;
}
export interface Activity {
	author: string;
	/** `changed status to Done`, `created the issue` */
	text: string;
	at: number;
	/** the status it moved to, for the glyph */
	status?: StatusId;
}
export interface Details {
	description: string;
	comments: Comment[];
	activity: Activity[];
}

/** "Today" for the app: 23 Nov 2026, 12:00 UTC. Dates fall in the 26 weeks before it. */
export const EPOCH = Date.UTC(2026, 10, 23, 12, 0);
export const DAY = 86_400_000;
export const WEEK = 7 * DAY;
export const DEFAULT_N = 5000;

/**
 * The time of an edit. ponytail: the seed's epoch plus time since start, so
 * dates stay on the seed's calendar (Insights) and headless runs, where
 * `performance.now()` stands still, stay deterministic. Edits saved by an
 * earlier session can sort after this session's; use `Date.now()` if that matters.
 */
export function now(): number {
	return EPOCH + Math.floor(performance.now());
}

/** Display order: the board's columns, the list's groups. */
export const STATUSES: Status[] = [
	{ id: 'backlog', name: 'Backlog', open: true },
	{ id: 'todo', name: 'Todo', open: true },
	{ id: 'progress', name: 'In Progress', open: true },
	{ id: 'review', name: 'In Review', open: true },
	{ id: 'done', name: 'Done', open: false },
	{ id: 'canceled', name: 'Canceled', open: false }
];
/** Urgent first: sorting by priority uses this order. */
export const PRIORITIES: Priority[] = [
	{ id: 'urgent', name: 'Urgent' },
	{ id: 'high', name: 'High' },
	{ id: 'medium', name: 'Medium' },
	{ id: 'low', name: 'Low' },
	{ id: 'none', name: 'No priority' }
];

const NAMES = [
	'Shivam', 'Ada Lovelace', 'Grace Hopper', 'Alan Turing', 'Linus Pauling', 'Margaret Hamilton',
	'Ken Thompson', 'Barbara Liskov', 'Dennis Ritchie', 'Frances Allen', 'John Backus', 'Radia Perlman',
	'Edsger Dijkstra', 'Hedy Lamarr', 'Donald Knuth', 'Katherine Johnson', 'Tim Berners-Lee', 'Sophie Wilson',
	'Bjarne Stroustrup', 'Anita Borg', 'Guido van Rossum', 'Karen Jones', 'Leslie Lamport', 'Jean Sammet',
	'Niklaus Wirth'
];
/** 25 people. `people[0]` is the app's user ("me"); settings rename them. */
export const PEOPLE: Person[] = NAMES.map((name, i) => ({
	id: `p${i + 1}`,
	name,
	initials: initials(name),
	hue: (hash(name) % 12) + 1
}));
export const ME = 'p1';

export const PROJECTS: Project[] = [
	{ key: 'ENG', name: 'Engine', hue: 1 },
	{ key: 'WEB', name: 'Web', hue: 2 },
	{ key: 'API', name: 'API', hue: 3 },
	{ key: 'OPS', name: 'Ops', hue: 4 },
	{ key: 'DES', name: 'Design', hue: 5 }
];

export const LABELS: Label[] = [
	'bug', 'feature', 'performance', 'security', 'docs', 'regression',
	'ux', 'infra', 'tech debt', 'flaky', 'customer', 'good first issue'
].map((name, i) => ({ id: `l${i + 1}`, name, hue: i + 1 }));

export function initials(name: string): string {
	const parts = name.trim().split(/\s+/).filter(Boolean);
	if (parts.length === 0) return '?';
	const first = parts[0][0];
	const last = parts.length > 1 ? parts[parts.length - 1][0] : (parts[0][1] ?? '');
	return (first + last).toUpperCase();
}

/** FNV-1a: a stable number for a string (avatar hues, lazy details). */
export function hash(text: string): number {
	let h = 0x811c9dc5;
	for (let i = 0; i < text.length; i++) {
		h ^= text.charCodeAt(i);
		h = Math.imul(h, 0x01000193);
	}
	return h >>> 0;
}

export function mulberry32(seed: number): () => number {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

type Rng = () => number;
const pick = <T>(rng: Rng, list: readonly T[]): T => list[Math.floor(rng() * list.length)];
const between = (rng: Rng, low: number, high: number) => low + Math.floor(rng() * (high - low + 1));
/** Pick by weight: `weights[i]` is how likely `list[i]` is. */
function weighted<T>(rng: Rng, list: readonly T[], weights: readonly number[]): T {
	let total = 0;
	for (const w of weights) total += w;
	let r = rng() * total;
	for (let i = 0; i < list.length; i++) {
		r -= weights[i];
		if (r < 0) return list[i];
	}
	return list[list.length - 1];
}

const VERBS = [
	'Fix', 'Add', 'Remove', 'Refactor', 'Speed up', 'Document', 'Investigate', 'Migrate', 'Cache',
	'Retry', 'Validate', 'Log', 'Rename', 'Split', 'Deprecate', 'Test', 'Profile', 'Simplify'
];
const THINGS = [
	'deploy pipeline', 'login form', 'search index', 'billing worker', 'webhook retries', 'rate limiter',
	'session tokens', 'image uploads', 'CSV export', 'audit log', 'feature flags', 'email digest',
	'onboarding flow', 'API pagination', 'dark mode toggle', 'error boundary', 'build cache',
	'deploy preview', 'notification badge', 'settings page', 'access control', 'query planner',
	'metrics dashboard', 'invite links', 'release notes', 'keyboard shortcuts', 'sync engine',
	'crash reporter', 'payment retries', 'backup job'
];
const TAILS = [
	'', '', '', 'on Safari', 'for large teams', 'after timeout', 'in staging', 'under load',
	'on first run', 'when offline', 'for admins', 'behind the proxy', 'on Windows', 'during deploy'
];
const SENTENCES = [
	'This shows up for about one in twenty users.',
	'Steps to reproduce are in the linked thread.',
	'The last deploy made it worse.',
	'We should fix this before the next release.',
	'It only happens with more than a thousand rows.',
	'The logs point at a race between two workers.',
	'A customer reported it twice this week.',
	'There is a workaround, but it is not obvious.',
	'This blocks the migration to the new cluster.',
	'Design has a mock for the expected behaviour.',
	'The timeout is set to 30 seconds and that is too short.',
	'We added a metric for it, so we can see if a fix helps.'
];
const REPLIES = [
	'I can take this one.', 'Looks like the same root cause as last month.', 'Reproduced locally.',
	'Can we get a test for this?', 'Pushed a fix, waiting for review.', 'This is still happening on main.',
	'Moving it to next week.', 'Does this need a migration?', 'Confirmed with the customer.',
	'Closing once the deploy is out.', 'Added logs, will check tomorrow.', 'Not sure this is worth it.'
];

const STATUS_IDS = STATUSES.map((s) => s.id);
const STATUS_WEIGHTS = [15, 20, 12, 8, 38, 7];
const PRIORITY_IDS = PRIORITIES.map((p) => p.id);
const PRIORITY_WEIGHTS = [5, 18, 32, 25, 20];
const PROJECT_WEIGHTS = [30, 25, 20, 15, 10];
const ESTIMATES = [null, 1, 2, 3, 5, 8];

/** N issues, newest first. The same N always gives the same issues. */
export function seed(n: number = DEFAULT_N): Issue[] {
	const rng = mulberry32(42);
	const numbers: Record<string, number> = {};
	const issues: Issue[] = [];
	const span = 26 * WEEK;
	for (let i = 0; i < n; i++) {
		const project = weighted(rng, PROJECTS, PROJECT_WEIGHTS).key;
		const number = (numbers[project] = (numbers[project] ?? 0) + 1);
		const tail = pick(rng, TAILS);
		const title = `${pick(rng, VERBS)} ${pick(rng, THINGS)}${tail ? ' ' + tail : ''}`;
		const status = weighted(rng, STATUS_IDS, STATUS_WEIGHTS);
		const createdAt = EPOCH - Math.floor(rng() * span);
		const updatedAt = createdAt + Math.floor(rng() * (EPOCH - createdAt));
		const open = status !== 'done' && status !== 'canceled';
		const labelIds: string[] = [];
		const label_count = weighted(rng, [0, 1, 2, 3], [20, 45, 28, 7]);
		while (labelIds.length < label_count) {
			const id = pick(rng, LABELS).id;
			if (!labelIds.includes(id)) labelIds.push(id);
		}
		issues.push({
			key: `${project}-${number}`,
			project,
			number,
			title,
			status,
			priority: weighted(rng, PRIORITY_IDS, PRIORITY_WEIGHTS),
			assignee: rng() < 0.12 ? null : pick(rng, PEOPLE).id,
			labelIds,
			estimate: pick(rng, ESTIMATES),
			createdAt,
			updatedAt,
			closedAt: open ? null : updatedAt
		});
	}
	// Newest first, as a tracker opens.
	issues.sort((a, b) => b.createdAt - a.createdAt);
	return issues;
}

const details_cache = new Map<string, Details>();

/**
 * Description, comments and activity for an issue, made from hash(key) the
 * first time they are asked for. Only what an issue looked like at seed
 * time matters, so pass the seed's issue, not an edited one.
 */
export function details(issue: Issue): Details {
	let found = details_cache.get(issue.key);
	if (found) return found;
	const rng = mulberry32(hash(issue.key));
	const sentences = between(rng, 2, 5);
	const description: string[] = [];
	for (let i = 0; i < sentences; i++) description.push(pick(rng, SENTENCES));

	const span = Math.max(EPOCH - issue.createdAt, 1);
	const at = () => issue.createdAt + Math.floor(rng() * span);
	const comments: Comment[] = [];
	const comment_count = between(rng, 0, 8);
	for (let i = 0; i < comment_count; i++) comments.push({ author: pick(rng, PEOPLE).id, body: pick(rng, REPLIES), at: at() });
	comments.sort((a, b) => a.at - b.at);

	const activity: Activity[] = [{ author: pick(rng, PEOPLE).id, text: 'created the issue', at: issue.createdAt }];
	const activity_count = between(rng, 5, 40);
	for (let i = 1; i < activity_count; i++) {
		const kind = rng();
		if (kind < 0.4) {
			const status = pick(rng, STATUSES);
			activity.push({ author: pick(rng, PEOPLE).id, text: `changed status to ${status.name}`, at: at(), status: status.id });
		} else if (kind < 0.65) {
			activity.push({ author: pick(rng, PEOPLE).id, text: `set priority to ${pick(rng, PRIORITIES).name}`, at: at() });
		} else if (kind < 0.85) {
			activity.push({ author: pick(rng, PEOPLE).id, text: `assigned ${pick(rng, PEOPLE).name}`, at: at() });
		} else {
			activity.push({ author: pick(rng, PEOPLE).id, text: `added the label ${pick(rng, LABELS).name}`, at: at() });
		}
	}
	activity.sort((a, b) => a.at - b.at);

	found = { description: description.join(' '), comments, activity };
	details_cache.set(issue.key, found);
	return found;
}

export const status_by_id = Object.fromEntries(STATUSES.map((s) => [s.id, s])) as Record<StatusId, Status>;
export const priority_by_id = Object.fromEntries(PRIORITIES.map((p) => [p.id, p])) as Record<PriorityId, Priority>;
export const person_by_id: Record<string, Person> = Object.fromEntries(PEOPLE.map((p) => [p.id, p]));
export const project_by_key: Record<string, Project> = Object.fromEntries(PROJECTS.map((p) => [p.key, p]));
export const label_by_id: Record<string, Label> = Object.fromEntries(LABELS.map((l) => [l.id, l]));
