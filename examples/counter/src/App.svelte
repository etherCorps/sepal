<script lang="ts">
	import Row from './Row.svelte';

	let count = $state(0);
	let doubled = $derived(count * 2);

	let rows = $state(
		Array.from({ length: 1000 }, (_, i) => ({ id: i + 1, label: `Row ${i + 1}`, hits: 0 }))
	);
	let selected = $state(0);

	let scrolled = $state(0);

	let seconds = $state(0);
	let ticking = $state(false);

	$effect(() => {
		if (!ticking) return;
		const id = setInterval(() => seconds++, 1000);
		return () => clearInterval(id);
	});

	function shuffle() {
		for (let i = rows.length - 1; i > 0; i--) {
			const j = Math.floor(Math.random() * (i + 1));
			[rows[i], rows[j]] = [rows[j], rows[i]];
		}
	}
</script>

<main class="flex h-screen flex-col gap-4 bg-slate-50 p-6 font-sans text-slate-900">
	<h1 class="text-2xl font-semibold">Svelte on Blitz</h1>

	<section class="flex items-center gap-3">
		<button
			id="inc"
			class="rounded-lg bg-indigo-600 px-4 py-2 font-medium text-white hover:bg-indigo-500"
			onclick={() => count++}
		>
			count is {count}
		</button>
		<span id="doubled" class="text-slate-600">doubled: {doubled}</span>
		{#if count >= 3}
			<span id="badge" class="rounded-full bg-amber-100 px-3 py-1 text-sm text-amber-800">
				three or more
			</span>
		{/if}
	</section>

	<section class="flex items-center gap-3">
		<button
			id="tick"
			class="rounded-lg border border-slate-300 px-4 py-2 hover:bg-slate-100"
			onclick={() => (ticking = !ticking)}
		>
			{ticking ? 'stop' : 'start'} timer
		</button>
		<span id="seconds" class="tabular-nums text-slate-600">{seconds}s</span>
		<button
			id="shuffle"
			class="rounded-lg border border-slate-300 px-4 py-2 hover:bg-slate-100"
			onclick={shuffle}
		>
			shuffle rows
		</button>
		<span id="scrolled" class="tabular-nums text-slate-600">scrolled {scrolled}px</span>
	</section>

	<ul
		id="rows"
		class="min-h-0 flex-1 overflow-y-scroll rounded-lg border border-slate-200 bg-white"
		onscroll={(event) => (scrolled = event.currentTarget.scrollTop)}
	>
		{#each rows as row (row.id)}
			<Row {row} selected={selected === row.id} onselect={() => { selected = row.id; row.hits++; }} />
		{/each}
	</ul>
</main>
