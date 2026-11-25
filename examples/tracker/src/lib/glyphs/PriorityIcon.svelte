<!-- A priority as a 14px glyph: dashes, 1-3 signal bars, or the urgent square. currentColor only. -->
<script lang="ts">
	import { cn } from '#lib/utils.ts';
	import type { PriorityId } from '#lib/seed.ts';
	import { PRIORITY_GLYPH } from './shapes';

	let { priority, size = 14, class: className = '' }: { priority: PriorityId; size?: number; class?: string } = $props();

	const glyph = $derived(PRIORITY_GLYPH[priority]);
</script>

<svg class={cn('shrink-0', glyph.class, className)} width={size} height={size} viewBox="0 0 14 14" fill="currentColor" fill-rule="evenodd" aria-hidden="true"
	><path d={glyph.on} /><path d={glyph.off} opacity="0.3" /></svg
>
