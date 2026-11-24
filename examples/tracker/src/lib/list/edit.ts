// Edits made from the list, the bulk bar and the detail pane: change the
// store, then say what happened in a toast with Undo.

import { LABELS, PEOPLE, PRIORITIES, PROJECTS, STATUSES, label_by_id, priority_by_id, project_by_key, status_by_id } from '#lib/seed.ts';
import { bulk_update, delete_issues, person, undo_action, view, type Patch } from '#lib/store.svelte.ts';
import { push } from '#lib/ui.svelte.ts';
import { plural } from '#lib/format.ts';

export interface Item {
	value: string;
	label: string;
}

export const status_items: Item[] = STATUSES.map((s) => ({ value: s.id, label: s.name }));
export const priority_items: Item[] = PRIORITIES.map((p) => ({ value: p.id, label: p.name }));
export const project_items: Item[] = PROJECTS.map((p) => ({ value: p.key, label: p.name }));
export const label_items: Item[] = LABELS.map((l) => ({ value: l.id, label: l.name }));
export const estimate_items: Item[] = [
	{ value: 'none', label: 'No estimate' },
	...[1, 2, 3, 5, 8].map((n) => ({ value: String(n), label: plural(n, 'point') }))
];

/** People with the user's display name; call it in a `$derived` so a rename shows. */
export function people_items(): Item[] {
	return PEOPLE.map((p) => ({ value: p.id, label: person(p.id)!.name }));
}

function what(patch: Patch, who: string): string {
	if (patch.status !== undefined) return `Moved ${who} to ${status_by_id[patch.status].name}`;
	if (patch.priority !== undefined) return `Set ${who} to ${priority_by_id[patch.priority].name.toLowerCase()} priority`;
	if (patch.assignee !== undefined) return patch.assignee ? `Assigned ${who} to ${person(patch.assignee)?.name}` : `Unassigned ${who}`;
	if (patch.project !== undefined) return `Moved ${who} to ${project_by_key[patch.project]?.name}`;
	if (patch.labelIds !== undefined)
		return patch.labelIds.length ? `Labelled ${who} ${patch.labelIds.map((id) => label_by_id[id]?.name).join(', ')}` : `Removed the labels from ${who}`;
	if (patch.estimate !== undefined) return patch.estimate === null ? `Removed the estimate from ${who}` : `Estimated ${who} at ${plural(patch.estimate, 'point')}`;
	return `Renamed ${who}`;
}

/** Change some issues as one edit, and offer Undo. */
export function edit(keys: string[], patch: Patch): void {
	const changed = bulk_update(keys, patch);
	if (changed.length === 0) return;
	const who = changed.length === 1 ? changed[0].key : plural(changed.length, 'issue');
	push({ text: what(patch, who), status: patch.status, action: undo_action() });
}

export function remove(keys: string[]): number {
	const gone = delete_issues(keys);
	if (gone > 0) push({ text: `Deleted ${gone === 1 ? keys[0] : plural(gone, 'issue')}`, action: undo_action() });
	return gone;
}

/** Where closing the pane goes: the list, with the week from Insights if one is set. */
export function list_href(): string {
	return view.week === null ? '/issues' : `/issues?week=${view.week}`;
}
