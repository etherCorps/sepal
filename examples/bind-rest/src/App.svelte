<script lang="ts">
	import Field from './Field.svelte';

	let text = $state('start');
</script>

<Field bind:value={text} title="a field" />
<p id="out">{text}</p>
