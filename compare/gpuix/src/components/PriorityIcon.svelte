<!-- A priority as a 14px glyph: dashes, 1-3 signal bars, or the urgent square (glyphs.ts). -->
<script lang="ts">
	import { PRIORITY_SVG } from '../glyphs.ts';
	import type { PriorityId } from '../seed.ts';

	let { priority, size = 14 }: { priority: PriorityId; size?: number } = $props();
	const glyph = $derived(PRIORITY_SVG[priority]);
</script>

<svg class="glyph {glyph.tone}" source={glyph.source} style="width: {size}px; height: {size}px"></svg>

<style>
	.glyph { flex-shrink: 0; }
	.muted-foreground { color: var(--muted-foreground); }
	.ink-2 { color: var(--ink-2); }
	.priority-urgent { color: var(--priority-urgent); }
</style>
