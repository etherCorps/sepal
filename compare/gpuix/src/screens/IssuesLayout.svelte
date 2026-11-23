<!--
	/issues and /issues/:key (examples/tracker/src/routes/issues/+layout.svelte): the list stays
	mounted, and the detail pane opens at its right. App.svelte renders this from the Router's
	layout, outside the Router's per-path {#key}, so neither the list nor the pane remounts when
	the key changes, as with the Tracker's router.
-->
<script lang="ts">
	import Detail from '../detail/Detail.svelte';
	import IssueList from '../list/IssueList.svelte';

	let { key }: { key: string | null } = $props();
</script>

<div class="split">
	<div class="list"><IssueList title="Issues" /></div>
	{#if key !== null}
		<div class="pane" testid="pane"><Detail {key} /></div>
	{/if}
</div>

<style>
	.split { display: flex; flex-direction: row; flex-grow: 1; flex-basis: 0; min-height: 0; }
	.list { display: flex; flex-direction: column; flex-grow: 1; flex-basis: 0; min-width: 0; min-height: 0; }
	.pane {
		display: flex;
		flex-direction: column;
		width: 480px;
		flex-shrink: 0;
		min-height: 0;
		overflow-y: scroll;
		scrollbar-color: var(--input);
		border-width: 0 0 0 1px;
		border-style: solid;
		border-color: var(--border);
		background-color: var(--card);
	}
</style>
