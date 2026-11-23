<!-- /projects/:key; an unknown key is the Tracker's 404 (No project X). -->
<script lang="ts">
	import IssueList from '../list/IssueList.svelte';
	import { project_by_key } from '../seed.ts';
	import NotFound from './NotFound.svelte';

	let { params }: { params: Record<string, string> } = $props();
	const project = $derived(project_by_key[params.key]);
</script>

{#if project}
	<IssueList title={project.name} scope={{ project: project.key }} />
{:else}
	<NotFound message="No project {params.key}" />
{/if}
