<script lang="ts">
	import { untrack } from 'svelte';
	import type { Item } from '$lib/items';

	let { data, params }: { data: { item: Item; from: string | null; heading: string }; params: { id: string } } = $props();

	// The same component instance serves every item, so this survives going
	// from one item to another.
	let looked_at = $state(0);
	$effect(() => {
		params.id;
		// Reading the counter here would make the effect depend on what it writes.
		untrack(() => looked_at++);
	});
</script>

<h1 id="title" class="text-2xl font-semibold">{data.item.name}</h1>
<p id="heading">{data.heading}</p>
<p id="param">id {params.id}</p>
<p id="from">{data.from ?? 'no query'}</p>
<p id="looked-at">items seen: {looked_at}</p>
<a id="next" class="text-indigo-600" href="/items/{Number(params.id) + 1}">next item</a>
