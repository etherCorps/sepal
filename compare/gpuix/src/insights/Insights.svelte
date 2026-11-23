<!--
	/insights (examples/tracker/src/routes/insights/+page.svelte): aggregates over every issue,
	all $derived in the store. The range tabs are gpuix-svelte-ui's Tabs (an underlined row, where
	the Tracker has a pill segmented control), the project filter its Select. No tile tooltips:
	gpuix-svelte-ui has no Tooltip, and gpuix-svelte's hugs its trigger, so the tiles would not fill
	their columns. Grid columns (2fr 1fr 1fr 1fr, 3fr 2fr, 140px 1fr 44px) are flex
	rows with the same proportions.
-->
<script lang="ts">
	import { Select, Tabs } from 'gpuix-svelte-ui';
	import Avatar from '../components/Avatar.svelte';
	import PageHeader from '../components/PageHeader.svelte';
	import PriorityIcon from '../components/PriorityIcon.svelte';
	import { count, delta, short_date } from '../format.ts';
	import Row from '../list/Row.svelte';
	import { EPOCH, PROJECTS, WEEK, priority_by_id } from '../seed.ts';
	import { data, insights_view, person } from '../store.svelte.ts';
	import Chart from './Chart.svelte';

	const stats = $derived(data.insights);
	const range = $derived(insights_view.range);
	const since = $derived(short_date(EPOCH - range * WEEK));

	const tiles = $derived([
		{ id: 'open', label: 'Open', value: count(stats.open), delta: `${delta(stats.delta.open)} since ${since}`, grow: 2 },
		{ id: 'created', label: 'Created', value: count(stats.created), delta: `${delta(stats.delta.created)} vs prior ${range}w`, grow: 1 },
		{ id: 'closed', label: 'Closed', value: count(stats.closed), delta: `${delta(stats.delta.closed)} vs prior ${range}w`, grow: 1 },
		{ id: 'age', label: 'Median age', value: `${stats.median_age_days}d`, delta: `${delta(stats.delta.median_age_days)}d since ${since}`, grow: 1 }
	]);

	const assignee_max = $derived(Math.max(1, ...stats.by_assignee.map((a) => a.count)));
	const priority_total = $derived(Math.max(1, stats.by_priority.reduce((sum, p) => sum + p.count, 0)));
	const segment = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)', 'var(--chart-5)'];

	const ALL = 'all';
	const project_options = [{ value: ALL, label: 'All projects' }, ...PROJECTS.map((p) => ({ value: p.key, label: p.name }))];
</script>

<PageHeader title="Insights">
	<Tabs
		testid="range"
		items={[4, 12, 26].map((weeks) => ({ value: String(weeks), label: `${weeks}w` }))}
		value={String(range)}
		onchange={(value) => (insights_view.range = Number(value) as 4 | 12 | 26)}
	/>
	<Select testid="insights-project" options={project_options} value={insights_view.project || ALL} onchange={(value) => (insights_view.project = value === ALL ? '' : value)} />
</PageHeader>

<div class="scroll">
	<div class="page">
		<div class="tiles">
			{#each tiles as tile (tile.id)}
				<div class="tile-cell" style="flex-grow: {tile.grow}">
					<div class="tile" testid="tile-{tile.id}">
						<text class="tile-label">{tile.label}</text>
						<text class="stat" class:big={tile.id === 'open'} testid="stat-{tile.id}">{tile.value}</text>
						<text class="tile-delta">{tile.delta}</text>
					</div>
				</div>
			{/each}
		</div>

		<div class="panel" testid="chart">
			<div class="chart-head">
				<text class="heading">Created against closed</text>
				<div class="legend">
					<div class="key"><div class="swatch" style="background-color: var(--chart-1)"></div><text>Created</text></div>
					<div class="key"><div class="swatch" style="background-color: var(--chart-2)"></div><text>Closed</text></div>
				</div>
			</div>
			<Chart weeks={stats.weeks} />
		</div>

		<div class="pair">
			<div class="panel half" style="flex-grow: 3" testid="by-assignee">
				<text class="heading spaced">Open by assignee</text>
				<div class="rows">
					{#each stats.by_assignee as row (row.person ?? 'none')}
						<div class="assignee">
							<div class="who"><Avatar id={row.person} /><text class="who-name">{row.person ? person(row.person)?.name : 'Unassigned'}</text></div>
							<div class="track"><div class="fill" style="width: {(row.count / assignee_max) * 100}%"></div></div>
							<text class="n">{count(row.count)}</text>
						</div>
					{/each}
				</div>
			</div>

			<div class="panel half" style="flex-grow: 2" testid="by-priority">
				<text class="heading spaced">By priority</text>
				<div class="stack">
					{#each stats.by_priority as row, i (row.priority)}
						<div style="height: 10px; width: {(row.count / priority_total) * 100}%; background-color: {segment[i]}"></div>
					{/each}
				</div>
				<div class="rows priorities">
					{#each stats.by_priority as row, i (row.priority)}
						<div class="priority">
							<div class="swatch" style="background-color: {segment[i]}"></div>
							<PriorityIcon priority={row.priority} />
							<text class="p-name">{priority_by_id[row.priority].name}</text>
							<text class="n">{count(row.count)}</text>
							<text class="pct">{Math.round((row.count / priority_total) * 100)}%</text>
						</div>
					{/each}
				</div>
			</div>
		</div>

		<div class="panel" testid="oldest">
			<text class="heading oldest-heading">Oldest open</text>
			<div class="bleed">
				{#each stats.oldest as issue (issue.key)}
					<Row {issue} selectable={false} testid="oldest-row" date={issue.createdAt} />
				{:else}
					<text class="none">No open issues.</text>
				{/each}
			</div>
		</div>
	</div>
</div>

<style>
	.scroll { display: flex; flex-direction: column; flex-grow: 1; flex-basis: 0; min-height: 0; overflow-y: scroll; scrollbar-color: var(--input); }
	.page { display: flex; flex-direction: column; gap: 24px; padding: 24px; max-width: 1120px; }
	.tiles { display: flex; flex-direction: row; gap: 16px; }
	.tile-cell { display: flex; flex-direction: column; flex-basis: 0; min-width: 0; }
	.tile {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 16px;
		border-radius: 8px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
		background-color: var(--card);
	}
	.tile-label { font-size: 12px; color: var(--muted-foreground); }
	.stat { font-family: Space Grotesk Light; font-size: 32px; line-height: 32px; font-weight: 500; color: var(--foreground); }
	.stat.big { font-size: 48px; line-height: 48px; }
	.tile-delta { margin-top: 4px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.panel {
		display: flex;
		flex-direction: column;
		min-width: 0;
		padding: 16px;
		border-radius: 8px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--border);
		background-color: var(--card);
	}
	.heading { font-size: 13px; font-weight: 500; color: var(--foreground); }
	.spaced { margin-bottom: 12px; }
	.chart-head { display: flex; flex-direction: row; align-items: center; margin-bottom: 12px; }
	.legend { display: flex; flex-direction: row; align-items: center; justify-content: flex-end; flex-grow: 1; gap: 16px; font-size: 12px; color: var(--muted-foreground); }
	.key { display: flex; flex-direction: row; align-items: center; gap: 6px; }
	.swatch { width: 8px; height: 8px; flex-shrink: 0; border-radius: 2px; }
	.pair { display: flex; flex-direction: row; align-items: start; gap: 24px; }
	.half { flex-basis: 0; }
	.rows { display: flex; flex-direction: column; gap: 8px; }
	.assignee { display: flex; flex-direction: row; align-items: center; gap: 12px; height: 24px; }
	.who { display: flex; flex-direction: row; align-items: center; gap: 8px; width: 140px; flex-shrink: 0; min-width: 0; }
	.who-name { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
	.track { display: flex; flex-direction: row; flex-grow: 1; flex-basis: 0; height: 8px; overflow: hidden; border-radius: 2px; background-color: var(--muted); }
	.fill { height: 100%; border-radius: 2px; background-color: var(--chart-1); }
	.n { width: 44px; flex-shrink: 0; text-align: right; font-family: JetBrains Mono; font-size: 12px; color: var(--muted-foreground); }
	.stack { display: flex; flex-direction: row; height: 10px; overflow: hidden; border-radius: 2px; background-color: var(--muted); }
	.priorities { margin-top: 16px; }
	.priority { display: flex; flex-direction: row; align-items: center; gap: 8px; height: 24px; }
	.p-name { flex-grow: 1; }
	.pct { width: 40px; flex-shrink: 0; text-align: right; font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
	.oldest-heading { margin-bottom: 8px; }
	.bleed { display: flex; flex-direction: column; margin: 0 -16px -16px -16px; }
	.none { padding: 8px 16px; font-size: 12px; color: var(--muted-foreground); }
</style>
