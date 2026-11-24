<!-- A label's 6px colour dot. -->
<script lang="ts">
	import { cn } from '#lib/utils.ts';
	import { label_by_id } from '#lib/seed.ts';

	let { label, class: className = '' }: { label: string; class?: string } = $props();
</script>

<span
	class={cn('inline-block size-1.5 shrink-0 rounded-full', className)}
	style="background: var(--hue-{label_by_id[label]?.hue ?? 1});"
></span>
