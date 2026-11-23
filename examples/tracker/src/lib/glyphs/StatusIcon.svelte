<!-- A status as a 14px circle glyph, coloured with currentColor (docs/tracker-design.md, Glyphs). -->
<script lang="ts">
	import { cn } from '$lib/utils';
	import type { StatusId } from '$lib/seed';

	let { status, size = 14, class: className = '' }: { status: StatusId; size?: number; class?: string } = $props();

	const colour: Record<StatusId, string> = {
		backlog: 'text-status-backlog',
		todo: 'text-status-todo',
		progress: 'text-status-progress',
		review: 'text-status-review',
		done: 'text-status-done',
		canceled: 'text-status-canceled'
	};
</script>

<svg
	class={cn('shrink-0', colour[status], className)}
	width={size}
	height={size}
	viewBox="0 0 14 14"
	fill="none"
	stroke="currentColor"
	stroke-width="1.5"
	aria-hidden="true"
>
	{#if status === 'backlog'}
		<circle cx="7" cy="7" r="5.5" stroke-dasharray="2.2 1.9" />
	{:else if status === 'todo'}
		<circle cx="7" cy="7" r="5.5" />
	{:else if status === 'progress'}
		<circle cx="7" cy="7" r="5.5" />
		<path d="M7 4 A3 3 0 0 1 7 10 Z" fill="currentColor" stroke="none" />
	{:else if status === 'review'}
		<circle cx="7" cy="7" r="5.5" />
		<path d="M7 4 A3 3 0 1 1 4 7 L7 7 Z" fill="currentColor" stroke="none" />
	{:else if status === 'done'}
		<!-- the check is a hole (evenodd), so it shows whatever is behind -->
		<path
			fill="currentColor"
			stroke="none"
			fill-rule="evenodd"
			d="M1 7 A6 6 0 1 0 13 7 A6 6 0 1 0 1 7 Z M3.67 7.73 L6.2 10.25 L10.35 5.92 L9.25 4.88 L6.2 8.1 L4.73 6.67 Z"
		/>
	{:else}
		<circle cx="7" cy="7" r="5.5" />
		<path d="M5 5 L9 9 M9 5 L5 9" />
	{/if}
</svg>
