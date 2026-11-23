<!-- A status as a 14px circle glyph (glyphs.ts). -->
<script lang="ts">
	import { STATUS_SVG } from '../glyphs.ts';
	import type { StatusId } from '../seed.ts';

	let { status, size = 14 }: { status: StatusId; size?: number } = $props();
	const glyph = $derived(STATUS_SVG[status]);
</script>

<svg class="glyph {glyph.tone}" source={glyph.source} style="width: {size}px; height: {size}px"></svg>

<style>
	.glyph { flex-shrink: 0; }
	.status-backlog { color: var(--status-backlog); }
	.status-todo { color: var(--status-todo); }
	.status-progress { color: var(--status-progress); }
	.status-review { color: var(--status-review); }
	.status-done { color: var(--status-done); }
	.status-canceled { color: var(--status-canceled); }
</style>
