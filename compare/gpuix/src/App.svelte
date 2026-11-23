<!--
	The shell (examples/tracker/src/routes/+layout.svelte): sidebar, the screen, and the overlays.
	Routes are gpuix-svelte-ui's Router with an explicit table. /issues and /issues/:key are drawn
	by IssuesLayout from the Router's layout, outside its per-path {#key}, so the list and the pane
	stay mounted as the key changes (the Tracker's router keeps them too). Keys come from
	`on_window_key`; Escape from a dismiss layer under every other, so open Selects and Dialogs
	get it first, as Bits UI's do in the Tracker.
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { dismiss_layer, focus_element, on_window_key, set_window_title, type GpuixEvent, type ShadowNode } from 'gpuix-svelte';
	import { Router, after, apply_theme, before, navigate, router, type Palette } from 'gpuix-svelte-ui';
	import { BENCH, mark, mark_after_tick, set_rect_root, start_rect_poll } from './bench.ts';
	import Avatar from './components/Avatar.svelte';
	import Icon from './components/Icon.svelte';
	import ProjectSquare from './components/ProjectSquare.svelte';
	import { count } from './format.ts';
	import type { IconName } from './icons.ts';
	import Insights from './insights/Insights.svelte';
	import Board from './board/Board.svelte';
	import CommandPalette from './overlays/CommandPalette.svelte';
	import NewIssue from './overlays/NewIssue.svelte';
	import Shortcuts from './overlays/Shortcuts.svelte';
	import Toasts from './overlays/Toasts.svelte';
	import Empty from './screens/Empty.svelte';
	import IssuesLayout from './screens/IssuesLayout.svelte';
	import My from './screens/My.svelte';
	import NotFound from './screens/NotFound.svelte';
	import Project from './screens/Project.svelte';
	import Settings from './screens/Settings.svelte';
	import { ME, PROJECTS, project_by_key } from './seed.ts';
	import { data, person, view } from './store.svelte.ts';
	import { escape, handle_key, key_event, ui, use_keys } from './ui.svelte.ts';

	// gpuix-svelte-ui's components read --ui-* tokens: the Tracker's palette (App's :root below).
	const TRACKER: Palette = {
		appearance: 'light',
		bg: '#f8fafd',
		panel: '#fcfeff',
		raised: '#f1f4f7',
		'raised-hover': '#e8edf4',
		border: '#dadee3',
		'border-strong': '#d0d5db',
		ink: '#192029',
		muted: '#575e68',
		accent: '#0076ed',
		'accent-ink': '#0076ed',
		'accent-wash': '#0076ed1a',
		'on-accent': '#f8fafd',
		success: '#249057',
		'success-wash': '#2490571f',
		warning: '#be7125',
		'warning-wash': '#be71251f',
		danger: '#d02b31',
		'danger-wash': '#d02b311f'
	};
	$effect.pre(() => apply_theme(TRACKER));

	// / redirects to /issues (the Tracker's +page.ts).
	before((to) => (to.path === '/' ? '/issues' : undefined));

	// Bench: nav:<path> once the new screen is in (not the first screen, as in the Tracker).
	let first = true;
	after((to) => {
		if (first) first = false;
		else mark_after_tick(`nav:${to.path}`);
	});
	// Bench: search:<query> after each change of the search (the Tracker watches view.search in its shell too).
	let first_search = true;
	$effect(() => {
		const query = view.search;
		if (first_search) first_search = false;
		else mark_after_tick(`search:${query}`);
	});

	const nav: { name: string; label: string; href: string; icon: IconName }[] = [
		{ name: 'issues', label: 'Issues', href: '/issues', icon: 'layout-list' },
		{ name: 'my', label: 'My issues', href: '/my', icon: 'circle-user-round' },
		{ name: 'board', label: 'Board', href: '/board', icon: 'square-kanban' },
		{ name: 'insights', label: 'Insights', href: '/insights', icon: 'chart-column' }
	];

	const path = $derived(router.path);
	const active = (href: string) => path === href || path.startsWith(href + '/');

	const title = $derived.by(() => {
		const [, first, second] = path.split('/');
		if (first === 'issues' && second) return second;
		if (first === 'projects' && second) return project_by_key[second]?.name ?? second;
		const names: Record<string, string> = { issues: 'Issues', my: 'My issues', board: 'Board', insights: 'Insights', settings: 'Settings' };
		return names[first] ?? '';
	});
	$effect(() => set_window_title(title ? `${title} · Tracker` : 'Tracker'));

	$effect(() => on_window_key('keydown', (e) => handle_key(key_event(e))));
	// Under every other layer, so it hears only the Escapes no Select or Dialog took.
	$effect(() => dismiss_layer({ onEscape: escape }));

	use_keys({
		'Mod+k': () => {
			ui.palette = true;
		},
		c: () => {
			ui.new_issue = true;
		},
		'/': () => {
			if (!ui.search) return false;
			focus_element(ui.search);
		},
		'?': () => {
			ui.shortcuts = true;
		},
		'g i': () => void navigate('/issues'),
		'g m': () => void navigate('/my'),
		'g b': () => void navigate('/board'),
		'g n': () => void navigate('/insights'),
		'g s': () => void navigate('/settings')
	});

	const me = $derived(person(ME)!);

	// Bench: the clock (scenario 4), resize marks, RECT lines.
	let clock = $state('');
	onMount(() => {
		start_rect_poll();
		if (!BENCH?.clock) return;
		const show = () => (clock = new Date().toTimeString().slice(0, 8));
		show();
		const timer = setInterval(() => {
			show();
			mark_after_tick('clock');
		}, 1000);
		return () => clearInterval(timer);
	});
	let size = '';
	function on_layout(e: GpuixEvent) {
		const next = `${Math.round(e.width ?? 0)}x${Math.round(e.height ?? 0)}`;
		if (size !== '' && next !== size) mark_after_tick(`resize:${next}`);
		size = next;
	}

	const routes = {
		'/': Empty,
		'/issues': Empty,
		'/issues/:key': Empty,
		'/my': My,
		'/projects/:key': Project,
		'/board': Board,
		'/insights': Insights,
		'/settings': Settings,
		'*': NotFound
	};
</script>

<div class="app" onlayout={on_layout} {@attach (node: ShadowNode) => set_rect_root(node)}>
	<div class="sidebar">
		<div class="top">
			<text class="brand">Tracker</text>
			{#if clock}<text class="clock" testid="bench-clock">{clock}</text>{/if}
			<button class="palette-button" testid="nav-palette" hitbox="self" onclick={() => (ui.palette = true)}>⌘K</button>
		</div>
		<button class="new" testid="nav-new" hitbox="self" onclick={() => (ui.new_issue = true)}>
			<div class="new-label"><Icon name="plus" color="var(--primary-foreground)" /><text>New issue</text></div>
			<text class="new-key">C</text>
		</button>

		<div class="nav">
			{#each nav as entry (entry.name)}
				<div class="item" class:on={active(entry.href)} testid="nav-{entry.name}" hitbox="self" onclick={() => navigate(entry.href)}>
					<Icon name={entry.icon} color={active(entry.href) ? 'var(--primary)' : 'var(--muted-foreground)'} />
					<text>{entry.label}</text>
				</div>
			{/each}
		</div>

		<text class="section">PROJECTS</text>
		<div class="nav">
			{#each PROJECTS as project (project.key)}
				{@const href = `/projects/${project.key}`}
				<div class="item" class:on={active(href)} testid="nav-project-{project.key}" hitbox="self" onclick={() => navigate(href)}>
					<ProjectSquare project={project.key} />
					<text class="project-name">{project.name}</text>
					<text class="open">{count(data.open_by_project[project.key])}</text>
				</div>
			{/each}
		</div>

		<div class="bottom">
			<div class="item" class:on={active('/settings')} testid="nav-settings" hitbox="self" onclick={() => navigate('/settings')}>
				<Icon name="settings" color={active('/settings') ? 'var(--primary)' : 'var(--muted-foreground)'} />
				<text>Settings</text>
			</div>
			<div class="me">
				<Avatar id={ME} />
				<text class="me-name" testid="user-name">{me.name}</text>
			</div>
		</div>
	</div>

	<div class="main">
		<Router {routes} initial="/issues" mode="memory" shortcuts={false}>
			{#snippet layout(content, route)}
				{#if route.pattern === '/issues' || route.pattern === '/issues/:key'}
					<IssuesLayout key={route.params.key ?? null} />
				{:else}
					{@render content()}
				{/if}
			{/snippet}
		</Router>
	</div>

	<CommandPalette />
	<NewIssue />
	<Shortcuts />
	<Toasts />
</div>

<style>
	:root {
		/* examples/tracker/src/app.css, oklch() worked out to sRGB hex (scripts/colors.ts): this renderer takes hex, rgb() and hsl() */
		--background: #f8fafd;
		--foreground: #192029;
		--ink-2: #323841;
		--card: #fcfeff;
		--popover: #fcfeff;
		--primary: #0076ed;
		--primary-80: #0076edcc;
		--primary-foreground: #f8fafd;
		--primary-foreground-70: #f8fafdb3;
		--ring: #0076ed;
		--secondary: #edf0f4;
		--muted: #f1f4f7;
		--muted-foreground: #575e68;
		--muted-foreground-60: #575e6899;
		--muted-foreground-50: #575e6880;
		--accent: #e8edf4;
		--destructive: #d02b31;
		--destructive-10: #d02b311a;
		--destructive-20: #d02b3133;
		--border: #dadee3;
		--input: #d0d5db;
		--sidebar: #f3f5f8;
		--sidebar-accent: #e3e8ef;
		--graphite: #161b22;
		--graphite-2: #262c34;
		--graphite-foreground: #e8ebef;
		--graphite-foreground-85: #e8ebefd9;
		--graphite-muted: #9299a2;
		--row-hover: #edf0f6;
		--row-active: #deedff;
		--row-checked: #e9f3ff;
		--status-backlog: #575e68;
		--status-todo: #323841;
		--status-progress: #0076ed;
		--status-review: #8962c5;
		--status-done: #249057;
		--status-canceled: #575e68;
		--priority-urgent: #d02b31;
		--chart-1: #0076ed;
		--chart-2: #a1c0eb;
		--chart-3: #7b8189;
		--chart-4: #4f565f;
		--chart-5: #2d333c;
		--foreground-4: #1920290a;
		--scrim: #0000001a;
		--hue-1: #4e87d3;
		--hue-2: #349d62;
		--hue-3: #be7125;
		--hue-4: #9372c8;
		--hue-5: #c86265;
		--hue-6: #009ca5;
		--hue-7: #998700;
		--hue-8: #b9649f;
		--hue-9: #0093c5;
		--hue-10: #6c9539;
		--hue-11: #c86556;
		--hue-12: #787bd2;
	}
	.app {
		display: flex;
		flex-direction: row;
		width: 100%;
		height: 100%;
		overflow: hidden;
		background-color: var(--background);
		color: var(--ink-2);
		font-family: Inter;
		font-size: 13px;
		line-height: 18.85px;
	}
	.sidebar {
		display: flex;
		flex-direction: column;
		width: 240px;
		flex-shrink: 0;
		min-height: 0;
		gap: 4px;
		padding: 12px;
		border-width: 0 1px 0 0;
		border-style: solid;
		border-color: var(--border);
		background-color: var(--sidebar);
	}
	.top { display: flex; flex-direction: row; align-items: center; justify-content: space-between; height: 28px; margin-bottom: 8px; padding-left: 8px; }
	.brand { font-family: Space Grotesk Light; font-size: 15px; font-weight: 600; color: var(--foreground); }
	.clock { font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.palette-button {
		display: flex;
		align-items: center;
		height: 28px;
		padding: 0 8px;
		border-radius: 6px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
		font-family: JetBrains Mono;
		font-size: 11px;
		color: var(--muted-foreground);
		cursor: pointer;
	}
	.palette-button:hover { background-color: var(--sidebar-accent); }
	.new {
		display: flex;
		flex-direction: row;
		align-items: center;
		justify-content: space-between;
		flex-shrink: 0;
		height: 32px;
		margin-bottom: 12px;
		padding: 0 10px;
		border-radius: 6px;
		background-color: var(--primary);
		color: var(--primary-foreground);
		font-size: 14px;
		font-weight: 500;
		cursor: pointer;
	}
	.new:hover { background-color: var(--primary-80); }
	.new-label { display: flex; flex-direction: row; align-items: center; gap: 6px; }
	.new-key { font-family: JetBrains Mono; font-size: 11px; color: var(--primary-foreground-70); }
	.nav { display: flex; flex-direction: column; gap: 2px; }
	.item {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-shrink: 0;
		gap: 10px;
		height: 30px;
		padding: 0 8px;
		border-radius: 6px;
		color: var(--ink-2);
		cursor: pointer;
	}
	.item:hover { background-color: var(--sidebar-accent); }
	.item.on { background-color: var(--sidebar-accent); color: var(--foreground); font-weight: 500; }
	.section { margin: 20px 0 4px 0; padding: 0 8px; font-family: JetBrains Mono; font-size: 11px; font-weight: 500; color: var(--muted-foreground); }
	.project-name { flex-grow: 1; flex-basis: 0; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
	.open { font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.bottom { display: flex; flex-direction: column; justify-content: flex-end; flex-grow: 1; gap: 2px; }
	.me { display: flex; flex-direction: row; align-items: center; gap: 10px; height: 36px; padding: 0 8px; color: var(--foreground); }
	.me-name { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; font-weight: 500; }
	.main { display: flex; flex-direction: column; flex-grow: 1; flex-basis: 0; min-width: 0; min-height: 0; }
</style>
