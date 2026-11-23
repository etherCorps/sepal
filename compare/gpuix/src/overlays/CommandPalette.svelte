<!--
	⌘K (examples/tracker/src/lib/overlays/CommandPalette.svelte): a graphite panel with an input and
	its own list. Built on gpuix-svelte's Portal and dismiss layer, the pieces gpuix-svelte-ui's
	Dialog is made of, because that Dialog's panel (light, with a title row and close button)
	cannot take the palette's look. Same items, same search, same keys; no shadow, and the panel
	sits 18% from the top by a fixed 144 px (800 × 0.18), since `top` takes pixels only.
-->
<script lang="ts">
	import Portal from 'gpuix-svelte/components/Portal.svelte';
	import { dismiss_layer, dismiss_scope, type GpuixEvent } from 'gpuix-svelte';
	import { navigate } from 'gpuix-svelte-ui';
	import Icon from '../components/Icon.svelte';
	import ProjectSquare from '../components/ProjectSquare.svelte';
	import StatusIcon from '../components/StatusIcon.svelte';
	import { count } from '../format.ts';
	import type { IconName } from '../icons.ts';
	import { PROJECTS, type Issue, type StatusId } from '../seed.ts';
	import { data, history, settings, store_settings, undo } from '../store.svelte.ts';
	import { key_label, push, ui } from '../ui.svelte.ts';

	const ISSUE_LIMIT = 8;

	interface Item {
		id: string;
		group: 'Actions' | 'Projects' | 'Issues';
		label: string;
		icon?: IconName;
		project?: string;
		status?: StatusId;
		keys?: string;
		meta?: string;
		run: () => void;
	}

	const actions = $derived<Item[]>([
		{ id: 'new', group: 'Actions', label: 'Create new issue', icon: 'plus', keys: 'c', run: () => (ui.new_issue = true) },
		{ id: 'issues', group: 'Actions', label: 'Go to Issues', icon: 'layout-list', keys: 'g i', run: () => navigate('/issues') },
		{ id: 'my', group: 'Actions', label: 'Go to My issues', icon: 'circle-user-round', keys: 'g m', run: () => navigate('/my') },
		{ id: 'board', group: 'Actions', label: 'Go to Board', icon: 'square-kanban', keys: 'g b', run: () => navigate('/board') },
		{ id: 'insights', group: 'Actions', label: 'Go to Insights', icon: 'chart-column', keys: 'g n', run: () => navigate('/insights') },
		{ id: 'settings', group: 'Actions', label: 'Go to Settings', icon: 'settings', keys: 'g s', run: () => navigate('/settings') },
		{ id: 'shortcuts', group: 'Actions', label: 'Keyboard shortcuts', icon: 'keyboard', keys: '?', run: () => (ui.shortcuts = true) },
		{
			id: 'compact',
			group: 'Actions',
			label: settings.compact ? 'Use comfortable rows' : 'Use compact rows',
			icon: 'rows-3',
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
						icon: 'undo-2' as const,
						run: () => {
							const label = undo();
							if (label) push({ text: `Undid: ${label}` });
						}
					}
				]
			: [])
	]);

	let query = $state('');

	const haystacks = new WeakMap<Issue, string>();
	function haystack(issue: Issue): string {
		let text = haystacks.get(issue);
		if (text === undefined) haystacks.set(issue, (text = `${issue.key} ${issue.title}`.toLowerCase()));
		return text;
	}

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
				run: () => navigate(`/projects/${project.key}`)
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
						navigate(`/issues/${issue.key}`);
					}
				});
			}
		}
		return out;
	});

	let active = $derived(items.length > 0 ? 0 : -1);

	const scope = dismiss_scope();
	$effect(() => {
		if (!ui.palette) return;
		query = '';
		return dismiss_layer({ onEscape: () => (ui.palette = false), scope });
	});

	// The palette closes first, so an item that opens another overlay opens it on top.
	function run(index: number) {
		const item = items[index];
		if (!item) return;
		ui.palette = false;
		item.run();
	}

	function onkeydown(event: GpuixEvent) {
		const n = items.length;
		if (event.key === 'down' && n > 0) active = (active + 1) % n;
		else if (event.key === 'up' && n > 0) active = (active - 1 + n) % n;
		else return;
		event.preventDefault();
	}
</script>

{#if ui.palette}
	<Portal>
		<div class="scrim" onclick={() => (ui.palette = false)}>
			<div class="panel" testid="palette" onclick={(e: GpuixEvent) => e.stopPropagation()}>
				<input
					class="input"
					testid="palette-input"
					placeholder="Type a command or search issues…"
					autofocus
					value={query}
					onchange={(e) => (query = e.value)}
					onsubmit={() => run(active)}
					{onkeydown}
				/>
				<div class="list">
					{#each items as item, index (item.id)}
						{#if index === 0 || items[index - 1].group !== item.group}
							<text class="group">{item.group.toUpperCase()}</text>
						{/if}
						<div class="item" class:active={index === active} testid="palette-item" hitbox="self" onmousemove={() => (active = index)} onclick={() => run(index)}>
							<div class="icon" class:on={index === active}>
								{#if item.icon}
									<Icon name={item.icon} color={index === active ? 'var(--primary)' : 'var(--graphite-muted)'} />
								{:else if item.project}
									<ProjectSquare project={item.project} />
								{:else if item.status}
									<StatusIcon status={item.status} />
								{/if}
							</div>
							<text class="label">{item.label}</text>
							{#if item.meta}
								<text class="meta">{item.meta}</text>
							{:else if item.keys}
								<text class="kbd">{key_label(item.keys)}</text>
							{/if}
						</div>
					{:else}
						<text class="empty">No results for ‘{query.trim()}’.</text>
					{/each}
				</div>
				<div class="foot">
					<text>↑↓ move</text><text>·</text><text>enter open</text><text>·</text><text>esc close</text><text>·</text><text>&gt; actions</text>
				</div>
			</div>
		</div>
	</Portal>
{/if}

<style>
	.scrim { display: flex; flex-direction: column; align-items: center; width: 100%; height: 100%; padding-top: 144px; background-color: var(--scrim); }
	.panel {
		display: flex;
		flex-direction: column;
		width: 560px;
		overflow: hidden;
		border-radius: 8px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--graphite-2);
		background-color: var(--graphite);
		color: var(--graphite-foreground);
	}
	.input {
		height: 48px;
		flex-shrink: 0;
		padding: 0 16px;
		border-width: 0 0 1px 0;
		border-style: solid;
		border-color: var(--graphite-2);
		font-size: 15px;
		color: var(--graphite-foreground);
	}
	.list { display: flex; flex-direction: column; max-height: 360px; padding: 6px; overflow-y: scroll; }
	.group { display: flex; align-items: end; flex-shrink: 0; height: 28px; padding: 0 10px 4px 10px; font-family: JetBrains Mono; font-size: 11px; font-weight: 500; color: var(--graphite-muted); }
	.item { display: flex; flex-direction: row; align-items: center; flex-shrink: 0; gap: 10px; height: 36px; padding: 0 10px; border-radius: 6px; font-size: 13px; color: var(--graphite-foreground-85); cursor: pointer; }
	.item.active { background-color: var(--graphite-2); color: var(--graphite-foreground); }
	.icon { display: flex; align-items: center; justify-content: center; width: 16px; height: 16px; flex-shrink: 0; }
	.label { flex-grow: 1; flex-basis: 0; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
	.meta { flex-shrink: 0; font-family: JetBrains Mono; font-size: 11px; color: var(--graphite-muted); }
	.kbd { flex-shrink: 0; padding: 0 6px; border-radius: 4px; border-width: 1px; border-style: solid; border-color: var(--graphite-2); font-family: JetBrains Mono; font-size: 11px; line-height: 18px; color: var(--graphite-muted); }
	.empty { padding: 24px 10px; text-align: center; font-size: 13px; color: var(--graphite-muted); }
	.foot { display: flex; flex-direction: row; align-items: center; flex-shrink: 0; gap: 12px; height: 32px; padding: 0 16px; border-width: 1px 0 0 0; border-style: solid; border-color: var(--graphite-2); font-family: JetBrains Mono; font-size: 11px; color: var(--graphite-muted); }
</style>
