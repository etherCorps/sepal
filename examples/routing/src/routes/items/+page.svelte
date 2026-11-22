<script lang="ts">
	import type { Item } from '$lib/items';

	let { data }: { data: { app: string; items: Item[] } } = $props();
</script>

<h1 id="title" class="text-2xl font-semibold">Items</h1>
<ul id="items">
	{#each data.items as item (item.id)}
		<li><a class="text-indigo-600" href="/items/{item.id}">{item.name}</a></li>
	{/each}
	<li><a id="item-unknown" class="text-indigo-600" href="/items/99">an item that does not exist</a></li>
</ul>
