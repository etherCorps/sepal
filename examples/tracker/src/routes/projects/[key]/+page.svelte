<script lang="ts">
	import IssueList from '$lib/list/IssueList.svelte';
	import type { Project } from '$lib/seed';

	let { data }: { data: { project: Project } } = $props();
</script>

<IssueList title={data.project.name} scope={{ project: data.project.key }} />
