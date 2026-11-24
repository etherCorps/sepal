<!--
	Created against closed, one pair of bars per week (docs/tracker-design.md,
	Insights). The SVG draws only bars and gridlines, in currentColor: usvg
	sees no @font-face fonts and does not hit test, so the labels and the
	click targets are HTML laid over it. The SVG stretches to the box
	(preserveAspectRatio none); its height matches the box, so hairlines stay 1px.
-->
<script lang="ts">
	import { count, short_date } from '#lib/format.ts';
	import type { Week } from '#lib/store.svelte.ts';

	let { weeks }: { weeks: Week[] } = $props();

	/** Plot height in px and SVG units alike. */
	const H = 180;
	/** Each week is 100 units wide; the bars sit in the middle 56. */
	const BAR = 26;

	/** A round step that splits `max` into about 4 bands. */
	function nice_step(max: number): number {
		if (max <= 0) return 1;
		const raw = max / 4;
		const power = 10 ** Math.floor(Math.log10(raw));
		const m = raw / power;
		return (m <= 1 ? 1 : m <= 2 ? 2 : m <= 5 ? 5 : 10) * power;
	}

	const max = $derived(Math.max(0, ...weeks.map((w) => Math.max(w.created, w.closed))));
	const step = $derived(nice_step(max));
	const top = $derived(Math.max(step, Math.ceil(max / step) * step));
	const ticks = $derived(Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step));
	const y = (value: number) => H - (value / top) * H;

	const n = $derived(weeks.length);
	const slot = $derived(100 / n);
	/** At 26 weeks a date would not fit under every pair. */
	const every = $derived(n > 12 ? 2 : 1);

	// Values go above the tallest bar of each series only.
	const tallest = $derived({
		created: weeks.reduce((best, w, i) => (w.created > weeks[best].created ? i : best), 0),
		closed: weeks.reduce((best, w, i) => (w.closed > weeks[best].closed ? i : best), 0)
	});
</script>

<div class="relative h-[220px] pt-4 pl-10 font-mono text-[11px]">
	<!-- y axis labels, right-aligned in the 40px gutter -->
	{#each ticks as tick (tick)}
		<span class="text-muted-foreground absolute left-0 w-8 text-right leading-3" style="top: {16 + y(tick) - 6}px">{count(tick)}</span>
	{/each}

	<div class="relative h-[180px]">
		<svg class="absolute inset-0 h-full w-full" viewBox="0 0 {n * 100} {H}" preserveAspectRatio="none" aria-hidden="true">
			<g class="text-border" stroke="currentColor" stroke-width="1" shape-rendering="crispEdges">
				{#each ticks as tick (tick)}
					<line x1="0" x2={n * 100} y1={Math.min(H - 0.5, y(tick) + 0.5)} y2={Math.min(H - 0.5, y(tick) + 0.5)} />
				{/each}
			</g>
			{#each weeks as week, i (week.index)}
				<rect class="text-chart-1" fill="currentColor" x={i * 100 + 50 - BAR - 1} y={y(week.created)} width={BAR} height={H - y(week.created)} />
				<rect class="text-chart-2" fill="currentColor" x={i * 100 + 51} y={y(week.closed)} width={BAR} height={H - y(week.closed)} />
			{/each}
		</svg>

		{#each weeks as week, i (week.index)}
			{#if i === tallest.created && week.created > 0}
				<span class="text-foreground absolute w-12 text-center leading-3" style="left: calc({(i + 0.36) * slot}% - 24px); top: {y(week.created) - 15}px"
					>{count(week.created)}</span
				>
			{/if}
			{#if i === tallest.closed && week.closed > 0}
				<span class="text-muted-foreground absolute w-12 text-center leading-3" style="left: calc({(i + 0.64) * slot}% - 24px); top: {y(week.closed) - 15}px"
					>{count(week.closed)}</span
				>
			{/if}
		{/each}

		<!-- One link per week over the bars: SVG children take no clicks in Blitz. -->
		{#each weeks as week, i (week.index)}
			<a
				data-testid="chart-week"
				data-week={week.index}
				href="/issues?week={week.index}"
				aria-label="Issues created in the week of {short_date(week.start)}"
				class="hover:bg-foreground/[0.04] absolute top-0 bottom-0"
				style="left: {i * slot}%; width: {slot}%"
			></a>
		{/each}
	</div>

	<div class="relative h-6">
		{#each weeks as week, i (week.index)}
			{#if (n - 1 - i) % every === 0}
				<span class="text-muted-foreground absolute top-1.5 text-center leading-3" style="left: {(i + 0.5 - every / 2) * slot}%; width: {every * slot}%"
					>{short_date(week.start)}</span
				>
			{/if}
		{/each}
	</div>
</div>
