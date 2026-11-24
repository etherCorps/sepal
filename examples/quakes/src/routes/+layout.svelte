<!-- The window's frame: the feed picker and the pages. It stays mounted while pages change, and so does the feed. -->
<script lang="ts">
	import '../app.css';
	import type { Snippet } from 'svelte';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import ActivityIcon from '@lucide/svelte/icons/activity';
	import RotateCwIcon from '@lucide/svelte/icons/rotate-cw';
	import * as Select from '#lib/components/ui/select/index.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import { feed } from '#lib/feed.svelte.ts';
	import { PRESETS, type PresetId } from '#lib/usgs.ts';
	import { clock, count } from '#lib/format.ts';
	import { cn } from '#lib/utils.ts';

	let { children }: { children: Snippet } = $props();

	onMount(() => {
		feed.start();
		return () => feed.stop();
	});

	const links = [
		{ href: '/', label: 'Map' },
		{ href: '/stats', label: 'Stats' }
	];
	const current = $derived(PRESETS.find((preset) => preset.id === feed.preset)!);
</script>

<div class="flex h-screen flex-col">
	<header class="flex h-14 shrink-0 items-center gap-6 border-b px-5">
		<a href="/" class="flex items-center gap-2 text-[15px] font-semibold tracking-tight text-foreground">
			<ActivityIcon class="size-5 text-primary" />
			Quakes
		</a>
		<nav class="flex items-center gap-1">
			{#each links as link (link.href)}
				<a
					href={link.href}
					class={cn(
						'rounded-md px-3 py-1.5 text-[13px] font-medium',
						page.url.pathname === link.href ? 'bg-accent text-foreground' : 'text-muted-foreground hover:text-foreground'
					)}
				>
					{link.label}
				</a>
			{/each}
		</nav>

		<div class="ml-auto flex items-center gap-3">
			<span class="font-mono text-[12px] text-muted-foreground" data-testid="status">
				{#if feed.error}
					<span class="text-destructive">offline: {feed.error}</span>
				{:else if feed.fetched}
					{count(feed.quakes.length)} quakes · updated {clock(feed.fetched)}
				{:else}
					loading…
				{/if}
			</span>
			<Select.Root type="single" bind:value={() => feed.preset, (next) => next && feed.choose(next as PresetId)}>
				<Select.Trigger size="sm" class="w-52" data-testid="preset">
					<span class="font-medium">{current.label}</span>
					<span class="text-muted-foreground">{current.period}</span>
				</Select.Trigger>
				<Select.Content>
					{#each PRESETS as preset (preset.id)}
						<Select.Item value={preset.id} label="{preset.label}, {preset.period}">
							<span class="font-medium">{preset.label}</span>
							<span class="text-muted-foreground">{preset.period}</span>
						</Select.Item>
					{/each}
				</Select.Content>
			</Select.Root>
			<Button variant="outline" size="icon-sm" aria-label="Refresh" disabled={feed.loading} onclick={() => feed.refresh()}>
				<RotateCwIcon />
			</Button>
		</div>
	</header>

	<main class="flex min-h-0 flex-1 flex-col">
		{@render children()}
	</main>
</div>
