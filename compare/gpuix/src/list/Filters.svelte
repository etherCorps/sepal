<!--
	Search and the four filter Selects, bound to the store's shared `view`: the list's toolbar and
	the board's (examples/tracker/src/lib/list/Filters.svelte). Renders no wrapper.
	GPUI's <input> draws its own placeholder, so the Tracker's placeholder overlay is not needed.
-->
<script lang="ts">
	import { blur, type GpuixEvent } from 'gpuix-svelte';
	import Icon from '../components/Icon.svelte';
	import Kbd from '../components/Kbd.svelte';
	import Pick from '../components/Pick.svelte';
	import { people_items, priority_items, project_items, status_items } from '../edit.ts';
	import type { PriorityId, StatusId } from '../seed.ts';
	import { view } from '../store.svelte.ts';
	import { ui } from '../ui.svelte.ts';

	// compact: the list beside the detail pane (480px at the 1200px window) has no room
	// for four filters; Assignee and Project stay only while they filter something.
	let { scope = {}, compact = false }: { scope?: { assignee?: string; project?: string }; compact?: boolean } = $props();

	const ALL = 'all';
	const status_filter = [{ value: ALL, label: 'All statuses' }, ...status_items];
	const priority_filter = [{ value: ALL, label: 'All priorities' }, ...priority_items];
	const assignee_filter = $derived([{ value: ALL, label: 'Anyone' }, { value: 'none', label: 'Unassigned' }, ...people_items()]);
	const project_filter = [{ value: ALL, label: 'All projects' }, ...project_items];
	const unset = (v: string) => (v === ALL ? '' : v);

	function search_keys(event: GpuixEvent) {
		// Escape leaves the search, so j/k work on the results.
		if (event.key === 'escape') {
			event.preventDefault();
			blur();
		}
	}
</script>

<div class="search">
	<div class="icon"><Icon name="search" /></div>
	<input
		class="field"
		testid="search"
		placeholder="Search issues…"
		value={view.search}
		onchange={(e) => (view.search = e.value)}
		onkeydown={search_keys}
		{@attach (node) => {
			ui.search = node;
			return () => (ui.search = null);
		}}
	/>
	<div class="kbd"><Kbd text="/" /></div>
</div>
<Pick testid="filter-status" placeholder="Status" value={view.status} items={status_filter} dot={view.status !== ''} onpick={(v) => (view.status = unset(v) as StatusId | '')} />
<Pick testid="filter-priority" placeholder="Priority" value={view.priority} items={priority_filter} dot={view.priority !== ''} onpick={(v) => (view.priority = unset(v) as PriorityId | '')} />
{#if !scope.assignee && (!compact || view.assignee !== '')}
	<Pick testid="filter-assignee" placeholder="Assignee" value={view.assignee} items={assignee_filter} dot={view.assignee !== ''} onpick={(v) => (view.assignee = unset(v))} />
{/if}
{#if !scope.project && (!compact || view.project !== '')}
	<Pick testid="filter-project" placeholder="Project" value={view.project} items={project_filter} dot={view.project !== ''} onpick={(v) => (view.project = unset(v))} />
{/if}

<style>
	.search { position: relative; display: flex; flex-direction: row; width: 260px; min-width: 160px; flex-shrink: 1; }
	.icon { position: absolute; top: 6px; left: 8px; pointer-events: none; }
	.field {
		flex-grow: 1;
		height: 28px;
		padding: 0 28px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--input);
		font-size: 14px;
		color: var(--foreground);
	}
	.field:focus-visible { border-color: var(--ring); }
	.kbd { position: absolute; top: 4px; right: 6px; pointer-events: none; }
</style>
