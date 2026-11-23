<!--
	One board card. A plain link: no Bits UI in here (the status Select is the
	board's, anchored to the focused card), no transition (300 of these).
	Written for few host nodes, as Row is: glyphs inline, no whitespace
	between tags.
-->
<script lang="ts">
	import { PRIORITY_GLYPH, avatar } from '$lib/glyphs';
	import { label_by_id, type Issue } from '$lib/seed';

	let { issue, focused }: { issue: Issue; focused: boolean } = $props();

	const priority = $derived(PRIORITY_GLYPH[issue.priority]);
	const who = $derived(avatar(issue.assignee));
</script>

<a
	data-testid="card"
	data-key={issue.key}
	href="/issues/{issue.key}"
	class={[
		'bg-card hover:border-input focus-visible:ring-ring flex shrink-0 flex-col gap-1.5 rounded-md border p-2.5 outline-none focus-visible:ring-2',
		focused && 'ring-ring ring-2'
	]}
	><div class="flex h-5 items-center justify-between"
		><span class="text-muted-foreground font-mono text-[11px]">{issue.key}</span
		><span class={who.class} style={who.style} title={who.title}>{who.initials}</span
	></div
	><div class="text-foreground max-h-[2.9em] overflow-hidden text-[13px] leading-[1.45] font-medium">{issue.title}</div
	><div class="flex h-4 items-center gap-1.5"
		><svg class={['shrink-0', priority.class]} width="14" height="14" viewBox="0 0 14 14" fill="currentColor" fill-rule="evenodd" aria-hidden="true"
			><path d={priority.on} /><path d={priority.off} opacity="0.3" /></svg
		>{#each issue.labelIds.slice(0, 2) as label (label)}<span
				class="inline-block size-1.5 shrink-0 rounded-full"
				style="background: var(--hue-{label_by_id[label]?.hue ?? 1});"
			></span>{/each}<span class="text-muted-foreground ml-auto font-mono text-[11px]">{issue.estimate ?? ''}</span
		></div
	></a
>
