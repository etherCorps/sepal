<script lang="ts">
	import IssueList from '#lib/list/IssueList.svelte';
	import { ME } from '#lib/seed.ts';
</script>

<IssueList title="My issues" scope={{ assignee: ME }} />
