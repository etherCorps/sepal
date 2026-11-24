<script lang="ts">
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
	import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
	import { open } from 'sepal/shell';
	import { Badge } from '#lib/components/ui/badge/index.ts';
	import { Button } from '#lib/components/ui/button/index.ts';
	import * as Card from '#lib/components/ui/card/index.ts';
	import MagBadge from '#lib/components/MagBadge.svelte';
	import { feed } from '#lib/feed.svelte.ts';
	import { distance, globe, PALETTE } from '#lib/geo.ts';
	import { depthClass } from '#lib/usgs.ts';
	import { ago, coords, count, datetime, km, moment } from '#lib/format.ts';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	const detail = $derived(data.detail);
	const quake = $derived(detail.quake);
	const view = $derived(globe(quake.lon, quake.lat, 240));
	/** Other quakes in the feed on screen within 500 km, nearest first. */
	const nearby = $derived(
		feed.quakes
			.filter((other) => other.id !== quake.id)
			.map((other) => ({ quake: other, km: distance(quake, other) }))
			.filter((other) => other.km < 500)
			.sort((a, b) => a.km - b.km)
			.slice(0, 6)
	);
	const alert_tone: Record<string, string> = {
		green: 'border-mag-2/50 text-mag-2',
		yellow: 'border-mag-3/50 text-mag-3',
		orange: 'border-mag-4/50 text-mag-4',
		red: 'border-mag-5/50 text-mag-5'
	};
</script>

<svelte:head><title>M{quake.mag.toFixed(1)} {quake.place} · Quakes</title></svelte:head>

<div class="min-h-0 flex-1 overflow-y-scroll">
	<div class="mx-auto flex max-w-5xl flex-col gap-5 p-6">
		<a href="/" class="flex w-fit items-center gap-1.5 text-[13px] text-muted-foreground hover:text-foreground">
			<ArrowLeftIcon class="size-4" /> Map
		</a>

		<header class="flex items-start gap-5">
			<MagBadge mag={quake.mag} size="lg" />
			<div class="min-w-0 flex-1">
				<h1 class="text-[22px] font-semibold tracking-tight" data-testid="place">{quake.place}</h1>
				<p class="mt-1 font-mono text-[12px] text-muted-foreground">
					{datetime(quake.time)} · {ago(quake.time, feed.now)} · {quake.id}
				</p>
				<div class="mt-3 flex flex-wrap gap-2">
					{#if detail.pager}
						<Badge variant="outline" class={alert_tone[detail.pager.alert]}>PAGER {detail.pager.alert}</Badge>
					{/if}
					{#if quake.tsunami}
						<Badge variant="outline" class="border-mag-4/50 text-mag-4">Tsunami warning issued</Badge>
					{/if}
					<Badge variant="outline">{quake.status}</Badge>
					{#each detail.products as product (product)}
						<Badge variant="secondary">{product}</Badge>
					{/each}
				</div>
			</div>
			<Button variant="outline" onclick={() => open(quake.url)}>
				Open on USGS <ExternalLinkIcon />
			</Button>
		</header>

		<div class="grid grid-cols-3 gap-4">
			<Card.Root class="col-span-1">
				<Card.Header><Card.Title>Where</Card.Title></Card.Header>
				<Card.Content class="flex flex-col items-center gap-3">
					<svg width="240" height="240" viewBox="0 0 240 240" aria-hidden="true">
						<path d={view.sphere} fill={PALETTE.ocean} stroke={PALETTE.coast} />
						<path d={view.graticule} fill="none" stroke={PALETTE.grid} stroke-width="0.6" />
						<path d={view.land} fill={PALETTE.land} />
						<path d={view.borders} fill="none" stroke={PALETTE.coast} stroke-width="0.5" />
						<circle cx={view.point.x} cy={view.point.y} r="14" fill="none" stroke={PALETTE.mag[5]} stroke-width="1.5" opacity="0.6" />
						<circle cx={view.point.x} cy={view.point.y} r="5" fill={PALETTE.mag[5]} stroke="white" stroke-width="1.5" />
					</svg>
					<dl class="grid w-full grid-cols-2 gap-y-1.5 text-[12px]">
						<dt class="text-muted-foreground">Position</dt>
						<dd class="text-right font-mono">{coords(quake.lat, quake.lon)}</dd>
						<dt class="text-muted-foreground">Depth</dt>
						<dd class="text-right font-mono">{km(quake.depth)}, {depthClass(quake.depth)}</dd>
					</dl>
				</Card.Content>
			</Card.Root>

			<div class="col-span-2 grid grid-cols-2 gap-4">
				<Card.Root>
					<Card.Header><Card.Title>Measurement</Card.Title></Card.Header>
					<Card.Content>
						<dl class="grid grid-cols-2 gap-y-2 text-[13px]">
							<dt class="text-muted-foreground">Magnitude</dt>
							<dd class="text-right font-mono">{quake.mag.toFixed(1)} {quake.magType}</dd>
							<dt class="text-muted-foreground">Stations</dt>
							<dd class="text-right font-mono">{detail.stations ?? '—'}</dd>
							<dt class="text-muted-foreground">Location error</dt>
							<dd class="text-right font-mono">{detail.horizontalError != null ? km(detail.horizontalError) : '—'}</dd>
							<dt class="text-muted-foreground">Depth from</dt>
							<dd class="text-right">{detail.depthType ?? '—'}</dd>
						</dl>
					</Card.Content>
				</Card.Root>

				<Card.Root>
					<Card.Header><Card.Title>Impact</Card.Title></Card.Header>
					<Card.Content>
						<dl class="grid grid-cols-2 gap-y-2 text-[13px]">
							<dt class="text-muted-foreground">Significance</dt>
							<dd class="text-right font-mono">{count(quake.sig)}</dd>
							<dt class="text-muted-foreground">Felt reports</dt>
							<dd class="text-right font-mono">{quake.felt != null ? count(quake.felt) : '—'}</dd>
							<dt class="text-muted-foreground">PAGER alert</dt>
							<dd class="text-right">{detail.pager?.alert ?? '—'}</dd>
							<dt class="text-muted-foreground">Tsunami</dt>
							<dd class="text-right">{quake.tsunami ? 'warning issued' : 'none'}</dd>
						</dl>
					</Card.Content>
				</Card.Root>

				<Card.Root>
					<Card.Header><Card.Title>Fault</Card.Title></Card.Header>
					<Card.Content>
						{#if detail.planes.length}
							<table class="w-full text-[13px]">
								<thead class="text-muted-foreground">
									<tr><th class="text-left font-normal">Plane</th><th class="text-right font-normal">Strike</th><th class="text-right font-normal">Dip</th><th class="text-right font-normal">Rake</th></tr>
								</thead>
								<tbody class="font-mono">
									{#each detail.planes as plane, i (i)}
										<tr><td>{i + 1}</td><td class="text-right">{plane.strike}°</td><td class="text-right">{plane.dip}°</td><td class="text-right">{plane.rake}°</td></tr>
									{/each}
								</tbody>
							</table>
							{#if detail.scalarMoment}
								<p class="mt-3 text-[12px] text-muted-foreground">Scalar moment <span class="font-mono text-foreground">{moment(detail.scalarMoment)}</span></p>
							{/if}
						{:else}
							<p class="text-muted-foreground">No moment tensor yet. USGS computes one for larger quakes, usually within the hour.</p>
						{/if}
					</Card.Content>
				</Card.Root>

				<Card.Root>
					<Card.Header><Card.Title>Nearby in this feed</Card.Title></Card.Header>
					<Card.Content>
						{#if nearby.length}
							<ul class="flex flex-col gap-1.5">
								{#each nearby as other (other.quake.id)}
									<li>
										<a href="/quake/{other.quake.id}" class="flex items-center gap-2 hover:text-primary">
											<MagBadge mag={other.quake.mag} size="sm" />
											<span class="min-w-0 flex-1 truncate">{other.quake.place}</span>
											<span class="font-mono text-[11px] text-muted-foreground">{Math.round(other.km)} km</span>
										</a>
									</li>
								{/each}
							</ul>
						{:else}
							<p class="text-muted-foreground">Nothing else within 500 km.</p>
						{/if}
					</Card.Content>
				</Card.Root>
			</div>
		</div>
	</div>
</div>
