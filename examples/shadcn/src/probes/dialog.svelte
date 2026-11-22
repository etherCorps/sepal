<script lang="ts">
	import { buttonVariants } from '$lib/components/ui/button/index.js';
	import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from '$lib/components/ui/dialog/index.js';
</script>

<div class="p-6">
	<Dialog>
		<DialogTrigger class={buttonVariants({ variant: 'outline' })}>Open dialog</DialogTrigger>
		<DialogContent>
			<DialogHeader>
				<DialogTitle>Are you sure?</DialogTitle>
				<DialogDescription>This action cannot be undone.</DialogDescription>
			</DialogHeader>
		</DialogContent>
	</Dialog>
</div>
