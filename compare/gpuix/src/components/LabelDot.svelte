<!-- A label's 6px colour dot. -->
<script lang="ts">
	import { label_by_id } from '../seed.ts';

	let { label }: { label: string } = $props();
</script>

<div class="dot" style="background-color: var(--hue-{label_by_id[label]?.hue ?? 1})"></div>

<style>
	.dot { width: 6px; height: 6px; flex-shrink: 0; border-radius: 999px; }
</style>
