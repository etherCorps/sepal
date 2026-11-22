<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	let n = $state(0);
</script>

<div class="flex gap-2 p-6">
	<Button id="count" onclick={() => n++}>Clicked {n}</Button>
	<Button variant="outline">Outline</Button>
	<Button variant="secondary">Secondary</Button>
	<Button variant="destructive">Destructive</Button>
	<Button variant="ghost">Ghost</Button>
	<Button variant="link">Link</Button>
</div>
