<script lang="ts">
	import { page } from '$app/state';
	import PageHeader from '$lib/components/PageHeader.svelte';
</script>

<PageHeader title="Not found" />
<div class="p-8">
	<p data-testid="error-message" class="text-foreground text-sm font-medium">{page.error?.message ?? `Error ${page.status}`}</p>
	<a class="text-primary mt-2 inline-block text-[13px]" href="/issues">Back to issues</a>
</div>
