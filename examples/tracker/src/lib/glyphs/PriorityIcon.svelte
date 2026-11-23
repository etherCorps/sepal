<!-- A priority as a 14px glyph: dashes, 1-3 signal bars, or the urgent square. currentColor only. -->
<script lang="ts">
	import { cn } from '$lib/utils';
	import type { PriorityId } from '$lib/seed';

	let { priority, size = 14, class: className = '' }: { priority: PriorityId; size?: number; class?: string } = $props();

	const filled = $derived(priority === 'high' ? 3 : priority === 'medium' ? 2 : 1);
	const colour = $derived(
		priority === 'urgent' ? 'text-priority-urgent' : priority === 'none' ? 'text-muted-foreground' : 'text-ink-2'
	);
</script>

<svg class={cn('shrink-0', colour, className)} width={size} height={size} viewBox="0 0 14 14" fill="currentColor" aria-hidden="true">
	{#if priority === 'none'}
		<rect x="1.5" y="6.25" width="2.5" height="1.5" rx="0.5" />
		<rect x="5.75" y="6.25" width="2.5" height="1.5" rx="0.5" />
		<rect x="10" y="6.25" width="2.5" height="1.5" rx="0.5" />
	{:else if priority === 'urgent'}
		<path
			fill-rule="evenodd"
			d="M4 1 H10 A3 3 0 0 1 13 4 V10 A3 3 0 0 1 10 13 H4 A3 3 0 0 1 1 10 V4 A3 3 0 0 1 4 1 Z M6.25 3.5 H7.75 V8 H6.25 Z M6.25 9.25 H7.75 V10.75 H6.25 Z"
		/>
	{:else}
		<rect x="1.5" y="8.5" width="3" height="4" rx="0.75" opacity={filled >= 1 ? 1 : 0.3} />
		<rect x="5.5" y="5.5" width="3" height="7" rx="0.75" opacity={filled >= 2 ? 1 : 0.3} />
		<rect x="9.5" y="2.5" width="3" height="10" rx="0.75" opacity={filled >= 3 ? 1 : 0.3} />
	{/if}
</svg>
