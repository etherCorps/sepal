<script lang="ts">
	let { data }: { data: { waited: number } } = $props();
</script>

<h1 id="title" class="text-2xl font-semibold">Slow</h1>
<p id="waited">waited {data.waited} ms</p>
