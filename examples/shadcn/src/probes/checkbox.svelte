<script lang="ts">
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	let checked = $state(false);
</script>

<div class="flex items-center gap-3 p-6">
	<Checkbox id="check" bind:checked />
	<span id="state">{checked ? 'checked' : 'unchecked'}</span>
</div>
