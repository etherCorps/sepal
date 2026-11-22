<script lang="ts">
	import { goto } from '$app/navigation';
</script>

<h1 class="text-3xl font-semibold">About</h1>
<p class="text-slate-600">
	A page is a <code class="rounded bg-slate-200 px-1.5 py-0.5 text-sm">+page.svelte</code> in
	<code class="rounded bg-slate-200 px-1.5 py-0.5 text-sm">src/routes</code>.
</p>
<button class="rounded-lg border border-slate-300 px-5 py-2.5 hover:bg-slate-100" onclick={() => goto('/')}>
	back home
</button>
