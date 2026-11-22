<script lang="ts">
	let count = $state(0);
</script>

<main
	class="flex h-screen flex-col items-center justify-center gap-6 bg-slate-50 font-sans text-slate-900"
>
	<h1 class="text-3xl font-semibold">Svelte on Blitz</h1>
	<p class="text-slate-600">
		Edit <code class="rounded bg-slate-200 px-1.5 py-0.5 text-sm">src/App.svelte</code> and save.
	</p>
	<button
		class="rounded-lg bg-indigo-600 px-5 py-2.5 font-medium text-white hover:bg-indigo-500"
		onclick={() => count++}
	>
		clicked {count}
		{count === 1 ? 'time' : 'times'}
	</button>
</main>
