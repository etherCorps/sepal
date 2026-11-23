<!--
	The 48px bar at the top of every screen: title, an optional count in mono, and whatever the
	screen puts at the right (`children`).
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, count, children }: { title: string; count?: string; children?: Snippet } = $props();
</script>

<div class="header">
	<text class="title" testid="page-title">{title}</text>
	{#if count !== undefined}
		<text class="count" testid="page-count">{count}</text>
	{/if}
	<div class="right">
		{@render children?.()}
	</div>
</div>

<style>
	.header {
		display: flex;
		flex-direction: row;
		align-items: center;
		flex-shrink: 0;
		gap: 12px;
		height: 48px;
		padding: 0 20px;
		border-width: 0 0 1px 0;
		border-style: solid;
		border-color: var(--border);
	}
	.title { font-family: Space Grotesk Light; font-size: 18px; font-weight: 500; color: var(--foreground); }
	.count { font-family: JetBrains Mono; font-size: 12px; color: var(--muted-foreground); }
	.right { display: flex; flex-direction: row; align-items: center; justify-content: flex-end; flex-grow: 1; gap: 8px; }
</style>
