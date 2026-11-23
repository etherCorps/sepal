<!--
	The shell: sidebar, the screen, and the overlays. Always mounted.
	It holds the app's only document keydown listener (`handle_key` in
	ui.svelte.ts); screens add keys with `use_keys`. Each screen renders its
	own header (`PageHeader`) and toolbar, so the shell has none.
-->
<script lang="ts">
	import '../app.css';
	import { tick, type Snippet } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { setTitle } from '@ethercorps/svelte-blitz/window';
	import LayoutListIcon from '@lucide/svelte/icons/layout-list';
	import CircleUserIcon from '@lucide/svelte/icons/circle-user-round';
	import KanbanIcon from '@lucide/svelte/icons/square-kanban';
	import ChartIcon from '@lucide/svelte/icons/chart-column';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Avatar, ProjectSquare } from '$lib/glyphs';
	import { PROJECTS, ME, project_by_key } from '$lib/seed';
	import { data, person, view } from '$lib/store.svelte';
	import { bench, bench_ready, mark, rects } from '$lib/bench';
	import { count } from '$lib/format';
	import { cn } from '$lib/utils';
	import { handle_key, ui, use_keys } from '$lib/ui.svelte';
	import CommandPalette from '$lib/overlays/CommandPalette.svelte';
	import NewIssue from '$lib/overlays/NewIssue.svelte';
	import Shortcuts from '$lib/overlays/Shortcuts.svelte';
	import Toasts from '$lib/overlays/Toasts.svelte';

	let { children }: { children: Snippet } = $props();

	const nav = [
		{ name: 'issues', label: 'Issues', href: '/issues', icon: LayoutListIcon },
		{ name: 'my', label: 'My issues', href: '/my', icon: CircleUserIcon },
		{ name: 'board', label: 'Board', href: '/board', icon: KanbanIcon },
		{ name: 'insights', label: 'Insights', href: '/insights', icon: ChartIcon }
	];

	const path = $derived(page.url.pathname);
	const active = (href: string) => path === href || path.startsWith(href + '/');

	// The window title follows the route.
	const title = $derived.by(() => {
		const [, first, second] = path.split('/');
		if (first === 'issues' && second) return second;
		if (first === 'projects' && second) return project_by_key[second]?.name ?? second;
		const names: Record<string, string> = { issues: 'Issues', my: 'My issues', board: 'Board', insights: 'Insights', settings: 'Settings' };
		return names[first] ?? '';
	});
	$effect(() => {
		setTitle(title ? `${title} · Tracker` : 'Tracker').catch(() => {});
	});

	// dom: true gives a `document`; <svelte:window> does not exist for custom renderers.
	$effect(() => {
		document.addEventListener('keydown', handle_key);
		return () => document.removeEventListener('keydown', handle_key);
	});

	const focus_search = () => {
		const search = document.querySelector<HTMLElement>('[data-testid=search]');
		if (!search) return false;
		search.focus();
	};
	use_keys({
		'Mod+k': () => {
			ui.palette = true;
		},
		c: () => {
			ui.new_issue = true;
		},
		'/': focus_search,
		'?': () => {
			ui.shortcuts = true;
		},
		'g i': () => void goto('/issues'),
		'g m': () => void goto('/my'),
		'g b': () => void goto('/board'),
		'g n': () => void goto('/insights'),
		'g s': () => void goto('/settings')
	});

	// Bench mode (compare/shared/PROTOCOL.md). `bench` is null until bench.json
	// was read, so the first navigation and the empty search make no marks.
	$effect(() => {
		const at = path;
		if (bench) tick().then(() => (mark(`nav:${at}`), rects()));
	});
	$effect(() => {
		const query = view.search;
		if (bench) tick().then(() => mark(`search:${query}`));
	});
	let clock = $state('');
	bench_ready.then((found) => {
		if (!found) return;
		window.addEventListener('resize', () => {
			const size = `${Math.round(innerWidth)}x${Math.round(innerHeight)}`;
			tick().then(() => mark(`resize:${size}`));
		});
		if (!found.clock) return;
		const show = () => (clock = new Date().toTimeString().slice(0, 8));
		show();
		setInterval(() => {
			show();
			tick().then(() => mark('clock'));
		}, 1000);
	});

	const me = $derived(person(ME)!);
	const item = 'text-sidebar-foreground hover:bg-sidebar-accent focus-visible:ring-ring flex h-[30px] items-center gap-2.5 rounded-md px-2 outline-none transition-colors focus-visible:ring-2';
	const item_active = 'bg-sidebar-accent text-foreground font-medium';
</script>

<div class="bg-background text-ink-2 grid h-screen grid-cols-[240px_minmax(0,1fr)] overflow-hidden">
	<aside class="bg-sidebar flex min-h-0 flex-col gap-1 border-r p-3">
		<div class="mb-2 flex h-7 items-center justify-between pl-2">
			<span class="font-display text-foreground text-[15px] font-semibold">Tracker</span>
			{#if clock}<span data-testid="bench-clock" class="text-muted-foreground font-mono text-[11px]">{clock}</span>{/if}
			<button
				data-testid="nav-palette"
				class="text-muted-foreground hover:bg-sidebar-accent flex h-7 items-center rounded-md border px-2 font-mono text-[11px] transition-colors"
				onclick={() => (ui.palette = true)}>⌘K</button
			>
		</div>
		<Button data-testid="nav-new" class="mb-3 h-8 w-full justify-between" onclick={() => (ui.new_issue = true)}>
			<span class="flex items-center gap-1.5"><PlusIcon class="size-4" strokeWidth={1.75} />New issue</span>
			<span class="text-primary-foreground/70 font-mono text-[11px]">C</span>
		</Button>

		<nav class="flex flex-col gap-0.5">
			{#each nav as entry (entry.name)}
				<a data-testid="nav-{entry.name}" href={entry.href} class={[item, active(entry.href) && item_active]}>
					<entry.icon class={cn('size-4', active(entry.href) ? 'text-primary' : 'text-muted-foreground')} strokeWidth={1.75} />
					{entry.label}
				</a>
			{/each}
		</nav>

		<div class="text-muted-foreground mt-5 mb-1 px-2 font-mono text-[11px] font-medium tracking-[0.06em] uppercase">Projects</div>
		<nav class="flex flex-col gap-0.5">
			{#each PROJECTS as project (project.key)}
				{@const href = `/projects/${project.key}`}
				<a data-testid="nav-project-{project.key}" {href} class={[item, active(href) && item_active]}>
					<ProjectSquare project={project.key} />
					<span class="min-w-0 flex-1 truncate">{project.name}</span>
					<span class="text-muted-foreground font-mono text-[11px]">{count(data.open_by_project[project.key])}</span>
				</a>
			{/each}
		</nav>

		<div class="mt-auto flex flex-col gap-0.5">
			<a data-testid="nav-settings" href="/settings" class={[item, active('/settings') && item_active]}>
				<SettingsIcon class={cn('size-4', active('/settings') ? 'text-primary' : 'text-muted-foreground')} strokeWidth={1.75} />
				Settings
			</a>
			<div class="text-foreground flex h-9 items-center gap-2.5 px-2">
				<Avatar id={ME} />
				<span data-testid="user-name" class="truncate font-medium">{me.name}</span>
			</div>
		</div>
	</aside>

	<main class="flex min-h-0 min-w-0 flex-col">
		{@render children()}
	</main>
</div>

<CommandPalette />
<NewIssue />
<Shortcuts />
<Toasts />
