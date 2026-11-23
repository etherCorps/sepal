<!--
	Search and the four filter Selects, bound to the store's shared `view`:
	the list's toolbar and the board's. Filters the route fixes (`scope`) are
	left out. Renders no wrapper; the caller's toolbar lays the parts out.
-->
<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import Kbd from '$lib/components/Kbd.svelte';
	import { Input } from '$lib/components/ui/input/index.js';
	import type { PriorityId, StatusId } from '$lib/seed';
	import { view } from '$lib/store.svelte';
	import Pick from './Pick.svelte';
	import { people_items, priority_items, project_items, status_items } from './edit';

	// compact: the list beside the detail pane (480px at the 1200px window) has no room
	// for four filters; Assignee and Project stay only while they filter something.
	let { scope = {}, compact = false }: { scope?: { assignee?: string; project?: string }; compact?: boolean } = $props();

	// Bits UI treats '' as "nothing chosen"; the store uses '' for "all".
	const ALL = 'all';
	const status_filter = [{ value: ALL, label: 'All statuses' }, ...status_items];
	const priority_filter = [{ value: ALL, label: 'All priorities' }, ...priority_items];
	const assignee_filter = $derived([{ value: ALL, label: 'Anyone' }, { value: 'none', label: 'Unassigned' }, ...people_items()]);
	const project_filter = [{ value: ALL, label: 'All projects' }, ...project_items];
	const unset = (v: string) => (v === ALL ? '' : v);

	function search_keys(event: KeyboardEvent) {
		// Escape leaves the search, so j/k work on the results.
		if (event.key === 'Escape') {
			event.preventDefault();
			(event.currentTarget as HTMLElement).blur();
		}
	}
</script>

<div class="relative flex w-[260px] min-w-[160px] shrink">
	<SearchIcon class="text-muted-foreground pointer-events-none absolute top-1.5 left-2 size-4" strokeWidth={1.75} />
	<Input
		data-testid="search"
		class="h-7 pr-7 pl-7 text-[14px] md:text-[14px]"
		placeholder="Search issues…"
		bind:value={view.search}
		onkeydown={search_keys}
	/>
	{#if !view.search}
		<!-- Blitz draws no placeholder text. -->
		<span class="text-muted-foreground pointer-events-none absolute top-[5px] left-7 text-[14px] leading-[18px]">Search issues…</span>
	{/if}
	<span class="pointer-events-none absolute top-1 right-1.5"><Kbd>/</Kbd></span>
</div>
<Pick
	testid="filter-status"
	kind="status"
	placeholder="Status"
	value={view.status}
	items={status_filter}
	dot={view.status !== ''}
	onpick={(v) => (view.status = unset(v) as StatusId | '')}
/>
<Pick
	testid="filter-priority"
	kind="priority"
	placeholder="Priority"
	value={view.priority}
	items={priority_filter}
	dot={view.priority !== ''}
	onpick={(v) => (view.priority = unset(v) as PriorityId | '')}
/>
{#if !scope.assignee && (!compact || view.assignee !== '')}
	<Pick
		testid="filter-assignee"
		kind="assignee"
		placeholder="Assignee"
		value={view.assignee}
		items={assignee_filter}
		dot={view.assignee !== ''}
		onpick={(v) => (view.assignee = unset(v))}
	/>
{/if}
{#if !scope.project && (!compact || view.project !== '')}
	<Pick
		testid="filter-project"
		kind="project"
		placeholder="Project"
		value={view.project}
		items={project_filter}
		dot={view.project !== ''}
		onpick={(v) => (view.project = unset(v))}
	/>
{/if}
