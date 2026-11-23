<!-- A key hint: `⌘K`, `C`, `/`. Mono 11, bordered, on muted. -->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils';

	let { children, class: className = '' }: { children: Snippet; class?: string } = $props();
</script>

<kbd class={cn('bg-muted text-muted-foreground rounded border px-1.5 font-mono text-[11px] leading-[18px] font-medium', className)}
	>{@render children()}</kbd
>
