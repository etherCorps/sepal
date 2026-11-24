import { error } from '@sveltejs/kit';
import { get_issue } from '#lib/store.svelte.ts';

// Runs on every navigation (j/k with the pane open): keep it a lookup.
export function load({ params }: { params: { key: string } }) {
	if (!get_issue(params.key)) error(404, `No issue ${params.key}`);
	return { key: params.key };
}
