<!--
	Renders the current view: each layout around the next one, the page innermost.
	A layout keeps its instance (and its state) for as long as it stays in the
	chain; only what is below a change is replaced.
-->
<!-- Without a rune in the file Svelte would compile it in legacy mode, where a
     component tag is resolved once and never switches. -->
<svelte:options runes={true} />

<script>
	import { page, router } from './routing.svelte.js';
</script>

{#snippet level(index)}
	{@const view = router.view}
	{#if index < view.layouts.length}
		{@const Layout = view.layouts[index].component}
		{#if Layout}
			<Layout data={view.data[index]} params={page.params}>
				{@render level(index + 1)}
			</Layout>
		{:else}
			{@render level(index + 1)}
		{/if}
	{:else}
		<view.page data={view.page_data} params={page.params} />
	{/if}
{/snippet}

{#if router.view}
	{@render level(0)}
{/if}
