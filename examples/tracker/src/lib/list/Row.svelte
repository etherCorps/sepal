<!--
	One issue in the list (docs/tracker-design.md, Row). Plain elements only:
	A screenful of these mounts on every scroll, so no Bits UI and no transitions.
	The checkbox sits outside the link so a click on it never navigates.
	`selectable={false}` drops the checkbox column, and the highlight with it
	(Insights' oldest list).

	Every node here is paid for thousands of times, so the markup is written
	for the host: glyphs and the avatar inline (a component or {#if} adds a
	comment node), one <svg> path per glyph (glyphs/shapes.ts), and no
	whitespace between tags (each gap is a text node; a grid ignores it).
-->
<script lang="ts">
	import { short_date } from '#lib/format.ts';
	import { PRIORITY_GLYPH, STATUS_GLYPH, avatar } from '#lib/glyphs/index.ts';
	import { label_by_id, type Issue } from '#lib/seed.ts';
	import { settings } from '#lib/store.svelte.ts';
	import { mark } from '#lib/ui.svelte.ts';

	let {
		issue,
		selectable = true,
		narrow = false,
		testid = 'row',
		date,
		oncheck,
		onopen
	}: {
		issue: Issue;
		selectable?: boolean;
		testid?: string;
		/** the date at the end, in ms; the last update unless given */
		date?: number;
		/** the detail pane is open: labels give their room to the title */
		narrow?: boolean;
		oncheck?: (event: MouseEvent) => void;
		onopen?: (event: MouseEvent) => void;
	} = $props();

	// The j/k highlight and the checkbox, this row's alone (ui.svelte.ts).
	const marked = $derived(selectable ? mark(issue.key) : null);
	const labels = $derived(settings.showLabels && !settings.compact && !narrow ? issue.labelIds.slice(0, 2) : []);
	const who = $derived(avatar(issue.assignee));
</script>

<div
	data-testid={testid}
	data-key={issue.key}
	class={[
		'grid items-center gap-2.5 border-b px-4',
		selectable ? 'grid-cols-[16px_minmax(0,1fr)]' : 'grid-cols-[minmax(0,1fr)]',
		settings.compact ? 'h-7' : 'h-9',
		marked?.active ? 'bg-row-active' : marked?.checked ? 'bg-row-checked' : 'hover:bg-row-hover'
	]}
	><!-- Blitz ignores accent-color and paints the box in `color`, the outline too. --><input
		type="checkbox"
		class={['accent-primary size-3.5', marked?.checked ? 'text-primary' : 'text-input', !selectable && 'hidden']}
		checked={marked?.checked}
		aria-label="Select {issue.key}"
		onclick={oncheck}
	/><a
		href="/issues/{issue.key}"
		data-testid={marked?.active ? 'row-active' : undefined}
		class="ring-ring grid h-full min-w-0 grid-cols-[16px_76px_16px_minmax(0,1fr)_minmax(0,max-content)_20px_56px] items-center gap-2.5 outline-none focus-visible:ring-2 focus-visible:ring-inset"
		onclick={onopen}
		><svg class={['shrink-0', PRIORITY_GLYPH[issue.priority].class]} width="14" height="14" viewBox="0 0 14 14" fill="currentColor" fill-rule="evenodd" aria-hidden="true"
			><path d={PRIORITY_GLYPH[issue.priority].on} /><path d={PRIORITY_GLYPH[issue.priority].off} opacity="0.3" /></svg
		><span class="text-muted-foreground font-mono text-xs">{issue.key}</span
		><svg class={['shrink-0', STATUS_GLYPH[issue.status].class]} width="14" height="14" viewBox="0 0 14 14" aria-hidden="true"
			><path
				d={STATUS_GLYPH[issue.status].d}
				fill={STATUS_GLYPH[issue.status].fill ? 'currentColor' : 'none'}
				fill-rule="evenodd"
				stroke={STATUS_GLYPH[issue.status].fill ? 'none' : 'currentColor'}
				stroke-width="1.5"
				stroke-dasharray={STATUS_GLYPH[issue.status].dash}
			/></svg
		><span class="text-foreground truncate font-medium">{issue.title}</span
		><!-- Labels beside the title where the list is at least 760px wide (a 1000px window
			without the pane; the sidebar is 240px): below that, the title needs the room. A
			label that still does not fit drops out of sight whole, and never pushes the date
			out of the row. The box stays, so the avatar and date keep their columns.
		--><span class="flex h-4 min-w-0 flex-wrap items-center gap-x-3 gap-y-4 overflow-hidden"
			>{#each labels as label (label)}<span class="text-muted-foreground flex items-center gap-1.5 text-[11px] whitespace-nowrap max-[1000px]:hidden"
					><span class="inline-block size-1.5 shrink-0 rounded-full" style="background: var(--hue-{label_by_id[label]?.hue ?? 1});"></span
					>{label_by_id[label]?.name}</span
				>{/each}</span
		><span class={who.class} style={who.style} title={who.title}>{who.initials}</span
		><span class="text-muted-foreground text-right font-mono text-[11px]">{short_date(date ?? issue.updatedAt)}</span
	></a
></div>
