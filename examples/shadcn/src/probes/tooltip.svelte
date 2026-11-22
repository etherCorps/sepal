<script lang="ts">
	import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '$lib/components/ui/tooltip/index.js';
</script>

<div class="p-6">
	<TooltipProvider>
		<Tooltip>
			<TooltipTrigger>Hover me</TooltipTrigger>
			<TooltipContent><p>Add to library</p></TooltipContent>
		</Tooltip>
	</TooltipProvider>
</div>
