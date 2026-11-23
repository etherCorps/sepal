<script lang="ts">
	import IssueList from '../list/IssueList.svelte';
	import { ME } from '../seed.ts';
</script>

<IssueList title="My issues" scope={{ assignee: ME }} />
