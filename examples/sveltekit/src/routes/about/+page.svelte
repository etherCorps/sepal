<script lang="ts">
	import { goto } from '$app/navigation';
</script>

<svelte:head>
	<title>About this app</title>
</svelte:head>

<h1 id="title">About</h1>
<button id="go" onclick={() => goto('/items/7')}>go to item 7</button>
