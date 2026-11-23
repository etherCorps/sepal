<!--
	The one list behind /issues, /my and /projects/[key]
	(docs/tracker-design.md, Issues list). `scope` is what the route fixes.
	It also owns the list's keys, including j/k with the detail pane open:
	the pane only exists inside the issues layout, next to this list.
-->
<script lang="ts">
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import XIcon from '@lucide/svelte/icons/x';
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import PageHeader from '$lib/components/PageHeader.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { count, short_date } from '$lib/format';
	import { Avatar, PriorityIcon, ProjectSquare, StatusIcon } from '$lib/glyphs';
	import type { PriorityId, StatusId } from '$lib/seed';
	import { PAGE_SIZE, clear_filters, data, neighbor, set_scope, show_all, show_more, toggle_group, view, week_range, type GroupBy, type SortBy } from '$lib/store.svelte';
	import { checked, ui, use_keys } from '$lib/ui.svelte';
	import BulkBar from './BulkBar.svelte';
	import Filters from './Filters.svelte';
	import Pick from './Pick.svelte';
	import Row from './Row.svelte';
	import { list_href } from './edit';

	let { title, scope = {} }: { title: string; scope?: { assignee?: string; project?: string } } = $props();

	$effect.pre(() => {
		const next = { assignee: scope.assignee, project: scope.project };
		untrack(() => {
			set_scope(next);
			checked.clear();
		});
	});

	// /issues?week=N (a bar in Insights). Opening an issue drops the query, so keep the week while the pane is open.
	const week_param = $derived(page.url.searchParams.get('week'));
	$effect.pre(() => {
		if (week_param !== null && Number.isFinite(Number(week_param))) view.week = Number(week_param);
		else if (!page.url.pathname.startsWith('/issues/')) view.week = null;
	});

	const pane_open = $derived(page.url.pathname.startsWith('/issues/'));
	const paged = $derived(data.paged);
	const shown = $derived(data.filtered.length);
	const total = $derived(data.total);
	const filtering = $derived(
		view.search.trim() !== '' || view.status !== '' || view.priority !== '' || view.assignee !== '' || view.project !== '' || view.week !== null
	);

	const group_items = [
		{ value: 'status', label: 'Status' },
		{ value: 'assignee', label: 'Assignee' },
		{ value: 'priority', label: 'Priority' },
		{ value: 'project', label: 'Project' },
		{ value: 'none', label: 'None' }
	];
	const sort_items = [
		{ value: 'updated', label: 'Updated' },
		{ value: 'created', label: 'Created' },
		{ value: 'priority', label: 'Priority' },
		{ value: 'title', label: 'Title' }
	];

	function clear() {
		const had_week = view.week !== null;
		clear_filters();
		if (had_week) void goto('/issues');
	}

	// --- selection ----------------------------------------------------------

	/** Check every mounted row from the anchor to `key`, inclusive. */
	function check_range(key: string) {
		const rows = paged.rows;
		const from = rows.findIndex((issue) => issue.key === ui.anchor);
		const to = rows.findIndex((issue) => issue.key === key);
		if (from === -1 || to === -1) return false;
		for (let i = Math.min(from, to); i <= Math.max(from, to); i++) checked.add(rows[i].key);
		return true;
	}

	function toggle(key: string) {
		if (checked.has(key)) checked.delete(key);
		else checked.add(key);
		ui.anchor = key;
	}

	function on_check(event: MouseEvent, key: string) {
		const had = checked.has(key);
		if (event.shiftKey && check_range(key)) {
			// A range only adds, but the click would untick this box: keep it ticked.
			if (had) event.preventDefault();
			ui.anchor = key;
			return;
		}
		toggle(key);
	}

	function on_open(event: MouseEvent, key: string) {
		// The router leaves Shift+clicks alone; here they extend the selection.
		if (event.shiftKey) {
			event.preventDefault();
			if (!check_range(key)) toggle(key);
			ui.anchor = key;
			return;
		}
		ui.highlighted = key;
		ui.anchor = key;
	}

	// --- keys ---------------------------------------------------------------

	function move(delta: number) {
		const next = neighbor(ui.highlighted, delta);
		if (next === null) return;
		ui.highlighted = next;
		if (pane_open) void goto(`/issues/${next}`);
	}
	function open() {
		if (ui.highlighted === null) return false;
		void goto(`/issues/${ui.highlighted}`);
	}

	use_keys({
		j: () => move(1),
		k: () => move(-1),
		Enter: open,
		o: open,
		x: () => {
			if (ui.highlighted === null) return false;
			toggle(ui.highlighted);
		},
		Escape: () => {
			if (pane_open) void goto(list_href());
			else if (checked.size > 0) checked.clear();
			else return false;
		}
	});
</script>

{#snippet group_glyph(by: GroupBy, id: string)}
	{#if by === 'status'}
		<StatusIcon status={id as StatusId} />
	{:else if by === 'priority'}
		<PriorityIcon priority={id as PriorityId} />
	{:else if by === 'assignee'}
		<Avatar id={id === 'none' ? null : id} />
	{:else if by === 'project'}
		<ProjectSquare project={id} />
	{/if}
{/snippet}

<div class="relative flex min-h-0 min-w-0 flex-1 flex-col">
	<PageHeader {title} count={shown === total ? count(total) : `${count(shown)} of ${count(total)}`} />

	<!-- One 44px line. With the pane open the list is 560px: Group and Sort step aside, the search shrinks. -->
	<div class="flex h-11 shrink-0 flex-nowrap items-center gap-2 border-b px-4">
		<Filters {scope} />
		{#if view.week !== null}
			{@const [from] = week_range(view.week)}
			<button
				data-testid="filter-week"
				class="border-border text-foreground hover:bg-accent flex h-7 shrink-0 items-center gap-1.5 rounded-md border px-2 text-[13px] transition-colors"
				onclick={() => void goto('/issues')}
			>
				<span class="text-muted-foreground font-mono text-[11px]">Week</span>{short_date(from)}
				<XIcon class="text-muted-foreground size-3.5" strokeWidth={1.75} />
			</button>
		{/if}
		{#if !pane_open}
			<div class="ml-auto flex shrink-0 items-center gap-1">
				<Pick
					testid="group-by"
					variant="ghost"
					prefix="Group"
					value={view.groupBy}
					items={group_items}
					onpick={(v) => {
						view.groupBy = v as GroupBy;
						view.visible = PAGE_SIZE;
					}}
				/>
				<Pick testid="sort-by" variant="ghost" prefix="Sort" value={view.sortBy} items={sort_items} onpick={(v) => (view.sortBy = v as SortBy)} />
			</div>
		{/if}
	</div>

	<div data-testid="list" class="min-h-0 flex-1 overflow-y-scroll">
		{#if shown === 0}
			<div data-testid="empty" class="flex flex-col items-center gap-3 pt-[120px]">
				<p class="text-foreground text-[14px] font-medium">
					{view.search.trim() ? `No issues match ‘${view.search.trim()}’.` : 'No issues match these filters.'}
				</p>
				{#if filtering}
					<Button data-testid="clear-filters" variant="ghost" size="sm" onclick={clear}>Clear filters</Button>
				{/if}
			</div>
		{:else}
			{#each paged.groups as group (group.id)}
				{#if view.groupBy !== 'none'}
					<button
						data-testid="group-header"
						data-group={group.id}
						class="bg-muted hover:bg-accent ring-ring flex h-8 w-full shrink-0 items-center gap-2 border-b px-4 text-left outline-none focus-visible:ring-2 focus-visible:ring-inset"
						aria-expanded={group.collapsed ? 'false' : 'true'}
						onclick={() => toggle_group(group.id)}
					>
						{#if group.collapsed}
							<ChevronRightIcon class="text-muted-foreground size-3.5" strokeWidth={1.75} />
						{:else}
							<ChevronDownIcon class="text-muted-foreground size-3.5" strokeWidth={1.75} />
						{/if}
						{@render group_glyph(view.groupBy, group.id)}
						<span class="text-foreground text-xs font-medium">{group.name}</span>
						<span class="text-muted-foreground font-mono text-[11px]">{count(group.total)}</span>
					</button>
				{/if}
				{#each group.issues as issue (issue.key)}
					<Row
						{issue}
						narrow={pane_open}
						oncheck={(event) => on_check(event, issue.key)}
						onopen={(event) => on_open(event, issue.key)}
					/>
				{/each}
			{/each}
			{#if paged.hidden > 0}
				<div class="flex h-9 items-center gap-1 border-b px-3">
					<Button data-testid="show-more" variant="ghost" size="sm" class="h-7" onclick={show_more}>
						Show {count(Math.min(PAGE_SIZE, paged.hidden))} more
					</Button>
					<Button data-testid="show-all" variant="ghost" size="sm" class="h-7" onclick={show_all}>
						Show all {count(paged.shown + paged.hidden)}
					</Button>
					<span class="text-muted-foreground ml-auto pr-1 font-mono text-[11px]">{count(paged.hidden)} hidden</span>
				</div>
			{/if}
		{/if}
	</div>

	{#if checked.size > 0}
		<BulkBar />
	{/if}
</div>
