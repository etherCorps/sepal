<!--
	⌘K (docs/tracker-design.md, Command palette): a graphite Dialog with an
	input and its own list, not Bits UI's Command. Searches actions, projects
	and the top 8 issues across all N on every keystroke; `>` limits it to
	actions. Arrows move, Enter runs, Escape closes (Bits UI), and Bits UI
	gives the focus back to where it was.
-->
<script lang="ts">
	import type { Component } from 'svelte';
	import { goto } from '$app/navigation';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import LayoutListIcon from '@lucide/svelte/icons/layout-list';
	import CircleUserIcon from '@lucide/svelte/icons/circle-user-round';
	import KanbanIcon from '@lucide/svelte/icons/square-kanban';
	import ChartIcon from '@lucide/svelte/icons/chart-column';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import KeyboardIcon from '@lucide/svelte/icons/keyboard';
	import UndoIcon from '@lucide/svelte/icons/undo-2';
	import RowsIcon from '@lucide/svelte/icons/rows-3';
	import { Dialog, DialogContent, DialogTitle } from '#lib/components/ui/dialog/index.ts';
	import { ProjectSquare, StatusIcon } from '#lib/glyphs/index.ts';
	import { PROJECTS, type Issue, type StatusId } from '#lib/seed.ts';
	import { data, history, settings, store_settings, undo } from '#lib/store.svelte.ts';
	import { count } from '#lib/format.ts';
	import { key_label, push, ui } from '#lib/ui.svelte.ts';

	const ISSUE_LIMIT = 8;

	interface Item {
		id: string;
		group: 'Actions' | 'Projects' | 'Issues';
		label: string;
		icon?: Component<any>;
		project?: string;
		status?: StatusId;
		/** a key hint (`Mod+k` notation) or a mono value at the right */
		keys?: string;
		meta?: string;
		run: () => void;
		/** opens another Dialog: wait until this one is gone, or its focus return lands in the new one */
		after_close?: boolean;
	}

	const actions = $derived<Item[]>([
		{ id: 'new', group: 'Actions', label: 'Create new issue', icon: PlusIcon, keys: 'c', run: () => (ui.new_issue = true), after_close: true },
		{ id: 'issues', group: 'Actions', label: 'Go to Issues', icon: LayoutListIcon, keys: 'g i', run: () => goto('/issues') },
		{ id: 'my', group: 'Actions', label: 'Go to My issues', icon: CircleUserIcon, keys: 'g m', run: () => goto('/my') },
		{ id: 'board', group: 'Actions', label: 'Go to Board', icon: KanbanIcon, keys: 'g b', run: () => goto('/board') },
		{ id: 'insights', group: 'Actions', label: 'Go to Insights', icon: ChartIcon, keys: 'g n', run: () => goto('/insights') },
		{ id: 'settings', group: 'Actions', label: 'Go to Settings', icon: SettingsIcon, keys: 'g s', run: () => goto('/settings') },
		{ id: 'shortcuts', group: 'Actions', label: 'Keyboard shortcuts', icon: KeyboardIcon, keys: '?', run: () => (ui.shortcuts = true), after_close: true },
		{
			id: 'compact',
			group: 'Actions',
			label: settings.compact ? 'Use comfortable rows' : 'Use compact rows',
			icon: RowsIcon,
			run: () => {
				settings.compact = !settings.compact;
				store_settings().catch(() => {});
			}
		},
		...(history.size > 0
			? [
					{
						id: 'undo',
						group: 'Actions' as const,
						label: `Undo: ${history.last}`,
						icon: UndoIcon,
						run: () => {
							const label = undo();
							if (label) push({ text: `Undid: ${label}` });
						}
					}
				]
			: [])
	]);

	let query = $state('');
	let input = $state<HTMLInputElement | null>(null);

	/** Lower-cased "key title" per issue object; objects are replaced on edit, so this never goes stale. */
	const haystacks = new WeakMap<Issue, string>();
	function haystack(issue: Issue): string {
		let text = haystacks.get(issue);
		if (text === undefined) haystacks.set(issue, (text = `${issue.key} ${issue.title}`.toLowerCase()));
		return text;
	}

	/** Exact key first, then keys that start with the query, then any match; newest first within each. */
	function search_issues(q: string): Issue[] {
		const exact: Issue[] = [];
		const prefix: Issue[] = [];
		const rest: Issue[] = [];
		for (const issue of data.issues) {
			const text = haystack(issue);
			if (!text.includes(q)) continue;
			if (text.startsWith(q + ' ')) exact.push(issue);
			else if (text.startsWith(q)) prefix.length < ISSUE_LIMIT && prefix.push(issue);
			else if (rest.length < ISSUE_LIMIT) rest.push(issue);
			// ponytail: keeps scanning after 8 for an exact key further down; one pass over N either way.
		}
		return [...exact, ...prefix, ...rest].slice(0, ISSUE_LIMIT);
	}

	const items = $derived.by((): Item[] => {
		const raw = query.trim();
		const actions_only = raw.startsWith('>');
		const q = (actions_only ? raw.slice(1) : raw).trim().toLowerCase();
		const out = actions.filter((a) => a.label.toLowerCase().includes(q));
		if (actions_only) return out;
		for (const project of PROJECTS) {
			if (!project.name.toLowerCase().includes(q) && !project.key.toLowerCase().startsWith(q)) continue;
			out.push({
				id: `project-${project.key}`,
				group: 'Projects',
				label: project.name,
				project: project.key,
				meta: `${count(data.open_by_project[project.key] ?? 0)} open`,
				run: () => goto(`/projects/${project.key}`)
			});
		}
		if (q) {
			for (const issue of search_issues(q)) {
				out.push({
					id: issue.key,
					group: 'Issues',
					label: issue.title,
					status: issue.status,
					meta: issue.key,
					run: () => {
						ui.highlighted = issue.key;
						goto(`/issues/${issue.key}`);
					}
				});
			}
		}
		return out;
	});

	// Back to the first item whenever the list changes.
	let active = $derived(items.length > 0 ? 0 : -1);

	/** Set when the chosen item opens another Dialog; run once this one has closed. */
	let pending: (() => void) | null = null;

	function run(index: number) {
		const item = items[index];
		if (!item) return;
		ui.palette = false;
		if (item.after_close) pending = item.run;
		else item.run();
	}

	function onkeydown(event: KeyboardEvent) {
		const n = items.length;
		if (event.key === 'ArrowDown' && n > 0) active = (active + 1) % n;
		else if (event.key === 'ArrowUp' && n > 0) active = (active - 1 + n) % n;
		else if (event.key === 'Enter') run(active);
		else return;
		event.preventDefault();
	}
</script>

<Dialog
	bind:open={ui.palette}
	onOpenChangeComplete={(open) => {
		if (open || !pending) return;
		const next = pending;
		pending = null;
		next();
	}}
>
	<DialogContent
		data-testid="palette"
		showCloseButton={false}
		class="bg-graphite text-graphite-foreground ring-graphite-2 top-[18vh] right-0 left-0 mx-auto translate-x-0 translate-y-0 gap-0 overflow-hidden rounded-lg p-0 shadow-sm sm:max-w-[560px]"
		onOpenAutoFocus={(event) => {
			event.preventDefault();
			query = '';
			pending = null;
			input?.focus();
		}}
	>
		<DialogTitle class="sr-only">Command palette</DialogTitle>
		<div class="relative shrink-0">
			<input
				data-testid="palette-input"
				bind:this={input}
				class="text-graphite-foreground border-graphite-2 h-12 w-full border-b bg-transparent px-4 text-[15px] outline-none"
				aria-label="Type a command or search issues"
				bind:value={query}
				{onkeydown}
			/>
			<!-- Blitz draws no placeholder text. -->
			{#if !query}
				<span class="text-graphite-muted pointer-events-none absolute top-0 left-4 flex h-12 items-center text-[15px]"
					>Type a command or search issues…</span
				>
			{/if}
		</div>
		<div class="flex max-h-[360px] flex-col overflow-y-scroll p-1.5">
			{#each items as item, index (item.id)}
				{#if index === 0 || items[index - 1].group !== item.group}
					<div
						class="text-graphite-muted flex h-7 shrink-0 items-end px-2.5 pb-1 font-mono text-[11px] font-medium tracking-[0.06em] uppercase"
					>
						{item.group}
					</div>
				{/if}
				<button
					data-testid="palette-item"
					data-active={index === active ? '' : undefined}
					tabindex="-1"
					class={[
						'flex h-9 shrink-0 items-center gap-2.5 rounded-md px-2.5 text-left text-[13px]',
						index === active ? 'bg-graphite-2 text-graphite-foreground' : 'text-graphite-foreground/85'
					]}
					onpointermove={() => (active = index)}
					onclick={() => run(index)}
				>
					<span class={['flex size-4 shrink-0 items-center justify-center', index === active ? 'text-primary' : 'text-graphite-muted']}>
						{#if item.icon}
							<item.icon class="size-4" strokeWidth={1.75} />
						{:else if item.project}
							<ProjectSquare project={item.project} />
						{:else if item.status}
							<StatusIcon status={item.status} />
						{/if}
					</span>
					<span class="min-w-0 flex-1 truncate">{item.label}</span>
					{#if item.meta}
						<span class="text-graphite-muted shrink-0 font-mono text-[11px]">{item.meta}</span>
					{:else if item.keys}
						<kbd class="border-graphite-2 text-graphite-muted shrink-0 rounded border px-1.5 font-mono text-[11px] leading-[18px]"
							>{key_label(item.keys)}</kbd
						>
					{/if}
				</button>
			{:else}
				<div class="text-graphite-muted px-2.5 py-6 text-center text-[13px]">No results for ‘{query.trim()}’.</div>
			{/each}
		</div>
		<div
			class="text-graphite-muted border-graphite-2 flex h-8 shrink-0 items-center gap-3 border-t px-4 font-mono text-[11px]"
		>
			<span>↑↓ move</span><span>·</span><span>enter open</span><span>·</span><span>esc close</span><span>·</span><span>&gt; actions</span>
		</div>
	</DialogContent>
</Dialog>
