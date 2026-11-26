<!--
	The 48px bar at the top of every screen: title, an optional count in mono,
	and whatever the screen puts at the right (`children`).
	The shell has no header of its own; each screen renders one of these.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, count, children }: { title: string; count?: string; children?: Snippet } = $props();
</script>

<header class="flex h-12 shrink-0 items-center gap-3 border-b px-5">
	<h1 data-testid="page-title" class="font-display text-foreground text-[18px] font-medium tracking-[-0.02em]">{title}</h1>
	{#if count !== undefined}
		<span data-testid="page-count" class="text-muted-foreground font-mono text-xs">{count}</span>
	{/if}
	<div class="ml-auto flex items-center gap-2">
		{@render children?.()}
	</div>
</header>
