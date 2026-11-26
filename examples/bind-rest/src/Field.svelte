<script lang="ts">
	import Inner from './Inner.svelte';

	// The bound prop, and the rest of the props spread on with it, as
	// wrappers in component libraries do. Inner's props are then a Proxy,
	// which Svelte asks for the setter of `value`.
	let { value = $bindable(), ...rest }: { value: string; [key: string]: unknown } = $props();
</script>

<Inner bind:value {...rest} />
