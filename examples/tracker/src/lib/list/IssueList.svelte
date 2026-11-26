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
	import PageHeader from '#lib/components/PageHeader.svelte';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { count, short_date } from '#lib/format.ts';
	import { Avatar, PriorityIcon, ProjectSquare, StatusIcon } from '#lib/glyphs/index.ts';
	import type { PriorityId, StatusId } from '#lib/seed.ts';
	import { PAGE_SIZE, clear_filters, data, neighbor, set_scope, settings, show_all, show_more, toggle_group, view, week_range, type GroupBy, type PagedGroup, type SortBy } from '#lib/store.svelte.ts';
	import { checked, ui, use_keys } from '#lib/ui.svelte.ts';
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

	// --- the window ----------------------------------------------------------
	// Only the rows in view are mounted, plus OVERSCAN px either side; spacers
	// stand in for the rest. Rows and headers have fixed heights, so where
	// anything is follows from counts, and there are at most a dozen groups.

	const HEADER = 32;
	/** The "Show 200 more" row under the list (h-9), when there is more to page in. */
	const MORE = 36;
	const OVERSCAN = 300;
	const row_height = $derived(settings.compact ? 28 : 36);
	const header_height = $derived(view.groupBy === 'none' ? 0 : HEADER);
	let list: HTMLElement | undefined;
	let scroll_top = $state(0);
	// A guess until the ResizeObserver has the list's height.
	let viewport = $state(1000);

	function track(node: HTMLElement) {
		list = node;
		const observer = new ResizeObserver(([entry]) => (viewport = entry.contentRect.height));
		observer.observe(node);
		return () => {
			observer.disconnect();
			list = undefined;
		};
	}

	const windowed = $derived.by(() => {
		const header = header_height;
		const row = row_height;
		let height = 0;
		for (const group of paged.groups) height += header + group.issues.length * row;
		// What the box scrolls over ends below the "Show more" row, not the last row.
		const end_of_list = height + (paged.hidden > 0 ? MORE : 0);
		const top = Math.min(scroll_top, Math.max(0, end_of_list - viewport));
		const from = top - OVERSCAN;
		const to = top + viewport + OVERSCAN;
		const groups: (PagedGroup & { header: boolean })[] = [];
		let y = 0;
		let start = -1;
		let end = 0;
		for (const group of paged.groups) {
			const head = y;
			const rows = head + header;
			y = rows + group.issues.length * row;
			if (y <= from || head >= to) continue;
			const with_header = header > 0 && rows > from;
			const first = Math.max(0, Math.floor((from - rows) / row));
			const last = Math.min(group.issues.length, Math.ceil((to - rows) / row));
			const issues = first < last ? group.issues.slice(first, last) : [];
			if (!with_header && issues.length === 0) continue;
			if (start === -1) start = with_header ? head : rows + first * row;
			end = issues.length > 0 ? rows + last * row : rows;
			groups.push({ ...group, header: with_header, issues });
		}
		return { groups, above: Math.max(0, start), below: height - end, end_of_list };
	});

	// Blitz keeps a scroll offset past the end when the content shrinks (a
	// filter, a collapsed group), and the list would show nothing. Pull it
	// back, as a browser does.
	$effect(() => {
		const end = Math.max(0, windowed.end_of_list - viewport);
		if (list !== undefined && scroll_top > end) list.scrollTop = end;
	});

	/** Scroll the list so the row of `key` is in view, as `scrollIntoView({ block: 'nearest' })`; it may not be mounted. */
	function reveal(key: string) {
		if (list === undefined) return;
		let y = 0;
		for (const group of paged.groups) {
			y += header_height;
			const at = group.issues.findIndex((issue) => issue.key === key);
			if (at !== -1) {
				y += at * row_height;
				// The first row of a group brings its header along.
				const top = at === 0 ? y - header_height : y;
				if (top < scroll_top) list.scrollTop = top;
				else if (y + row_height > scroll_top + viewport) list.scrollTop = y + row_height - viewport;
				scroll_top = list.scrollTop;
				return;
			}
			y += group.issues.length * row_height;
		}
	}

	function clear() {
		const had_week = view.week !== null;
		clear_filters();
		if (had_week) void goto('/issues');
	}

	// --- selection ----------------------------------------------------------

	/** Check every paged-in row from the anchor to `key`, inclusive, mounted or not. */
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
		reveal(next);
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

	<!--
		One 44px line where there is room. With the pane open Group and Sort step aside and
		the search starts small. In a narrow window it wraps onto another line rather than
		run past the list: every control stays where it can be seen and clicked.
	-->
	<div class="flex min-h-11 shrink-0 flex-wrap items-center gap-2 border-b px-4 py-[7.5px]">
		<Filters {scope} compact={pane_open} />
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

	<div data-testid="list" class="min-h-0 flex-1 overflow-y-scroll" {@attach track} onscroll={() => (scroll_top = list?.scrollTop ?? 0)}>
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
			<div style="height: {windowed.above}px"></div>
			{#each windowed.groups as group (group.id)}
				{#if group.header}
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
			<div style="height: {windowed.below}px"></div>
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
