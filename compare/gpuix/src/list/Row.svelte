<!--
	One issue in the list (examples/tracker/src/lib/list/Row.svelte). Same cells, same sizes:
	checkbox, priority glyph, key, status glyph, title, up to two labels, avatar, date. The
	Tracker's CSS grid columns (16px 76px 16px 1fr auto 20px 56px, gap 10px) are a flex row here:
	this renderer's grid takes a column count only. The checkbox is a drawn box (there is no
	<input type="checkbox">), outside the clickable part as in the Tracker.
-->
<script module lang="ts">
	export interface Press {
		shift: boolean;
	}
</script>

<script lang="ts">
	import type { GpuixEvent } from 'gpuix-svelte';
	import { navigate } from 'gpuix-svelte-ui';
	import { short_date } from '../format.ts';
	import { PRIORITY_SVG, STATUS_SVG, avatar } from '../glyphs.ts';
	import { label_by_id, type Issue } from '../seed.ts';
	import { settings } from '../store.svelte.ts';
	import { mark } from '../ui.svelte.ts';

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
		oncheck?: (press: Press) => void;
		/** return false to stay put (a Shift+click extends the selection instead) */
		onopen?: (press: Press) => boolean | void;
	} = $props();

	const marked = $derived(selectable ? mark(issue.key) : null);
	const labels = $derived(settings.showLabels && !settings.compact && !narrow ? issue.labelIds.slice(0, 2) : []);
	const who = $derived(avatar(issue.assignee));
	const priority = $derived(PRIORITY_SVG[issue.priority]);
	const status = $derived(STATUS_SVG[issue.status]);

	const CHECK = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 14 14"><path d="M3.5 7.2L6 9.6L10.5 4.6" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>';

	const press = (e: GpuixEvent): Press => ({ shift: e.modifiers?.shift === true });
	function open(e: GpuixEvent) {
		if (onopen?.(press(e)) === false) return;
		void navigate(`/issues/${issue.key}`);
	}
</script>

<div
	class="row"
	class:compact={settings.compact}
	class:active={marked?.active}
	class:checked={!marked?.active && marked?.checked}
	class:plain={!marked?.active && !marked?.checked}
	{testid}
	data-key={issue.key}
>
	{#if selectable}<div class="check" class:on={marked?.checked} hitbox="self" onclick={(e) => oncheck?.(press(e))}>{#if marked?.checked}<svg class="tick" source={CHECK}></svg>{/if}</div>{/if}<div
		class="link"
		hitbox="self"
		testid={marked?.active ? 'row-active' : undefined}
		onclick={open}
	><svg class="glyph {priority.tone}" source={priority.source}></svg><text class="key">{issue.key}</text><svg class="glyph {status.tone}" source={status.source}></svg><text class="title">{issue.title}</text><div class="labels">{#each labels as label (label)}<div class="label"><div class="dot" style="background-color: var(--hue-{label_by_id[label]?.hue ?? 1})"></div><text>{label_by_id[label]?.name}</text></div>{/each}</div><div class="avatar" class:empty={who.empty} style={who.style}>{who.initials}</div><text class="date">{short_date(date ?? issue.updatedAt)}</text></div>
</div>

<style>
	.row {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-shrink: 0;
		gap: 10px;
		height: 36px;
		padding: 0 16px;
		border-width: 0 0 1px 0;
		border-style: solid;
		border-color: var(--border);
	}
	.row.compact { height: 28px; }
	.row.active { background-color: var(--row-active); }
	.row.checked { background-color: var(--row-checked); }
	.row.plain:hover { background-color: var(--row-hover); }
	.check {
		display: flex;
		width: 14px;
		height: 14px;
		margin: 0 1px;
		flex-shrink: 0;
		border-radius: 3px;
		border-width: 1px;
		border-style: solid;
		border-color: var(--input);
		background-color: var(--background);
		cursor: pointer;
	}
	.check.on { border-color: var(--primary); background-color: var(--primary); color: var(--primary-foreground); }
	.tick { width: 12px; height: 12px; }
	.link {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-grow: 1;
		flex-basis: 0;
		min-width: 0;
		height: 100%;
		gap: 10px;
		cursor: pointer;
	}
	.glyph { width: 14px; height: 14px; margin-right: 2px; flex-shrink: 0; }
	.muted-foreground { color: var(--muted-foreground); }
	.ink-2 { color: var(--ink-2); }
	.priority-urgent { color: var(--priority-urgent); }
	.status-backlog { color: var(--status-backlog); }
	.status-todo { color: var(--status-todo); }
	.status-progress { color: var(--status-progress); }
	.status-review { color: var(--status-review); }
	.status-done { color: var(--status-done); }
	.status-canceled { color: var(--status-canceled); }
	.key { width: 76px; flex-shrink: 0; font-family: JetBrains Mono; font-size: 12px; color: var(--muted-foreground); }
	.title {
		flex-grow: 1;
		flex-basis: 0;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-weight: 500;
		color: var(--foreground);
	}
	.labels { display: flex; flex-direction: row; align-items: center; flex-shrink: 0; gap: 12px; }
	.label { display: flex; flex-direction: row; align-items: center; gap: 6px; font-size: 11px; white-space: nowrap; color: var(--muted-foreground); }
	.dot { width: 6px; height: 6px; flex-shrink: 0; border-radius: 999px; }
	.avatar { display: flex; align-items: center; justify-content: center; flex-shrink: 0; border-radius: 999px; font-weight: 600; line-height: 10px; }
	.avatar.empty { border-width: 1px; border-style: dashed; border-color: var(--muted-foreground-60); }
	.date { width: 56px; flex-shrink: 0; text-align: right; font-family: JetBrains Mono; font-size: 11px; color: var(--muted-foreground); }
</style>
