<script lang="ts">
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import WavesIcon from '@lucide/svelte/icons/waves';
	import * as Tabs from '#lib/components/ui/tabs/index.ts';
	import MagBadge from '#lib/components/MagBadge.svelte';
	import WorldMap from '#lib/components/WorldMap.svelte';
	import Seismogram from '#lib/components/Seismogram.svelte';
	import { feed } from '#lib/feed.svelte.ts';
	import { ago, km } from '#lib/format.ts';
	import { cn } from '#lib/utils.ts';

	let order = $state<'newest' | 'strongest'>('newest');

	const list = $derived(order === 'newest' ? feed.quakes : [...feed.quakes].sort((a, b) => b.mag - a.mag));
	const selected = $derived(feed.find(feed.selected));
	const legend = [
		{ label: '< 1.5', tone: 'bg-mag-1' },
		{ label: '1.5', tone: 'bg-mag-2' },
		{ label: '3', tone: 'bg-mag-3' },
		{ label: '4.5', tone: 'bg-mag-4' },
		{ label: '6+', tone: 'bg-mag-5' }
	];
</script>

<svelte:head><title>Map · Quakes</title></svelte:head>

<div class="flex min-h-0 flex-1">
	<section class="flex min-w-0 flex-1 flex-col gap-3 p-5">
		<div class="relative overflow-hidden rounded-xl border bg-ocean">
			<WorldMap quakes={feed.quakes} selected={feed.selected} onselect={(id) => (feed.selected = id)} />

			{#if selected}
				<div class="absolute bottom-3 left-3 flex max-w-md items-center gap-3 rounded-lg border bg-popover/95 p-3 shadow-lg" data-testid="selected">
					<MagBadge mag={selected.mag} size="md" />
					<div class="min-w-0">
						<div class="truncate font-medium">{selected.place}</div>
						<div class="font-mono text-[12px] text-muted-foreground">
							{ago(selected.time, feed.now)} · {km(selected.depth)} deep
						</div>
					</div>
					<a href="/quake/{selected.id}" class="ml-2 flex shrink-0 items-center gap-1 text-[13px] font-medium text-primary hover:underline">
						Details <ArrowRightIcon class="size-4" />
					</a>
				</div>
			{/if}
		</div>

		<div class="flex items-center gap-4 text-[12px] text-muted-foreground">
			<span>Magnitude</span>
			{#each legend as step (step.label)}
				<span class="flex items-center gap-1.5"><span class="size-2.5 rounded-full {step.tone}"></span>{step.label}</span>
			{/each}
			<span class="ml-auto">{feed.title}</span>
		</div>

		<div class="mt-2 rounded-xl border bg-card p-4">
			<div class="mb-2 text-[12px] font-medium text-muted-foreground">Over time</div>
			<Seismogram quakes={feed.quakes} preset={feed.preset} now={feed.fetched || feed.now} selected={feed.selected} />
		</div>
	</section>

	<aside class="flex w-[380px] shrink-0 flex-col border-l">
		<div class="flex items-center justify-between px-4 py-3">
			<h2 class="text-[13px] font-semibold">Earthquakes</h2>
			<Tabs.Root bind:value={order}>
				<Tabs.List>
					<Tabs.Trigger value="newest">Newest</Tabs.Trigger>
					<Tabs.Trigger value="strongest">Strongest</Tabs.Trigger>
				</Tabs.List>
			</Tabs.Root>
		</div>

		<div class="min-h-0 flex-1 overflow-y-scroll border-t" data-testid="list">
			{#if feed.fetched && list.length === 0}
				<p class="p-6 text-center text-muted-foreground">No earthquakes in this feed right now.</p>
			{/if}
			{#each list as quake (quake.id)}
				<button
					type="button"
					class={cn(
						'flex w-full items-center gap-3 border-b px-4 py-2.5 text-left hover:bg-accent',
						quake.id === feed.selected && 'bg-accent'
					)}
					onclick={() => (feed.selected = quake.id)}
				>
					<MagBadge mag={quake.mag} size="sm" />
					<div class="min-w-0 flex-1">
						<div class="truncate">{quake.place}</div>
						<div class="font-mono text-[11px] text-muted-foreground">
							{ago(quake.time, feed.now)} · {km(quake.depth)}{quake.magType ? ` · ${quake.magType}` : ''}
						</div>
					</div>
					{#if quake.tsunami}
						<WavesIcon class="size-4 shrink-0 text-mag-4" aria-label="Tsunami warning issued" />
					{/if}
				</button>
			{/each}
		</div>
	</aside>
</div>
