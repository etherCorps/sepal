<script lang="ts">
	let { params }: { params: { path: string } } = $props();
</script>

<h1 id="title" class="text-2xl font-semibold">Docs</h1>
<p id="rest">{params.path}</p>
