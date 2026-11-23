<!--
	One board card. A plain link: no Bits UI in here (the status Select is the
	board's, anchored to the focused card), no transition (300 of these).
-->
<script lang="ts">
	import { Avatar, LabelDot, PriorityIcon } from '$lib/glyphs';
	import type { Issue } from '$lib/seed';

	let { issue, focused }: { issue: Issue; focused: boolean } = $props();
</script>

<a
	data-testid="card"
	data-key={issue.key}
	href="/issues/{issue.key}"
	class={[
		'bg-card hover:border-input focus-visible:ring-ring flex shrink-0 flex-col gap-1.5 rounded-md border p-2.5 outline-none focus-visible:ring-2',
		focused && 'ring-ring ring-2'
	]}
>
	<div class="flex h-5 items-center justify-between">
		<span class="text-muted-foreground font-mono text-[11px]">{issue.key}</span>
		<Avatar id={issue.assignee} />
	</div>
	<div class="text-foreground max-h-[2.9em] overflow-hidden text-[13px] leading-[1.45] font-medium">{issue.title}</div>
	<div class="flex h-4 items-center gap-1.5">
		<PriorityIcon priority={issue.priority} />
		{#each issue.labelIds.slice(0, 2) as label (label)}<LabelDot {label} />{/each}
		{#if issue.estimate !== null}
			<span class="text-muted-foreground ml-auto font-mono text-[11px]">{issue.estimate}</span>
		{/if}
	</div>
</a>
