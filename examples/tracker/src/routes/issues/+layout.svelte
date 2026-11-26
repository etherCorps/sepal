<!--
	/issues and /issues/[key]: the list stays mounted, and the detail pane
	(the child page) opens at its right, when there is room for both: the
	sidebar (240px), a list of at least 400px (the toolbar's search, Status
	and Priority, a row's title, the bulk bar's actions) and the pane (480px),
	1120px in all. In a narrower window the pane takes the list's place until
	it is closed. Each column keeps what it draws inside itself.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import IssueList from '#lib/list/IssueList.svelte';

	let { children }: { children: Snippet } = $props();

	const pane = $derived(page.url.pathname !== '/issues');
</script>

<div class={['grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)]', pane && 'min-[1120px]:grid-cols-[minmax(0,1fr)_480px]']}>
	<div class={['flex min-h-0 min-w-0 flex-col overflow-hidden', pane && 'max-[1120px]:hidden']}>
		<IssueList title="Issues" />
	</div>
	{#if pane}
		<section data-testid="pane" class="bg-card flex min-h-0 min-w-0 flex-col overflow-y-scroll min-[1120px]:border-l">
			{@render children()}
		</section>
	{/if}
</div>
