<script lang="ts">
	let { value = $bindable() }: { value: string } = $props();
</script>

<button id="change" onclick={() => (value = 'changed')}>{value}</button>
