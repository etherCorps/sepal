<!--
	One issue in the list (docs/tracker-design.md, Row). Plain elements only:
	200 to 20,000 of these are mounted, so no Bits UI and no transitions.
	The checkbox sits outside the link so a click on it never navigates.
	`selectable={false}` drops the checkbox column (Insights' oldest list).
-->
<script lang="ts">
	import { short_date } from '$lib/format';
	import { Avatar, LabelDot, PriorityIcon, StatusIcon } from '$lib/glyphs';
	import { label_by_id, type Issue } from '$lib/seed';
	import { settings } from '$lib/store.svelte';

	let {
		issue,
		active = false,
		selected = false,
		selectable = true,
		narrow = false,
		testid = 'row',
		date,
		oncheck,
		onopen
	}: {
		issue: Issue;
		/** the j/k highlight */
		active?: boolean;
		/** checked for the bulk bar */
		selected?: boolean;
		selectable?: boolean;
		testid?: string;
		/** the date at the end, in ms; the last update unless given */
		date?: number;
		/** the detail pane is open: labels give their room to the title */
		narrow?: boolean;
		oncheck?: (event: MouseEvent) => void;
		onopen?: (event: MouseEvent) => void;
	} = $props();

	const labels = $derived(settings.showLabels && !settings.compact && !narrow ? issue.labelIds.slice(0, 2) : []);
</script>

<div
	data-testid={testid}
	data-key={issue.key}
	class={[
		'grid items-center gap-2.5 border-b px-4',
		selectable ? 'grid-cols-[16px_minmax(0,1fr)]' : 'grid-cols-[minmax(0,1fr)]',
		settings.compact ? 'h-7' : 'h-9',
		active ? 'bg-row-active' : selected ? 'bg-row-checked' : 'hover:bg-row-hover'
	]}
>
	{#if selectable}
		<!-- Blitz ignores accent-color and paints the box in `color`, the outline too. -->
		<input type="checkbox" class={['accent-primary size-3.5', selected ? 'text-primary' : 'text-input']} checked={selected} aria-label="Select {issue.key}" onclick={oncheck} />
	{/if}
	<a
		href="/issues/{issue.key}"
		data-testid={active ? 'row-active' : undefined}
		class="ring-ring grid h-full min-w-0 grid-cols-[16px_76px_16px_minmax(0,1fr)_auto_20px_56px] items-center gap-2.5 outline-none focus-visible:ring-2 focus-visible:ring-inset"
		onclick={onopen}
	>
		<PriorityIcon priority={issue.priority} />
		<span class="text-muted-foreground font-mono text-xs">{issue.key}</span>
		<StatusIcon status={issue.status} />
		<span class="text-foreground truncate font-medium">{issue.title}</span>
		<span class="flex items-center gap-3">
			{#each labels as label (label)}
				<span class="text-muted-foreground flex items-center gap-1.5 text-[11px] whitespace-nowrap"><LabelDot {label} />{label_by_id[label]?.name}</span>
			{/each}
		</span>
		<Avatar id={issue.assignee} />
		<span class="text-muted-foreground text-right font-mono text-[11px]">{short_date(date ?? issue.updatedAt)}</span>
	</a>
</div>
