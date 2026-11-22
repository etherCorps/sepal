<script lang="ts">
	let { data } = $props();
</script>

<svelte:head>
	<title>Item {data.id}</title>
</svelte:head>

<h1 id="title">Item {data.id}</h1>
<p id="double">twice that is {data.double}</p>
