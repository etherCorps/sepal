<script>
	let { ids, has_fallback } = $props();
</script>

<ul id="routes">
	{#each ids as id (id)}
		<li>{id}</li>
	{/each}
</ul>
<p id="fallback">{has_fallback}</p>
