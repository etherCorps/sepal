<!-- A status as a 14px circle glyph, coloured with currentColor (docs/tracker-design.md, Glyphs). -->
<script lang="ts">
	import { cn } from '$lib/utils';
	import type { StatusId } from '$lib/seed';
	import { STATUS_GLYPH } from './shapes';

	let { status, size = 14, class: className = '' }: { status: StatusId; size?: number; class?: string } = $props();

	const glyph = $derived(STATUS_GLYPH[status]);
</script>

<svg class={cn('shrink-0', glyph.class, className)} width={size} height={size} viewBox="0 0 14 14" aria-hidden="true"
	><path
		d={glyph.d}
		fill={glyph.fill ? 'currentColor' : 'none'}
		fill-rule="evenodd"
		stroke={glyph.fill ? 'none' : 'currentColor'}
		stroke-width="1.5"
		stroke-dasharray={glyph.dash}
	/></svg
>
