<!-- A project's 10px colour square. -->
<script lang="ts">
	import { project_by_key } from '../seed.ts';

	let { project }: { project: string } = $props();
</script>

<div class="square" style="background-color: var(--hue-{project_by_key[project]?.hue ?? 1})"></div>

<style>
	.square { width: 10px; height: 10px; flex-shrink: 0; border-radius: 3px; }
</style>
