<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
</script>

<div class="flex items-center gap-4 p-6">
	<Skeleton class="size-12 rounded-full" />
	<div class="flex flex-col gap-2"><Skeleton class="h-4 w-60" /><Skeleton class="h-4 w-48" /></div>
</div>
