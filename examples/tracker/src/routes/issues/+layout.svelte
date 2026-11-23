<!--
	/issues and /issues/[key]: the list stays mounted, and the detail pane
	(the child page) opens at its right.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import IssueList from '$lib/list/IssueList.svelte';

	let { children }: { children: Snippet } = $props();

	const pane = $derived(page.url.pathname !== '/issues');
</script>

<div class={['grid min-h-0 flex-1', pane ? 'grid-cols-[minmax(0,1fr)_480px]' : 'grid-cols-[minmax(0,1fr)]']}>
	<div class="flex min-h-0 min-w-0 flex-col">
		<IssueList title="Issues" />
	</div>
	{#if pane}
		<section data-testid="pane" class="bg-card flex min-h-0 flex-col overflow-y-scroll border-l">
			{@render children()}
		</section>
	{/if}
</div>
