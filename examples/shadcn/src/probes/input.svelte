<script lang="ts">
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	let email = $state('me@example.com');
</script>

<div class="flex w-80 flex-col gap-2 p-6">
	<Label for="email">Email</Label>
	<Input id="email" type="email" placeholder="Email" bind:value={email} />
	<p id="echo">{email}</p>
</div>
