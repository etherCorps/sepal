<!--
	Created against closed, one pair of bars per week (examples/tracker/src/lib/insights/Chart.svelte).
	Same scale, ticks and bars. GPUI paints an <svg> as a mask in one colour, so the gridlines,
	the created bars and the closed bars are three <svg>s stacked in the same box, each in its
	colour. Positions the Tracker gives in % are worked out in px from the plot's width (onlayout),
	since `left` takes pixels only; the SVG is drawn at that width instead of stretched.
-->
<script lang="ts">
	import type { GpuixEvent } from 'gpuix-svelte';
	import { navigate } from 'gpuix-svelte-ui';
	import { count, short_date } from '../format.ts';
	import type { Week } from '../store.svelte.ts';

	let { weeks }: { weeks: Week[] } = $props();

	const H = 180;
	const BAR = 26;

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
	const every = $derived(n > 12 ? 2 : 1);

	const tallest = $derived({
		created: weeks.reduce((best, w, i) => (w.created > weeks[best].created ? i : best), 0),
		closed: weeks.reduce((best, w, i) => (w.closed > weeks[best].closed ? i : best), 0)
	});

	/** The plot's width in px; each week is 100 SVG units of it, as in the Tracker. */
	let width = $state(0);
	const unit = $derived(width / (n * 100));
	const slot = $derived(width / n);

	const svg = (body: string) => `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${H}" viewBox="0 0 ${width} ${H}">${body}</svg>`;
	const grid = $derived(svg(ticks.map((tick) => `<rect x="0" y="${Math.min(H - 1, y(tick))}" width="${width}" height="1" fill="currentColor"/>`).join('')));
	const created = $derived(svg(weeks.map((w, i) => `<rect x="${(i * 100 + 50 - BAR - 1) * unit}" y="${y(w.created)}" width="${BAR * unit}" height="${H - y(w.created)}" fill="currentColor"/>`).join('')));
	const closed = $derived(svg(weeks.map((w, i) => `<rect x="${(i * 100 + 51) * unit}" y="${y(w.closed)}" width="${BAR * unit}" height="${H - y(w.closed)}" fill="currentColor"/>`).join('')));
</script>

<div class="chart">
	{#each ticks as tick (tick)}
		<text class="tick" style="top: {16 + y(tick) - 6}px">{count(tick)}</text>
	{/each}

	<div class="plot" onlayout={(e: GpuixEvent) => (width = e.width ?? width)}>
		{#if width > 0}
			<svg class="layer border" source={grid}></svg>
			<svg class="layer chart-1" source={created}></svg>
			<svg class="layer chart-2" source={closed}></svg>
		{/if}

		{#each weeks as week, i (week.index)}
			{#if i === tallest.created && week.created > 0}
				<text class="value" style="left: {(i + 0.36) * slot - 24}px; top: {y(week.created) - 15}px">{count(week.created)}</text>
			{/if}
			{#if i === tallest.closed && week.closed > 0}
				<text class="value muted" style="left: {(i + 0.64) * slot - 24}px; top: {y(week.closed) - 15}px">{count(week.closed)}</text>
			{/if}
		{/each}

		{#each weeks as week, i (week.index)}
			<div class="week" testid="chart-week" style="left: {i * slot}px; width: {slot}px" onclick={() => navigate(`/issues?week=${week.index}`)}></div>
		{/each}
	</div>

	<div class="dates">
		{#each weeks as week, i (week.index)}
			{#if (n - 1 - i) % every === 0}
				<text class="date" style="left: {(i + 0.5 - every / 2) * slot}px; width: {every * slot}px">{short_date(week.start)}</text>
			{/if}
		{/each}
	</div>
</div>

<style>
	.chart { position: relative; display: flex; flex-direction: column; height: 220px; padding: 16px 0 0 40px; font-family: JetBrains Mono; font-size: 11px; }
	.tick { position: absolute; left: 0; width: 32px; text-align: right; line-height: 12px; color: var(--muted-foreground); }
	.plot { position: relative; height: 180px; flex-shrink: 0; }
	.layer { position: absolute; left: 0; top: 0; width: 100%; height: 100%; }
	.border { color: var(--border); }
	.chart-1 { color: var(--chart-1); }
	.chart-2 { color: var(--chart-2); }
	.value { position: absolute; width: 48px; text-align: center; line-height: 12px; color: var(--foreground); }
	.value.muted { color: var(--muted-foreground); }
	.week { position: absolute; top: 0; bottom: 0; cursor: pointer; }
	.week:hover { background-color: var(--foreground-4); }
	.dates { position: relative; height: 24px; }
	.date { position: absolute; top: 6px; text-align: center; line-height: 12px; color: var(--muted-foreground); }
</style>
