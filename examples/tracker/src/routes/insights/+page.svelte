<!--
	/insights: aggregates over every issue, all $derived in the store
	(`data.insights`, driven by `insights_view`). docs/tracker-design.md, Insights.
-->
<script lang="ts">
	import PageHeader from '#lib/components/PageHeader.svelte';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '#lib/components/ui/select/index.ts';
	import { Tabs, TabsList, TabsTrigger } from '#lib/components/ui/tabs/index.ts';
	import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '#lib/components/ui/tooltip/index.ts';
	import Chart from '#lib/insights/Chart.svelte';
	import Row from '#lib/list/Row.svelte';
	import { count, delta, short_date } from '#lib/format.ts';
	import { Avatar, PriorityIcon, ProjectSquare } from '#lib/glyphs/index.ts';
	import { EPOCH, PROJECTS, WEEK, priority_by_id, project_by_key } from '#lib/seed.ts';
	import { data, insights_view, person } from '#lib/store.svelte.ts';

	const stats = $derived(data.insights);
	const range = $derived(insights_view.range);
	const since = $derived(short_date(EPOCH - range * WEEK));

	const tiles = $derived([
		{
			id: 'open',
			label: 'Open',
			value: count(stats.open),
			delta: `${delta(stats.delta.open)} since ${since}`,
			help: 'Issues not yet done or canceled, today.'
		},
		{
			id: 'created',
			label: 'Created',
			value: count(stats.created),
			delta: `${delta(stats.delta.created)} vs prior ${range}w`,
			help: `Issues opened in the last ${range} weeks.`
		},
		{
			id: 'closed',
			label: 'Closed',
			value: count(stats.closed),
			delta: `${delta(stats.delta.closed)} vs prior ${range}w`,
			help: `Issues done or canceled in the last ${range} weeks.`
		},
		{
			id: 'age',
			label: 'Median age',
			value: `${stats.median_age_days}d`,
			delta: `${delta(stats.delta.median_age_days)}d since ${since}`,
			help: 'Half the open issues are older than this.'
		}
	]);

	const assignee_max = $derived(Math.max(1, ...stats.by_assignee.map((a) => a.count)));
	const priority_total = $derived(Math.max(1, stats.by_priority.reduce((sum, p) => sum + p.count, 0)));
	/** chart-1 (cobalt) for urgent, down to chart-5. Literal names so Tailwind sees them. */
	const segment = ['bg-chart-1', 'bg-chart-2', 'bg-chart-3', 'bg-chart-4', 'bg-chart-5'];

	// Bits UI treats '' as nothing chosen; the store uses '' for every project.
	const ALL = 'all';
	const heading = 'text-foreground text-[13px] font-medium';
	const panel = 'bg-card rounded-lg border p-4';
</script>

<PageHeader title="Insights">
	<Tabs value={String(range)} onValueChange={(value) => (insights_view.range = Number(value) as 4 | 12 | 26)}>
		<TabsList class="h-7 p-0.5">
			{#each [4, 12, 26] as weeks (weeks)}
				<TabsTrigger data-testid="range-{weeks}" value={String(weeks)} class="px-2.5 font-mono text-[11px]">{weeks}w</TabsTrigger>
			{/each}
		</TabsList>
	</Tabs>
	<Select
		type="single"
		bind:value={() => insights_view.project || ALL, (value) => (insights_view.project = value === ALL ? '' : value)}
	>
		<SelectTrigger data-testid="insights-project" size="sm" class="h-7 gap-1.5 px-2 text-[13px] data-[size=sm]:h-7">
			{#if insights_view.project}
				<ProjectSquare project={insights_view.project} />{project_by_key[insights_view.project].name}
			{:else}
				All projects
			{/if}
		</SelectTrigger>
		<SelectContent align="end">
			<SelectItem value={ALL} label="All projects">All projects</SelectItem>
			{#each PROJECTS as project (project.key)}
				<SelectItem value={project.key} label={project.name}><ProjectSquare project={project.key} />{project.name}</SelectItem>
			{/each}
		</SelectContent>
	</Select>
</PageHeader>

<div class="min-h-0 flex-1 overflow-y-scroll">
	<div class="mx-auto flex max-w-[1120px] flex-col gap-6 p-6">
		<TooltipProvider delayDuration={800}>
			<div class="grid grid-cols-[2fr_1fr_1fr_1fr] gap-4">
				{#each tiles as tile (tile.id)}
					<Tooltip>
						<TooltipTrigger>
							{#snippet child({ props })}
								<div {...props} data-testid="tile-{tile.id}" class="bg-card flex flex-col gap-1 rounded-lg border p-4">
									<span class="text-muted-foreground text-xs">{tile.label}</span>
									<span
										data-testid="stat-{tile.id}"
										class={[
											'font-display text-foreground leading-none font-medium tracking-[-0.02em] tabular-nums',
											tile.id === 'open' ? 'text-[48px]' : 'text-[32px]'
										]}>{tile.value}</span
									>
									<span class="text-muted-foreground mt-1 truncate font-mono text-[11px]">{tile.delta}</span>
								</div>
							{/snippet}
						</TooltipTrigger>
						<TooltipContent>{tile.help}</TooltipContent>
					</Tooltip>
				{/each}
			</div>
		</TooltipProvider>

		<section data-testid="chart" class={panel}>
			<div class="mb-3 flex items-center">
				<h2 class={heading}>Created against closed</h2>
				<div class="text-muted-foreground ml-auto flex items-center gap-4 text-xs">
					<span class="flex items-center gap-1.5"><span class="bg-chart-1 size-2 rounded-[2px]"></span>Created</span>
					<span class="flex items-center gap-1.5"><span class="bg-chart-2 size-2 rounded-[2px]"></span>Closed</span>
				</div>
			</div>
			<Chart weeks={stats.weeks} />
		</section>

		<div class="grid grid-cols-[3fr_2fr] items-start gap-6">
			<section data-testid="by-assignee" class={panel}>
				<h2 class="{heading} mb-3">Open by assignee</h2>
				<div class="flex flex-col gap-2">
					{#each stats.by_assignee as row (row.person ?? 'none')}
						<div class="grid h-6 grid-cols-[140px_minmax(0,1fr)_44px] items-center gap-3">
							<span class="flex min-w-0 items-center gap-2">
								<Avatar id={row.person} />
								<span class="truncate">{row.person ? person(row.person)?.name : 'Unassigned'}</span>
							</span>
							<span class="bg-muted h-2 overflow-hidden rounded-sm">
								<span class="bg-chart-1 block h-full rounded-sm" style="width: {(row.count / assignee_max) * 100}%"></span>
							</span>
							<span class="text-muted-foreground text-right font-mono text-xs">{count(row.count)}</span>
						</div>
					{/each}
				</div>
			</section>

			<section data-testid="by-priority" class={panel}>
				<h2 class="{heading} mb-3">By priority</h2>
				<div class="bg-muted flex h-2.5 overflow-hidden rounded-sm">
					{#each stats.by_priority as row, i (row.priority)}
						<span class={segment[i]} style="width: {(row.count / priority_total) * 100}%"></span>
					{/each}
				</div>
				<div class="mt-4 flex flex-col gap-2">
					{#each stats.by_priority as row, i (row.priority)}
						<div class="flex h-6 items-center gap-2">
							<span class="{segment[i]} size-2 rounded-[2px]"></span>
							<PriorityIcon priority={row.priority} />
							<span>{priority_by_id[row.priority].name}</span>
							<span class="text-muted-foreground ml-auto font-mono text-xs">{count(row.count)}</span>
							<span class="text-muted-foreground w-10 text-right font-mono text-[11px]">{Math.round((row.count / priority_total) * 100)}%</span>
						</div>
					{/each}
				</div>
			</section>
		</div>

		<section data-testid="oldest" class={panel}>
			<h2 class="{heading} mb-2">Oldest open</h2>
			<!-- Full bleed, and the last row's hairline is the panel's edge. -->
			<div class="-mx-4 -mb-4 [&>*:last-child]:border-b-0">
				{#each stats.oldest as issue (issue.key)}
					<Row {issue} selectable={false} testid="oldest-row" date={issue.createdAt} />
				{:else}
					<p class="text-muted-foreground px-4 py-2 text-xs">No open issues.</p>
				{/each}
			</div>
		</section>
	</div>
</div>
