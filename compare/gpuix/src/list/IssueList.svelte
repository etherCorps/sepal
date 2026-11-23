<!--
	The one list behind /issues, /my and /projects/:key (examples/tracker/src/lib/list/IssueList.svelte).
	`scope` is what the route fixes. The window is the Tracker's, unchanged: fixed row and header
	heights, spacers for what is not mounted, the visible slice from the scroll offset plus 300 px
	either side. The list's height comes from `onlayout` (the Tracker's ResizeObserver) and the
	offset from `onscrollchange` (its `onscroll`); `scrollTo` stands in for setting `scrollTop`.
-->
<script lang="ts">
	import { untrack } from 'svelte';
	import { get_native, type GpuixEvent, type ShadowNode } from 'gpuix-svelte';
	import { navigate, router } from 'gpuix-svelte-ui';
	import Avatar from '../components/Avatar.svelte';
	import Btn from '../components/Btn.svelte';
	import Icon from '../components/Icon.svelte';
	import PageHeader from '../components/PageHeader.svelte';
	import Pick from '../components/Pick.svelte';
	import PriorityIcon from '../components/PriorityIcon.svelte';
	import ProjectSquare from '../components/ProjectSquare.svelte';
	import StatusIcon from '../components/StatusIcon.svelte';
	import { list_href } from '../edit.ts';
	import { count, short_date } from '../format.ts';
	import type { PriorityId, StatusId } from '../seed.ts';
	import { PAGE_SIZE, clear_filters, data, neighbor, set_scope, settings, show_all, show_more, toggle_group, view, week_range, type GroupBy, type PagedGroup, type SortBy } from '../store.svelte.ts';
	import { checked, ui, use_keys } from '../ui.svelte.ts';
	import BulkBar from './BulkBar.svelte';
	import Filters from './Filters.svelte';
	import Row, { type Press } from './Row.svelte';

	let { title, scope = {} }: { title: string; scope?: { assignee?: string; project?: string } } = $props();

	$effect.pre(() => {
		const next = { assignee: scope.assignee, project: scope.project };
		untrack(() => {
			set_scope(next);
			checked.clear();
		});
	});

	// /issues?week=N (a bar in Insights). Opening an issue drops the query, so keep the week while the pane is open.
	const week_param = $derived(router.query.week ?? null);
	$effect.pre(() => {
		if (week_param !== null && Number.isFinite(Number(week_param))) view.week = Number(week_param);
		else if (!router.path.startsWith('/issues/')) view.week = null;
	});

	const pane_open = $derived(router.path.startsWith('/issues/'));
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

	const HEADER = 32;
	const MORE = 36;
	const OVERSCAN = 300;
	const row_height = $derived(settings.compact ? 28 : 36);
	const header_height = $derived(view.groupBy === 'none' ? 0 : HEADER);
	let list: ShadowNode | undefined;
	let scroll_top = $state(0);
	// A guess until onlayout has the list's height.
	let viewport = $state(1000);

	function set_scroll(top: number) {
		if (list?.nativeId == null) return;
		get_native()?.scrollTo(list.nativeId, 0, -top);
		scroll_top = top;
	}

	const windowed = $derived.by(() => {
		const header = header_height;
		const row = row_height;
		let height = 0;
		for (const group of paged.groups) height += header + group.issues.length * row;
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

	// Pull an offset past the end back when the content shrinks, as the Tracker does for Blitz.
	$effect(() => {
		const end = Math.max(0, windowed.end_of_list - viewport);
		if (list !== undefined && scroll_top > end) set_scroll(end);
	});

	/** Scroll the list so the row of `key` is in view, as `scrollIntoView({ block: 'nearest' })`. */
	function reveal(key: string) {
		if (list === undefined) return;
		let y = 0;
		for (const group of paged.groups) {
			y += header_height;
			const at = group.issues.findIndex((issue) => issue.key === key);
			if (at !== -1) {
				y += at * row_height;
				const top = at === 0 ? y - header_height : y;
				if (top < scroll_top) set_scroll(top);
				else if (y + row_height > scroll_top + viewport) set_scroll(y + row_height - viewport);
				return;
			}
			y += group.issues.length * row_height;
		}
	}

	function clear() {
		const had_week = view.week !== null;
		clear_filters();
		if (had_week) void navigate('/issues');
	}

	// --- selection ----------------------------------------------------------

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

	function on_check(press: Press, key: string) {
		if (press.shift && check_range(key)) {
			ui.anchor = key;
			return;
		}
		toggle(key);
	}

	function on_open(press: Press, key: string) {
		if (press.shift) {
			if (!check_range(key)) toggle(key);
			ui.anchor = key;
			return false;
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
		if (pane_open) void navigate(`/issues/${next}`);
	}
	function open() {
		if (ui.highlighted === null) return false;
		void navigate(`/issues/${ui.highlighted}`);
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
			if (pane_open) void navigate(list_href());
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

<div class="root">
	<PageHeader {title} count={shown === total ? count(total) : `${count(shown)} of ${count(total)}`} />

	<div class="toolbar">
		<Filters {scope} compact={pane_open} />
		{#if view.week !== null}
			{@const [from] = week_range(view.week)}
			<button class="week" testid="filter-week" hitbox="self" onclick={() => void navigate('/issues')}>
				<text class="week-label">Week</text><text>{short_date(from)}</text>
				<Icon name="x" size={14} />
			</button>
		{/if}
		{#if !pane_open}
			<div class="end">
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

	<div
		class="list"
		testid="list"
		{@attach (node: ShadowNode) => {
			list = node;
			return () => (list = undefined);
		}}
		onlayout={(e: GpuixEvent) => (viewport = e.height ?? viewport)}
		onscrollchange={(e: GpuixEvent) => (scroll_top = Math.max(0, -(e.scrollY ?? 0)))}
	>
		{#if shown === 0}
			<div class="empty" testid="empty">
				<text class="empty-text">
					{view.search.trim() ? `No issues match ‘${view.search.trim()}’.` : 'No issues match these filters.'}
				</text>
				{#if filtering}
					<Btn testid="clear-filters" variant="ghost" size="sm" onclick={clear}>Clear filters</Btn>
				{/if}
			</div>
		{:else}
			<div class="spacer" style="height: {windowed.above}px"></div>
			{#each windowed.groups as group (group.id)}
				{#if group.header}
					<button class="group" testid="group-header" hitbox="self" onclick={() => toggle_group(group.id)}>
						<Icon name={group.collapsed ? 'chevron-right' : 'chevron-down'} size={14} />
						{@render group_glyph(view.groupBy, group.id)}
						<text class="group-name">{group.name}</text>
						<text class="group-count">{count(group.total)}</text>
					</button>
				{/if}
				{#each group.issues as issue (issue.key)}
					<Row {issue} narrow={pane_open} oncheck={(press) => on_check(press, issue.key)} onopen={(press) => on_open(press, issue.key)} />
				{/each}
			{/each}
			<div class="spacer" style="height: {windowed.below}px"></div>
			{#if paged.hidden > 0}
				<div class="more">
					<Btn testid="show-more" variant="ghost" size="sm" style="height: 28px" onclick={show_more}>
						Show {count(Math.min(PAGE_SIZE, paged.hidden))} more
					</Btn>
					<Btn testid="show-all" variant="ghost" size="sm" style="height: 28px" onclick={show_all}>
						Show all {count(paged.shown + paged.hidden)}
					</Btn>
					<text class="hidden-count">{count(paged.hidden)} hidden</text>
				</div>
			{/if}
		{/if}
	</div>

	{#if checked.size > 0}
		<BulkBar />
	{/if}
</div>

<style>
	.root { position: relative; display: flex; flex-direction: column; flex-grow: 1; flex-basis: 0; min-height: 0; min-width: 0; }
	.toolbar {
		display: flex;
		flex-direction: row;
		flex-wrap: nowrap;
		align-items: center;
		flex-shrink: 0;
		gap: 8px;
		height: 44px;
		padding: 0 16px;
		overflow: hidden;
		border-width: 0 0 1px 0;
		border-style: solid;
		border-color: var(--border);
	}
	.week {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-shrink: 0;
		gap: 6px;
		height: 28px;
		padding: 0 8px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
		font-size: 13px;
		color: var(--foreground);
		cursor: pointer;
	}
	.week:hover { background-color: var(--accent); }
	.week-label { font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.end { display: flex; flex-direction: row; align-items: center; justify-content: flex-end; flex-grow: 1; flex-shrink: 0; gap: 4px; }
	.list { display: flex; flex-direction: column; flex-grow: 1; flex-basis: 0; min-height: 0; overflow-y: scroll; scrollbar-color: var(--input); }
	.spacer { flex-shrink: 0; }
	.empty { display: flex; flex-direction: column; align-items: center; gap: 12px; padding-top: 120px; }
	.empty-text { font-size: 14px; font-weight: 500; color: var(--foreground); }
	.group {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-shrink: 0;
		gap: 8px;
		width: 100%;
		height: 32px;
		padding: 0 16px;
		border-width: 0 0 1px 0;
		border-style: solid;
		border-color: var(--border);
		background-color: var(--muted);
		cursor: pointer;
	}
	.group:hover { background-color: var(--accent); }
	.group-name { font-size: 12px; font-weight: 500; color: var(--foreground); }
	.group-count { font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.more {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-shrink: 0;
		gap: 4px;
		height: 36px;
		padding: 0 12px;
		border-width: 0 0 1px 0;
		border-style: solid;
		border-color: var(--border);
	}
	.hidden-count { flex-grow: 1; padding-right: 4px; text-align: right; font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
</style>
