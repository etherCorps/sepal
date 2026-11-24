<script lang="ts">
	import { band } from '#lib/usgs.ts';
	import { mag as format } from '#lib/format.ts';
	import { cn } from '#lib/utils.ts';

	let { mag, size = 'md', class: className = '' }: { mag: number; size?: 'sm' | 'md' | 'lg'; class?: string } = $props();

	const tone = {
		1: 'bg-mag-1/15 text-mag-1 border-mag-1/40',
		2: 'bg-mag-2/15 text-mag-2 border-mag-2/40',
		3: 'bg-mag-3/15 text-mag-3 border-mag-3/40',
		4: 'bg-mag-4/15 text-mag-4 border-mag-4/40',
		5: 'bg-mag-5/20 text-mag-5 border-mag-5/50'
	};
	const sizes = {
		sm: 'h-6 min-w-10 px-1.5 text-[12px]',
		md: 'h-7 min-w-12 px-2 text-[13px]',
		lg: 'h-14 min-w-20 px-3 text-[28px]'
	};
</script>

<span
	class={cn(
		'inline-flex shrink-0 items-center justify-center rounded-md border font-mono font-medium tabular-nums',
		tone[band(mag)],
		sizes[size],
		className
	)}>{format(mag)}</span
>
