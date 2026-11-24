<!-- Numbers for the feed on screen: what struck, when, how deep and where. Plain bars: HTML, not a chart library. -->
<script lang="ts">
	import * as Card from '#lib/components/ui/card/index.ts';
	import MagBadge from '#lib/components/MagBadge.svelte';
	import { feed } from '#lib/feed.svelte.ts';
	import { band, depthClass, region } from '#lib/usgs.ts';
	import { count, day, km } from '#lib/format.ts';

	const quakes = $derived(feed.quakes);
	const strongest = $derived(quakes.reduce<(typeof quakes)[number] | undefined>((top, q) => (!top || q.mag > top.mag ? q : top), undefined));
	const felt = $derived(quakes.reduce((sum, q) => sum + (q.felt ?? 0), 0));
	const tsunamis = $derived(quakes.filter((q) => q.tsunami).length);
	const mean_depth = $derived(quakes.length ? quakes.reduce((sum, q) => sum + q.depth, 0) / quakes.length : 0);

	/** Whole-magnitude bins, as many as the feed spans. */
	const magnitudes = $derived.by(() => {
		if (!quakes.length) return [];
		const low = Math.floor(Math.min(...quakes.map((q) => q.mag)));
		const high = Math.floor(Math.max(...quakes.map((q) => q.mag)));
		const bins = Array.from({ length: high - low + 1 }, (_, i) => ({ label: `${low + i}–${low + i + 1}`, from: low + i, n: 0 }));
		for (const q of quakes) bins[Math.floor(q.mag) - low].n++;
		return bins;
	});

	/** By hour for a feed of a day or less, by day for longer ones. */
	const timeline = $derived.by(() => {
		if (!quakes.length) return { unit: 'hour', bins: [] as { label: string; n: number }[] };
		const newest = quakes[0].time;
		const oldest = quakes[quakes.length - 1].time;
		const hourly = newest - oldest <= 26 * 3600_000;
		const step = hourly ? 3600_000 : 86_400_000;
		const start = Math.floor(oldest / step) * step;
		const bins = Array.from({ length: Math.floor(newest / step) - start / step + 1 }, (_, i) => {
			const at = start + i * step;
			return { label: hourly ? `${new Date(at).getUTCHours()}h` : day(at), n: 0 };
		});
		for (const q of quakes) bins[Math.floor(q.time / step) - start / step].n++;
		return { unit: hourly ? 'hour' : 'day', bins };
	});

	const depths = $derived(
		['shallow', 'intermediate', 'deep'].map((name) => ({ name, n: quakes.filter((q) => depthClass(q.depth) === name).length }))
	);

	const regions = $derived.by(() => {
		const counts = new Map<string, number>();
		for (const q of quakes) counts.set(region(q.place), (counts.get(region(q.place)) ?? 0) + 1);
		return [...counts].sort((a, b) => b[1] - a[1]).slice(0, 8);
	});

	const most = (bins: { n: number }[]) => Math.max(1, ...bins.map((bin) => bin.n));
	const tone = ['', 'bg-mag-1', 'bg-mag-2', 'bg-mag-3', 'bg-mag-4', 'bg-mag-5'];
</script>

<svelte:head><title>Stats · Quakes</title></svelte:head>

<div class="min-h-0 flex-1 overflow-y-scroll">
	<div class="mx-auto flex max-w-5xl flex-col gap-4 p-6">
		<h1 class="text-[18px] font-semibold tracking-tight">{feed.title || 'Loading…'}</h1>

		<div class="grid grid-cols-4 gap-4">
			<Card.Root>
				<Card.Content class="flex flex-col gap-1">
					<span class="text-[12px] text-muted-foreground">Earthquakes</span>
					<span class="font-mono text-[26px]" data-testid="total">{count(quakes.length)}</span>
				</Card.Content>
			</Card.Root>
			<Card.Root>
				<Card.Content class="flex flex-col gap-1">
					<span class="text-[12px] text-muted-foreground">Strongest</span>
					{#if strongest}
						<a href="/quake/{strongest.id}" class="flex items-center gap-2 hover:text-primary">
							<MagBadge mag={strongest.mag} size="sm" />
							<span class="truncate text-[12px]">{region(strongest.place)}</span>
						</a>
					{/if}
				</Card.Content>
			</Card.Root>
			<Card.Root>
				<Card.Content class="flex flex-col gap-1">
					<span class="text-[12px] text-muted-foreground">Mean depth</span>
					<span class="font-mono text-[26px]">{km(mean_depth)}</span>
				</Card.Content>
			</Card.Root>
			<Card.Root>
				<Card.Content class="flex flex-col gap-1">
					<span class="text-[12px] text-muted-foreground">Felt reports · tsunami warnings</span>
					<span class="font-mono text-[26px]">{count(felt)} · {tsunamis}</span>
				</Card.Content>
			</Card.Root>
		</div>

		<div class="grid grid-cols-2 gap-4">
			<Card.Root>
				<Card.Header><Card.Title>By magnitude</Card.Title></Card.Header>
				<Card.Content class="flex flex-col gap-2">
					{#each magnitudes as bin (bin.label)}
						<div class="flex items-center gap-3">
							<span class="w-12 font-mono text-[12px] text-muted-foreground">{bin.label}</span>
							<div class="h-4 flex-1 rounded-sm bg-muted">
								<div class="h-4 rounded-sm {tone[band(bin.from + 0.5)]}" style="width: {(bin.n / most(magnitudes)) * 100}%"></div>
							</div>
							<span class="w-10 text-right font-mono text-[12px]">{bin.n}</span>
						</div>
					{/each}
				</Card.Content>
			</Card.Root>

			<Card.Root>
				<Card.Header><Card.Title>By depth</Card.Title></Card.Header>
				<Card.Content class="flex flex-col gap-2">
					{#each depths as bin (bin.name)}
						<div class="flex items-center gap-3">
							<span class="w-24 text-[12px] text-muted-foreground">{bin.name}</span>
							<div class="h-4 flex-1 rounded-sm bg-muted">
								<div class="h-4 rounded-sm bg-primary/80" style="width: {(bin.n / most(depths)) * 100}%"></div>
							</div>
							<span class="w-10 text-right font-mono text-[12px]">{bin.n}</span>
						</div>
					{/each}
					<p class="mt-1 text-[12px] text-muted-foreground">Shallow is under 70 km, deep over 300 km.</p>
				</Card.Content>
			</Card.Root>

			<Card.Root>
				<Card.Header><Card.Title>By {timeline.unit} (UTC)</Card.Title></Card.Header>
				<Card.Content>
					<div class="flex h-36 items-end gap-0.5">
						{#each timeline.bins as bin, i (i)}
							<div class="flex-1 rounded-t-sm bg-primary/80" style="height: {(bin.n / most(timeline.bins)) * 100}%"></div>
						{/each}
					</div>
					<div class="mt-2 flex justify-between font-mono text-[11px] text-muted-foreground">
						<span>{timeline.bins[0]?.label ?? ''}</span>
						<span>{timeline.bins[timeline.bins.length - 1]?.label ?? ''}</span>
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root>
				<Card.Header><Card.Title>Most active regions</Card.Title></Card.Header>
				<Card.Content class="flex flex-col gap-1.5">
					{#each regions as [name, n] (name)}
						<div class="flex items-center justify-between text-[13px]">
							<span class="truncate">{name}</span>
							<span class="font-mono text-muted-foreground">{n}</span>
						</div>
					{/each}
				</Card.Content>
			</Card.Root>
		</div>
	</div>
</div>
