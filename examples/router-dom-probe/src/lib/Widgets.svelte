<!-- What each route puts on the page: a Select, a Dialog and a Tooltip, all portalled by Bits UI. -->
<script lang="ts">
	import { buttonVariants } from '#lib/components/ui/button/index.ts';
	import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '#lib/components/ui/dialog/index.ts';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '#lib/components/ui/select/index.ts';
	import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '#lib/components/ui/tooltip/index.ts';

	let { name }: { name: string } = $props();
	let fruit = $state('apple');
	let clicks = $state(0);
</script>

<h1 id="title">Page {name}</h1>
<div class="flex items-center gap-3">
	<Select type="single" bind:value={fruit}>
		<SelectTrigger id="fruit" class="w-40">{fruit}</SelectTrigger>
		<SelectContent>
			<SelectItem value="apple">Apple {name}</SelectItem>
			<SelectItem value="banana">Banana {name}</SelectItem>
		</SelectContent>
	</Select>
	<Dialog>
		<DialogTrigger id="open-dialog" class={buttonVariants({ variant: 'outline' })}>Open dialog</DialogTrigger>
		<DialogContent>
			<DialogHeader>
				<DialogTitle>Dialog {name}</DialogTitle>
			</DialogHeader>
			<button id="in-dialog" onclick={() => clicks++}>clicked {clicks}</button>
			<a id="dialog-link" href={name === 'A' ? '/b' : '/'}>switch</a>
		</DialogContent>
	</Dialog>
	<TooltipProvider>
		<Tooltip>
			<TooltipTrigger id="tip" class={buttonVariants({ variant: 'outline' })}>Hover me</TooltipTrigger>
			<TooltipContent><p>Tip {name}</p></TooltipContent>
		</Tooltip>
	</TooltipProvider>
</div>
