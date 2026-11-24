<!--
	The flat map. The land is one static SVG; each quake is a button laid over
	it at its projected position, in percent, so the dots follow the map at any
	window size and each can take a click on its own.
-->
<script lang="ts">
	import { MAP, PALETTE, place } from '#lib/geo.ts';
	import { band, type Quake } from '#lib/usgs.ts';
	import { mag } from '#lib/format.ts';

	let { quakes, selected, onselect }: { quakes: Quake[]; selected: string | null; onselect: (id: string) => void } =
		$props();

	const fill = {
		1: 'bg-mag-1/80',
		2: 'bg-mag-2/80',
		3: 'bg-mag-3/85',
		4: 'bg-mag-4/90',
		5: 'bg-mag-5'
	};

	/** Small first, so the big ones are drawn on top; the selected one last of all. */
	const dots = $derived(
		[...quakes]
			.sort((a, b) => (a.id === selected ? 1 : b.id === selected ? -1 : a.mag - b.mag))
			.map((quake) => {
				const { x, y } = place(quake.lon, quake.lat);
				const radius = 2 + Math.max(quake.mag, 0) ** 2 * 0.25;
				return { quake, left: x * 100, top: y * 100, radius };
			})
	);
</script>

<div class="relative w-full" style="aspect-ratio: {MAP.width} / {MAP.height}">
	<svg class="absolute inset-0 h-full w-full" viewBox={MAP.viewBox} aria-hidden="true">
		<path d={MAP.sphere} fill={PALETTE.ocean} stroke={PALETTE.coast} stroke-width="1" />
		<path d={MAP.graticule} fill="none" stroke={PALETTE.grid} stroke-width="0.6" />
		<path d={MAP.land} fill={PALETTE.land} />
		<path d={MAP.borders} fill="none" stroke={PALETTE.coast} stroke-width="0.5" />
	</svg>

	{#each dots as { quake, left, top, radius } (quake.id)}
		<button
			type="button"
			aria-label="M{mag(quake.mag)}, {quake.place}"
			class="absolute rounded-full border border-black/40 {fill[band(quake.mag)]} hover:border-white {quake.id === selected
				? 'border-2 border-white'
				: ''}"
			style="left: {left}%; top: {top}%; width: {radius * 2}px; height: {radius * 2}px; margin-left: {-radius}px; margin-top: {-radius}px"
			onclick={() => onselect(quake.id)}
		></button>
	{/each}
</div>
