<script lang="ts">
	import { Switch } from '$lib/components/ui/switch/index.js';
	let on = $state(false);
</script>

<div class="flex items-center gap-3 p-6">
	<Switch id="toggle" bind:checked={on} />
	<span id="state">{on ? 'on' : 'off'}</span>
</div>
