<!-- In the pane: the list stays. -->
<script lang="ts">
	import { page } from '$app/state';
</script>

<div class="p-5">
	<p data-testid="error-message" class="text-foreground text-base font-medium">{page.error?.message ?? 'Not found'}.</p>
	<p class="text-muted-foreground mt-1 text-[13px]">
		It may have been deleted. <a class="text-primary" href="/issues">Back to issues</a>
	</p>
</div>
