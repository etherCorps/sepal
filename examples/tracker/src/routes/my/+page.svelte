<script lang="ts">
	import IssueList from '$lib/list/IssueList.svelte';
	import { ME } from '$lib/seed';
</script>

<IssueList title="My issues" scope={{ assignee: ME }} />
