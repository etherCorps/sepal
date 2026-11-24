<!--
	The feed's period as a seismograph strip: one stem per quake at the time it
	struck, as tall as its magnitude. The selected one is drawn white.
-->
<script lang="ts">
	import { PALETTE } from '#lib/geo.ts';
	import { band, type PresetId, type Quake } from '#lib/usgs.ts';
	import { datetime } from '#lib/format.ts';

	let { quakes, preset, now, selected }: { quakes: Quake[]; preset: PresetId; now: number; selected: string | null } = $props();

	const PERIOD = { hour: 3600_000, day: 86_400_000, week: 7 * 86_400_000, month: 30 * 86_400_000 };
	const W = 1000;
	const H = 120;
	const BASE = H - 4;

	const span = $derived(PERIOD[preset.split('_')[1] as keyof typeof PERIOD]);
	const start = $derived(now - span);
	const stems = $derived(
		quakes.map((quake) => ({
			id: quake.id,
			x: Math.max(0, Math.min(W, ((quake.time - start) / span) * W)),
			top: BASE - (Math.max(quake.mag, 0) / 8) * (BASE - 6),
			colour: PALETTE.mag[band(quake.mag)]
		}))
	);
	const mark = $derived(stems.find((stem) => stem.id === selected));
</script>

<div class="flex flex-col gap-1.5">
	<svg class="h-[120px] w-full" viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true">
		{#each [2, 4, 6] as m (m)}
			<line x1="0" x2={W} y1={BASE - (m / 8) * (BASE - 6)} y2={BASE - (m / 8) * (BASE - 6)} stroke={PALETTE.grid} stroke-width="1" />
		{/each}
		<line x1="0" x2={W} y1={BASE} y2={BASE} stroke={PALETTE.coast} stroke-width="1" />
		{#each stems as stem (stem.id)}
			<line x1={stem.x} x2={stem.x} y1={BASE} y2={stem.top} stroke={stem.colour} stroke-width="2" opacity="0.85" />
		{/each}
		{#if mark}
			<line x1={mark.x} x2={mark.x} y1={BASE} y2={mark.top} stroke="white" stroke-width="3" />
		{/if}
	</svg>
	<div class="flex justify-between font-mono text-[11px] text-muted-foreground">
		<span>{datetime(start)}</span>
		<span>guides at M2, M4, M6</span>
		<span>now</span>
	</div>
</div>
