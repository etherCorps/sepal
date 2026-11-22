<script lang="ts">
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select/index.js';
	let value = $state('apple');
</script>

<div class="w-60 p-6">
	<Select type="single" bind:value>
		<SelectTrigger>{value}</SelectTrigger>
		<SelectContent>
			<SelectItem value="apple">Apple</SelectItem>
			<SelectItem value="banana">Banana</SelectItem>
		</SelectContent>
	</Select>
</div>
