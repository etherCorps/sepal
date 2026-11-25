<script>
	let { ids } = $props();
</script>

<ul id="routes">
	{#each ids as id (id)}
		<li>{id}</li>
	{/each}
</ul>
